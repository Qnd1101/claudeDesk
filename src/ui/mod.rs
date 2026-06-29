mod facet_view;
mod help;
mod layout;
mod list;
mod modal;
mod preview;
mod settings;
mod theme;
mod time;
mod trash_view;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::HashSet;
use std::io::stdout;
use std::time::Duration;

use crate::config::{expand_tilde, Config, ResumeMode};
use crate::preview::{read_preview, PreviewContent, MAX_PREVIEW_BYTES, MAX_PREVIEW_LINES};
use crate::service::{
    exec_resume, exec_resume_spawn, resume_session, AppState, DisplayRow, ResumeResult,
};

/// 미리보기 캐시 키 — 타입으로 세션/헤더를 구분한다.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PreviewCacheKey {
    /// 특정 세션의 미리보기 (session_id)
    Session(String),
    /// 그룹 헤더 또는 빈 목록 커서
    Header,
}
use crate::trash::{purge_sessions, restore_sessions, soft_delete_sessions, TrashIndex};
use help::render_help;
use list::{render_list, PREVIEW_MIN_WIDTH};
use modal::{
    render_age_select, render_alias_edit, render_delete_confirm, render_purge_confirm,
    AgeSelectData, AliasEditData, DeleteConfirmData, PurgeConfirmData,
};
use settings::{render_settings, SettingsData, SETTINGS_ROW_COUNT};
use trash_view::render_trash;

/// FR-14: 오래된 세션 선택 모달의 기준 일수 프리셋
const AGE_PRESET_DAYS: [u64; 5] = [7, 30, 90, 180, 365];

/// 하루 초 (FR-14 cutoff 계산)
const SECS_PER_DAY: u64 = 86_400;

/// UI 모드
#[derive(Debug, Clone, PartialEq, Eq)]
enum UiMode {
    /// 일반 목록 모드
    Normal,
    /// 검색 모드 (`/` 진입)
    Search,
    /// 소프트 삭제 확인 모달 (Del/d)
    DeleteConfirm,
    /// 휴지통 화면 (T)
    Trash,
    /// purge 2단계 확인 모달 (D in Trash)
    PurgeConfirm,
    /// 별칭 지정/편집 모달 (n, FR-06)
    AliasEdit,
    /// 오래된 세션 선택 모달 (o, FR-14)
    AgeSelect,
    /// 설정 화면 (`,`, FR-10 T11.2)
    Settings,
}

pub struct App {
    state: AppState,
    /// 런타임 설정 (T11.2 저장 대상, T11.3 색상 제어)
    config: Config,
    /// 현재 UI 모드
    mode: UiMode,
    /// 필터된 인덱스 목록 내에서의 커서 위치
    cursor: usize,
    show_help: bool,
    /// resume 요청 대기 (TUI 종료 후 실행)
    pending_resume: Option<(String, String)>,
    /// claude 미발견 안내 메시지
    not_found_msg: Option<(String, String)>,

    // ── 삭제 확인 모달 상태 (FR-04) ──────────────────────────────────────
    /// 삭제 확인 모달에 표시할 제목 목록 (확인 시점 스냅샷)
    delete_titles: Vec<String>,
    /// 활성 세션이라 스킵될 수 (모달 표시용)
    delete_active_count: usize,
    /// 삭제 대상 session_id 목록 (확인 시점 스냅샷)
    delete_pending_ids: Vec<String>,

    // ── 휴지통 상태 (FR-11) ───────────────────────────────────────────────
    /// 휴지통 항목 캐시 (화면 열 때 로드)
    trash_index: TrashIndex,
    /// 휴지통 화면 커서
    trash_cursor: usize,
    /// 휴지통 다중선택 session_id 집합
    trash_selected: HashSet<String>,

    // ── purge 확인 모달 상태 (FR-11) ─────────────────────────────────────
    /// purge 확인 모달에 표시할 제목 목록
    purge_titles: Vec<String>,
    /// purge 대상 session_id 목록
    purge_pending_ids: Vec<String>,
    /// "DELETE" 타이핑 버퍼
    purge_input: String,

    // ── FR-06: 별칭 편집 모달 상태 ───────────────────────────────────────
    /// 별칭 편집 모달 입력 버퍼
    alias_input: String,
    /// 편집 대상 session_id 스냅샷 (모달 열릴 때 캡처)
    alias_target_id: Option<String>,
    /// 편집 대상 세션의 표시 제목 스냅샷 (원본 제목 표시용)
    alias_target_title: String,

    // ── 상태 메시지 ───────────────────────────────────────────────────────
    /// 임시 상태 메시지 (작업 결과 표시용)
    status_message: Option<String>,

    // ── FR-08: 미리보기 ───────────────────────────────────────────────────
    /// 미리보기 패널 열림 여부 (Normal 모드 전용 토글)
    preview_open: bool,
    /// 미리보기 캐시: (키, PreviewContent) — 같은 키이면 재읽기 금지
    preview_cache: Option<(PreviewCacheKey, PreviewContent)>,

    // ── FR-14: 오래된 세션 선택 모달 상태 ────────────────────────────────
    /// 모달 내 커서(AGE_PRESET_DAYS 인덱스)
    age_cursor: usize,
    /// 프리셋별 대상 세션 수(모달 열 때 계산, AGE_PRESET_DAYS와 동일 순서)
    age_counts: Vec<usize>,

    // ── FR-10 T11.2: 설정 화면 상태 ──────────────────────────────────────
    /// 설정 화면 편집용 임시 복사본. `s` 저장 시 config에 반영.
    settings_draft: Config,
    /// 설정 화면 내 커서 위치 (0=Projects root … 5=Theme)
    settings_cursor: usize,
    /// Projects root 인라인 경로 편집 서브모드 여부
    settings_path_editing: bool,
    /// 경로 편집 버퍼
    settings_path_input: String,

    // ── FR-12/T5: FS 변경 자동 감지 (notify watcher) ──────────────────────
    /// FS watcher 소유자 — drop 방지용. 실제로 poll은 watch_rx로.
    _watcher: Option<notify::RecommendedWatcher>,
    /// watcher 이벤트 수신 채널
    watch_rx: Option<std::sync::mpsc::Receiver<()>>,
    /// 마지막 이벤트 수신 시각 (300ms 디바운스)
    watch_debounce: Option<std::time::Instant>,
}

impl App {
    pub fn new(state: AppState, config: Config) -> Self {
        let settings_draft = config.clone();

        // watcher 시작 (실패해도 TUI는 계속 동작)
        let (_watcher, watch_rx) = match crate::service::start_watcher(&state.projects_root) {
            Ok((w, rx)) => (Some(w), Some(rx)),
            Err(_) => (None, None),
        };

        App {
            state,
            config,
            mode: UiMode::Normal,
            cursor: 0,
            show_help: false,
            pending_resume: None,
            not_found_msg: None,

            delete_titles: vec![],
            delete_active_count: 0,
            delete_pending_ids: vec![],

            trash_index: TrashIndex::default(),
            trash_cursor: 0,
            trash_selected: HashSet::new(),

            purge_titles: vec![],
            purge_pending_ids: vec![],
            purge_input: String::new(),

            alias_input: String::new(),
            alias_target_id: None,
            alias_target_title: String::new(),

            status_message: None,

            preview_open: false,
            preview_cache: None,

            age_cursor: 0,
            age_counts: vec![],

            settings_draft,
            settings_cursor: 0,
            settings_path_editing: false,
            settings_path_input: String::new(),

            _watcher,
            watch_rx,
            watch_debounce: None,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        // 터미널 초기화
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;

        // panic hook: 터미널 복원
        let old_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = stdout().execute(LeaveAlternateScreen);
            old_hook(info);
        }));

        let backend = CrosstermBackend::new(stdout());
        let mut terminal = Terminal::new(backend)?;

        let result = self.event_loop(&mut terminal);

        // 터미널 복원
        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;

        result?;

        // resume 핸드오프: TUI 종료 후 실행
        if let Some((cwd, session_id)) = self.pending_resume.take() {
            exec_resume(&cwd, &session_id)?;
        }

        // claude 미발견 안내
        if let Some((cwd, session_id)) = self.not_found_msg.take() {
            eprintln!();
            eprintln!("claude 를 PATH에서 찾을 수 없습니다.");
            eprintln!("아래 명령을 직접 실행하세요:");
            eprintln!();
            if !cwd.is_empty() {
                eprintln!("  cd \"{}\"", cwd);
            }
            eprintln!("  claude --resume {}", session_id);
            eprintln!();
        }

        Ok(())
    }

    fn event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<()> {
        loop {
            terminal.draw(|f| {
                let color_enabled = self.config.color_enabled();
                let time_format = self.config.time_format;
                if self.show_help {
                    render_help(f);
                } else {
                    match &self.mode {
                        UiMode::Trash => {
                            let entries = self.trash_index.sorted_entries();
                            render_trash(f, &entries, self.trash_cursor, &self.trash_selected);
                        }
                        UiMode::DeleteConfirm => {
                            let preview_content = self.current_preview_content();
                            let preview_title = self.current_session_title();
                            let preview_path = self.current_session_cwd();
                            render_list(
                                f,
                                &self.state,
                                self.cursor,
                                false,
                                &self.state.selected_ids.clone(),
                                self.status_message.as_deref(),
                                self.preview_open,
                                preview_content,
                                &preview_title,
                                &preview_path,
                                color_enabled,
                                time_format,
                            );
                            let data = DeleteConfirmData {
                                titles: &self.delete_titles,
                                active_count: self.delete_active_count,
                                color_enabled,
                            };
                            render_delete_confirm(f, &data);
                        }
                        UiMode::PurgeConfirm => {
                            let entries = self.trash_index.sorted_entries();
                            render_trash(f, &entries, self.trash_cursor, &self.trash_selected);
                            let data = PurgeConfirmData {
                                titles: &self.purge_titles,
                                input: &self.purge_input,
                                color_enabled,
                            };
                            render_purge_confirm(f, &data);
                        }
                        UiMode::AliasEdit => {
                            let preview_content = self.current_preview_content();
                            let preview_title = self.current_session_title();
                            let preview_path = self.current_session_cwd();
                            render_list(
                                f,
                                &self.state,
                                self.cursor,
                                false,
                                &self.state.selected_ids.clone(),
                                self.status_message.as_deref(),
                                self.preview_open,
                                preview_content,
                                &preview_title,
                                &preview_path,
                                color_enabled,
                                time_format,
                            );
                            let data = AliasEditData {
                                original_title: &self.alias_target_title,
                                input: &self.alias_input,
                                color_enabled,
                            };
                            render_alias_edit(f, &data);
                        }
                        UiMode::AgeSelect => {
                            let preview_content = self.current_preview_content();
                            let preview_title = self.current_session_title();
                            let preview_path = self.current_session_cwd();
                            render_list(
                                f,
                                &self.state,
                                self.cursor,
                                false,
                                &self.state.selected_ids.clone(),
                                self.status_message.as_deref(),
                                self.preview_open,
                                preview_content,
                                &preview_title,
                                &preview_path,
                                color_enabled,
                                time_format,
                            );
                            // (기준 일수, 대상 수) 쌍으로 모달에 전달
                            let options: Vec<(u64, usize)> = AGE_PRESET_DAYS
                                .iter()
                                .copied()
                                .zip(self.age_counts.iter().copied())
                                .collect();
                            let data = AgeSelectData {
                                options: &options,
                                cursor: self.age_cursor,
                                color_enabled,
                            };
                            render_age_select(f, &data);
                        }
                        UiMode::Settings => {
                            // 배경: 현재 목록을 뒤에 그리고 모달 오버레이
                            let preview_content = self.current_preview_content();
                            let preview_title = self.current_session_title();
                            let preview_path = self.current_session_cwd();
                            render_list(
                                f,
                                &self.state,
                                self.cursor,
                                false,
                                &self.state.selected_ids.clone(),
                                self.status_message.as_deref(),
                                self.preview_open,
                                preview_content,
                                &preview_title,
                                &preview_path,
                                color_enabled,
                                time_format,
                            );
                            let data = SettingsData {
                                draft: &self.settings_draft,
                                cursor: self.settings_cursor,
                                path_editing: self.settings_path_editing,
                                path_input: &self.settings_path_input,
                                color_enabled,
                            };
                            render_settings(f, &data);
                        }
                        _ => {
                            let search_mode = self.mode == UiMode::Search;
                            let preview_content = self.current_preview_content();
                            let preview_title = self.current_session_title();
                            let preview_path = self.current_session_cwd();
                            facet_view::render(
                                f,
                                f.area(),
                                &self.state,
                                self.cursor,
                                search_mode,
                                self.preview_open,
                                preview_content,
                                &preview_title,
                                &preview_path,
                                color_enabled,
                                time_format,
                            );
                        }
                    }
                }
            })?;

            if event::poll(Duration::from_millis(200))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press
                        && self.handle_key(key.code, key.modifiers)?
                    {
                        return Ok(());
                    }
                }
            }

            // FS 변경 감지 poll (notify watcher)
            if let Some(ref rx) = self.watch_rx {
                if rx.try_recv().is_ok() {
                    // 채널 drain (누적 이벤트 정리)
                    while rx.try_recv().is_ok() {}
                    self.watch_debounce = Some(std::time::Instant::now());
                }
            }
            // 300ms 디바운스 후 reload
            if let Some(t) = self.watch_debounce {
                if t.elapsed() >= std::time::Duration::from_millis(300) {
                    self.watch_debounce = None;
                    self.reload_sessions()?;
                }
            }
        }
    }

    /// 키 이벤트 처리. true 반환 시 루프 종료
    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Result<bool> {
        // 도움말 오버레이가 열려 있으면 Esc/?로 닫기
        if self.show_help {
            match code {
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                    self.show_help = false;
                }
                _ => {}
            }
            return Ok(false);
        }

        match self.mode.clone() {
            UiMode::DeleteConfirm => return self.handle_delete_confirm_key(code),
            UiMode::Trash => return self.handle_trash_key(code),
            UiMode::PurgeConfirm => return self.handle_purge_confirm_key(code),
            UiMode::Search => return self.handle_search_key(code),
            UiMode::AliasEdit => return self.handle_alias_edit_key(code),
            UiMode::AgeSelect => return self.handle_age_select_key(code),
            UiMode::Settings => return self.handle_settings_key(code),
            UiMode::Normal => {}
        }

        // ── 일반 모드 ─────────────────────────────────────────────────────
        self.handle_normal_key(code, modifiers)
    }

    // ── 일반 모드 키 ──────────────────────────────────────────────────────

    fn handle_normal_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Result<bool> {
        match code {
            // 종료
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(true);
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(true);
            }

            // 검색 진입 (FR-05)
            KeyCode::Char('/') => {
                self.mode = UiMode::Search;
                self.state.search_query = Some(String::new());
                self.cursor = 0;
            }

            // 정렬 키 순환 (FR-07, `s`)
            KeyCode::Char('s') => {
                self.state.sort = self.state.sort.cycle_key();
                self.apply_sort_and_reset_cursor();
            }

            // 정렬 방향 토글 (FR-07, `S`)
            KeyCode::Char('S') => {
                self.state.sort = self.state.sort.toggle_dir();
                self.apply_sort_and_reset_cursor();
            }

            // 이동
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let count = crate::facet::facet_indices(&self.state).len();
                if count > 0 && self.cursor < count - 1 {
                    self.cursor += 1;
                }
            }
            KeyCode::Home => {
                self.cursor = 0;
            }
            KeyCode::End => {
                let count = crate::facet::facet_indices(&self.state).len();
                if count > 0 {
                    self.cursor = count - 1;
                }
            }
            KeyCode::PageUp => {
                self.cursor = self.cursor.saturating_sub(10);
            }
            KeyCode::PageDown => {
                let max = crate::facet::facet_indices(&self.state)
                    .len()
                    .saturating_sub(1);
                self.cursor = (self.cursor + 10).min(max);
            }

            // 도움말
            KeyCode::Char('?') => {
                self.show_help = true;
            }

            // ── FR-08: 미리보기 토글 (p, Normal 모드 전용) ──────────────────
            KeyCode::Char('p') => {
                let term_width = crossterm::terminal::size().map(|(w, _)| w).unwrap_or(0);
                self.toggle_preview(term_width);
            }

            // ── M2: 다중선택 (Space, FR-04) ─────────────────────────────
            KeyCode::Char(' ') => {
                if self.state.grouped {
                    let rows = self.state.display_rows();
                    if let Some(row) = rows.get(self.cursor) {
                        match row.clone() {
                            DisplayRow::Header { cwd, .. } => {
                                self.toggle_group_selection(&cwd);
                                return Ok(false);
                            }
                            DisplayRow::Session(_) => {}
                        }
                    }
                }
                if let Some(session) = self.current_session() {
                    let sid = session.session_id.clone();
                    if self.state.selected_ids.contains(&sid) {
                        self.state.selected_ids.remove(&sid);
                    } else {
                        self.state.selected_ids.insert(sid);
                    }
                }
            }

            // ── M2: 전체선택/해제 토글 (a, §5-2) ───────────────────────
            KeyCode::Char('a') => {
                let visible_ids: Vec<String> = crate::facet::facet_indices(&self.state)
                    .iter()
                    .filter_map(|&i| self.state.sessions.get(i))
                    .map(|s| s.session_id.clone())
                    .collect();

                let all_selected = visible_ids
                    .iter()
                    .all(|sid| self.state.selected_ids.contains(sid));

                if all_selected {
                    // 현재 표시 항목 전부 선택 해제
                    for sid in &visible_ids {
                        self.state.selected_ids.remove(sid);
                    }
                } else {
                    // 현재 표시 항목 전부 선택
                    for sid in visible_ids {
                        self.state.selected_ids.insert(sid);
                    }
                }
            }

            // ── M2: 삭제 (Del / d, FR-04) ───────────────────────────────
            KeyCode::Delete | KeyCode::Char('d') => {
                self.open_delete_confirm();
            }

            // ── M2: 휴지통 열기 (T, FR-11) ──────────────────────────────
            KeyCode::Char('T') => {
                self.open_trash();
            }

            // ── FR-14: 오래된 세션 선택 모달 열기 (o) ───────────────────
            KeyCode::Char('o') => {
                self.open_age_select();
            }

            // ── FR-15: facet 탭 전환 (Tab / Shift+Tab, 1-4) ──────────────
            KeyCode::Tab => {
                let current_id = self.current_session().map(|s| s.session_id.clone());
                self.state.facet = self.state.facet.next();
                if let Some(id) = current_id {
                    self.state.cursor_identity = Some(id);
                    self.restore_cursor_in_facet();
                } else {
                    self.cursor = 0;
                }
            }

            KeyCode::BackTab => {
                let current_id = self.current_session().map(|s| s.session_id.clone());
                self.state.facet = self.state.facet.prev();
                if let Some(id) = current_id {
                    self.state.cursor_identity = Some(id);
                    self.restore_cursor_in_facet();
                } else {
                    self.cursor = 0;
                }
            }

            KeyCode::Char('1') | KeyCode::Char('2') | KeyCode::Char('3') | KeyCode::Char('4') => {
                if let KeyCode::Char(c) = code {
                    if let Some(d) = c.to_digit(10) {
                        if let Some(new_facet) = crate::facet::Facet::from_digit(d) {
                            let current_id = self.current_session().map(|s| s.session_id.clone());
                            self.state.facet = new_facet;
                            if let Some(id) = current_id {
                                self.state.cursor_identity = Some(id);
                                self.restore_cursor_in_facet();
                            } else {
                                self.cursor = 0;
                            }
                        }
                    }
                }
            }

            // ── FR-10 T11.2: 설정 화면 열기 (`,`) ───────────────────────────
            KeyCode::Char(',') => {
                self.open_settings();
            }

            // ── FR-06: 별칭 지정/편집 (n) ────────────────────────────────────
            KeyCode::Char('n') => {
                if let Some(session) = self.current_session() {
                    let sid = session.session_id.clone();
                    // 모달엔 도출 원본 제목을 표시(별칭은 입력 prefill로 따로 채워 편집 시 원본 참조 가능)
                    let title = session.title.clone();
                    let prefill = session.alias.clone().unwrap_or_default();
                    self.alias_target_id = Some(sid);
                    self.alias_target_title = title;
                    self.alias_input = prefill;
                    self.mode = UiMode::AliasEdit;
                }
                // current_session() == None (그룹 헤더) 이면 무시
            }

            // Resume
            KeyCode::Enter => {
                if self.state.grouped {
                    let rows = self.state.display_rows();
                    if let Some(row) = rows.get(self.cursor) {
                        match row {
                            DisplayRow::Header { cwd, .. } => {
                                let cwd = cwd.clone();
                                if self.state.collapsed_projects.contains(&cwd) {
                                    self.state.collapsed_projects.remove(&cwd);
                                } else {
                                    self.state.collapsed_projects.insert(cwd);
                                }
                                self.clamp_cursor();
                                return Ok(false);
                            }
                            DisplayRow::Session(_) => {}
                        }
                    }
                }
                if let Some(session) = self.current_session() {
                    match resume_session(session) {
                        ResumeResult::Ready { cwd, session_id } => {
                            if self.config.resume_mode == ResumeMode::Spawn {
                                // Spawn 모드: 새 터미널 창 시도, claudedesk는 종료하지 않음
                                let msg = exec_resume_spawn(&cwd, &session_id);
                                self.status_message = Some(msg);
                                return Ok(false);
                            }
                            // Handoff 모드(기본): 기존 동작 그대로
                            self.pending_resume = Some((cwd, session_id));
                            return Ok(true);
                        }
                        ResumeResult::NotFound { cwd, session_id } => {
                            self.not_found_msg = Some((cwd, session_id));
                            return Ok(true);
                        }
                    }
                }
            }

            _ => {}
        }

        // 키 처리 후 미리보기 캐시 갱신 (커서·상태 변경이 반영된 시점에 호출)
        self.refresh_preview_cache();

        Ok(false)
    }

    // ── 검색 모드 키 처리 ─────────────────────────────────────────────────

    fn handle_search_key(&mut self, code: KeyCode) -> Result<bool> {
        match code {
            // 검색 해제 → 전체 목록 복귀
            KeyCode::Esc => {
                self.mode = UiMode::Normal;
                self.state.search_query = None;
                self.cursor = 0;
            }

            // 백스페이스: 쿼리 1자 삭제
            KeyCode::Backspace => {
                if let Some(ref mut q) = self.state.search_query {
                    q.pop();
                    self.cursor = 0;
                }
            }

            // Enter: 필터 결과 첫 항목 resume
            KeyCode::Enter => {
                if let Some(session) = self.current_session() {
                    match resume_session(session) {
                        ResumeResult::Ready { cwd, session_id } => {
                            if self.config.resume_mode == ResumeMode::Spawn {
                                let msg = exec_resume_spawn(&cwd, &session_id);
                                self.status_message = Some(msg);
                                return Ok(false);
                            }
                            self.pending_resume = Some((cwd, session_id));
                            return Ok(true);
                        }
                        ResumeResult::NotFound { cwd, session_id } => {
                            self.not_found_msg = Some((cwd, session_id));
                            return Ok(true);
                        }
                    }
                }
            }

            // ↑↓: 필터 결과 내 이동
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let count = crate::facet::facet_indices(&self.state).len();
                if count > 0 && self.cursor < count - 1 {
                    self.cursor += 1;
                }
            }

            // 타이핑: 쿼리에 추가
            KeyCode::Char(c) => {
                if let Some(ref mut q) = self.state.search_query {
                    q.push(c);
                    self.cursor = 0; // 쿼리 변경 시 커서 리셋
                }
            }

            _ => {}
        }
        // 검색 중에도 열려 있는 미리보기를 커서 이동에 맞게 갱신
        self.refresh_preview_cache();
        Ok(false)
    }

    // ── 삭제 확인 모달 키 처리 ────────────────────────────────────────────

    fn handle_delete_confirm_key(&mut self, code: KeyCode) -> Result<bool> {
        match code {
            KeyCode::Enter => {
                // 소프트 삭제 실행
                self.execute_soft_delete()?;
                self.mode = UiMode::Normal;
            }
            KeyCode::Esc => {
                // 취소
                self.mode = UiMode::Normal;
                self.delete_titles.clear();
                self.delete_pending_ids.clear();
                self.delete_active_count = 0;
            }
            _ => {}
        }
        Ok(false)
    }

    // ── 휴지통 화면 키 처리 (FR-11) ──────────────────────────────────────

    fn handle_trash_key(&mut self, code: KeyCode) -> Result<bool> {
        let entries_len = self.trash_index.sorted_entries().len();

        match code {
            // 닫기
            KeyCode::Esc | KeyCode::Char('T') => {
                self.mode = UiMode::Normal;
                self.trash_selected.clear();
                self.status_message = None;
            }

            // 이동
            KeyCode::Up | KeyCode::Char('k') => {
                if self.trash_cursor > 0 {
                    self.trash_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if entries_len > 0 && self.trash_cursor < entries_len - 1 {
                    self.trash_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.trash_cursor = 0;
            }
            KeyCode::End => {
                if entries_len > 0 {
                    self.trash_cursor = entries_len - 1;
                }
            }

            // 다중선택 토글 (Space)
            KeyCode::Char(' ') => {
                if let Some(entry) = self.current_trash_entry() {
                    let sid = entry.session_id.clone();
                    if self.trash_selected.contains(&sid) {
                        self.trash_selected.remove(&sid);
                    } else {
                        self.trash_selected.insert(sid);
                    }
                }
            }

            // 복구 (r)
            KeyCode::Char('r') => {
                self.execute_restore()?;
            }

            // 영구삭제 (D) → purge 확인 모달로
            KeyCode::Char('D') => {
                self.open_purge_confirm();
            }

            _ => {}
        }
        Ok(false)
    }

    // ── purge 확인 모달 키 처리 ───────────────────────────────────────────

    fn handle_purge_confirm_key(&mut self, code: KeyCode) -> Result<bool> {
        match code {
            KeyCode::Esc => {
                self.mode = UiMode::Trash;
                self.purge_input.clear();
                self.purge_titles.clear();
                self.purge_pending_ids.clear();
            }
            KeyCode::Enter => {
                if self.purge_input == "DELETE" {
                    self.execute_purge()?;
                    self.mode = UiMode::Trash;
                    self.purge_input.clear();
                }
                // DELETE 미입력 시 아무것도 안 함 (Enter 무시)
            }
            KeyCode::Backspace => {
                self.purge_input.pop();
            }
            KeyCode::Char(c) => {
                self.purge_input.push(c);
            }
            _ => {}
        }
        Ok(false)
    }

    // ── 별칭 편집 모달 키 처리 (FR-06) ──────────────────────────────────

    fn handle_alias_edit_key(&mut self, code: KeyCode) -> Result<bool> {
        match code {
            KeyCode::Esc => {
                self.cancel_alias_edit();
            }
            KeyCode::Enter => {
                if let Some(sid) = self.alias_target_id.clone() {
                    let input = self.alias_input.clone();
                    let msg = match self.state.set_alias(&sid, &input) {
                        Ok(()) => {
                            if input.trim().is_empty() {
                                "별칭을 삭제했습니다".to_string()
                            } else {
                                format!("별칭을 '{}'(으)로 설정했습니다", input.trim())
                            }
                        }
                        Err(e) => format!("별칭 저장 실패: {e}"),
                    };
                    self.status_message = Some(msg);
                    self.cancel_alias_edit();
                    self.refresh_preview_cache();
                }
            }
            KeyCode::Backspace => {
                self.alias_input.pop();
            }
            // 길이 가드: 80자 미만일 때만 입력 허용
            KeyCode::Char(c) if self.alias_input.chars().count() < 80 => {
                self.alias_input.push(c);
            }
            _ => {}
        }
        Ok(false)
    }

    // ── FR-14: 오래된 세션 선택 모달 키 처리 ─────────────────────────────

    fn handle_age_select_key(&mut self, code: KeyCode) -> Result<bool> {
        match code {
            KeyCode::Esc => {
                self.mode = UiMode::Normal;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.age_cursor > 0 {
                    self.age_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.age_cursor + 1 < AGE_PRESET_DAYS.len() {
                    self.age_cursor += 1;
                }
            }
            KeyCode::Enter => {
                let days = AGE_PRESET_DAYS[self.age_cursor];
                let cutoff =
                    std::time::SystemTime::now() - Duration::from_secs(days * SECS_PER_DAY);
                let n = self.state.select_older_than(cutoff);
                self.status_message = Some(if n == 0 {
                    format!("{}일 이전 세션이 없습니다", days)
                } else {
                    format!("{}일 이전 {}개 세션 선택됨 — d로 삭제 확인", days, n)
                });
                self.mode = UiMode::Normal;
                self.refresh_preview_cache();
            }
            _ => {}
        }
        Ok(false)
    }

    /// 별칭 편집 취소: 입력 버퍼·타깃 비우고 Normal 복귀
    fn cancel_alias_edit(&mut self) {
        self.alias_input.clear();
        self.alias_target_id = None;
        self.alias_target_title.clear();
        self.mode = UiMode::Normal;
    }

    // ── FR-10 T11.2: 설정 화면 키 처리 ──────────────────────────────────

    /// 설정 화면 열기 — draft를 현재 config 복사본으로 초기화
    fn open_settings(&mut self) {
        self.settings_draft = self.config.clone();
        self.settings_cursor = 0;
        self.settings_path_editing = false;
        self.settings_path_input = self
            .settings_draft
            .projects_root
            .to_string_lossy()
            .to_string();
        self.mode = UiMode::Settings;
    }

    /// 설정 화면 키 핸들러
    fn handle_settings_key(&mut self, code: KeyCode) -> Result<bool> {
        // ── 경로 편집 서브모드 ──────────────────────────────────────────
        if self.settings_path_editing {
            match code {
                KeyCode::Enter => {
                    // 입력 버퍼를 draft의 projects_root에 반영 후 서브모드 종료
                    self.settings_draft.projects_root = expand_tilde(&self.settings_path_input);
                    self.settings_path_editing = false;
                }
                KeyCode::Esc => {
                    // 취소 — 버퍼를 draft 현재 값으로 리셋
                    self.settings_path_input = self
                        .settings_draft
                        .projects_root
                        .to_string_lossy()
                        .to_string();
                    self.settings_path_editing = false;
                }
                KeyCode::Backspace => {
                    self.settings_path_input.pop();
                }
                KeyCode::Char(c) => {
                    self.settings_path_input.push(c);
                }
                _ => {}
            }
            return Ok(false);
        }

        // ── 일반 설정 키 ─────────────────────────────────────────────────
        match code {
            KeyCode::Esc => {
                // 저장 없이 닫기 (draft 폐기)
                self.mode = UiMode::Normal;
            }
            KeyCode::Char('s') => {
                // 저장: 파일에 먼저 쓰고, 성공했을 때만 런타임 config 반영.
                // (실패 시 self.config는 그대로 둬 파일과 런타임 상태가 어긋나지 않게)
                match self.settings_draft.save() {
                    Ok(()) => {
                        self.config = self.settings_draft.clone();
                        // default_sort 즉시 재정렬 반영 (다음 실행뿐 아니라 지금도 적용)
                        let new_sort = crate::service::SortState {
                            key: self.config.default_sort.key,
                            dir: self.config.default_sort.dir,
                        };
                        self.state.sort = new_sort;
                        crate::service::apply_sort(&mut self.state.sessions, new_sort);
                        self.cursor = 0;
                        self.status_message = Some("설정을 저장했습니다".to_string());
                    }
                    Err(e) => {
                        self.status_message = Some(format!("설정 저장 실패: {e}"));
                    }
                }
                self.mode = UiMode::Normal;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.settings_cursor > 0 {
                    self.settings_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.settings_cursor + 1 < SETTINGS_ROW_COUNT {
                    self.settings_cursor += 1;
                }
            }
            KeyCode::Left => {
                self.settings_apply_prev();
            }
            KeyCode::Right => {
                self.settings_apply_next();
            }
            KeyCode::Enter => {
                match self.settings_cursor {
                    0 => {
                        // Projects root: Enter → 인라인 경로 편집 진입
                        self.settings_path_input = self
                            .settings_draft
                            .projects_root
                            .to_string_lossy()
                            .to_string();
                        self.settings_path_editing = true;
                    }
                    1 => {
                        // Default sort: Enter → 방향 토글 (←→는 키 순환에 사용)
                        self.settings_draft.default_sort.dir =
                            self.settings_draft.default_sort.dir.toggle();
                    }
                    _ => {
                        // 나머지 enum/숫자 항목: Enter = next (Right와 동일)
                        self.settings_apply_next();
                    }
                }
            }
            _ => {}
        }
        Ok(false)
    }

    /// 현재 커서 항목을 다음 값으로 변경 (Right/Enter 공용)
    fn settings_apply_next(&mut self) {
        match self.settings_cursor {
            0 => {
                // Projects root: Enter로 편집 진입; Right로는 아무것도 하지 않음
            }
            1 => {
                self.settings_draft.default_sort.key = self.settings_draft.default_sort.key.next();
            }
            2 => {
                self.settings_draft.time_format = self.settings_draft.time_format.next();
            }
            3 => {
                self.settings_draft.resume_mode = self.settings_draft.resume_mode.next();
            }
            4 => {
                self.settings_draft.trash_retention_days =
                    self.settings_draft.trash_retention_days.saturating_add(1);
            }
            5 => {
                self.settings_draft.theme = self.settings_draft.theme.next();
            }
            _ => {}
        }
    }

    /// 현재 커서 항목을 이전 값으로 변경 (Left 전용)
    fn settings_apply_prev(&mut self) {
        match self.settings_cursor {
            0 => {
                // Projects root: 경로 편집은 Enter로만 진입
            }
            1 => {
                self.settings_draft.default_sort.key = self.settings_draft.default_sort.key.prev();
            }
            2 => {
                self.settings_draft.time_format = self.settings_draft.time_format.prev();
            }
            3 => {
                self.settings_draft.resume_mode = self.settings_draft.resume_mode.prev();
            }
            4 => {
                self.settings_draft.trash_retention_days =
                    self.settings_draft.trash_retention_days.saturating_sub(1);
            }
            5 => {
                self.settings_draft.theme = self.settings_draft.theme.prev();
            }
            _ => {}
        }
    }

    // ── 내부 동작 헬퍼 ────────────────────────────────────────────────────

    /// 삭제 확인 모달 열기
    fn open_delete_confirm(&mut self) {
        // 선택된 항목이 없으면 현재 커서 항목을 대상으로
        let target_ids: Vec<String> = if self.state.selected_ids.is_empty() {
            if let Some(s) = self.current_session() {
                vec![s.session_id.clone()]
            } else {
                return;
            }
        } else {
            self.state.selected_ids.iter().cloned().collect()
        };

        if target_ids.is_empty() {
            return;
        }

        // 대상 세션 분류
        let mut titles = vec![];
        let mut active_count = 0usize;
        let mut pending_ids = vec![];

        for sid in &target_ids {
            if let Some(session) = self.state.sessions.iter().find(|s| &s.session_id == sid) {
                if session.is_active {
                    active_count += 1;
                } else {
                    titles.push(session.title.clone());
                    pending_ids.push(sid.clone());
                }
            }
        }

        // 삭제할 게 하나도 없고 활성 세션도 없으면 조기 반환
        if pending_ids.is_empty() && active_count == 0 {
            return;
        }

        // §3: 전부 활성 세션이라 pending이 없으면 모달 열지 않고 안내만 표시
        if pending_ids.is_empty() {
            self.status_message = Some("활성 세션은 삭제 불가합니다".to_string());
            return;
        }

        self.delete_titles = titles;
        self.delete_active_count = active_count;
        self.delete_pending_ids = pending_ids;
        self.mode = UiMode::DeleteConfirm;
    }

    /// 소프트 삭제 실행
    fn execute_soft_delete(&mut self) -> Result<()> {
        let pending = self.delete_pending_ids.clone();

        // 세션 정보 수집 (session_id, path, title, cwd, is_active)
        let sessions_info: Vec<(&str, &std::path::Path, &str, &str, bool)> = pending
            .iter()
            .filter_map(|sid| {
                self.state
                    .sessions
                    .iter()
                    .find(|s| &s.session_id == sid)
                    .map(|s| {
                        (
                            s.session_id.as_str(),
                            s.path.as_path(),
                            s.title.as_str(),
                            s.cwd.as_str(),
                            s.is_active,
                        )
                    })
            })
            .collect();

        let result = soft_delete_sessions(&sessions_info)?;

        // 성공한 세션을 목록에서 제거
        for moved_id in &result.moved {
            self.state.sessions.retain(|s| &s.session_id != moved_id);
            self.state.selected_ids.remove(moved_id);
        }

        // 커서 보정
        let max = self.state.display_rows().len().saturating_sub(1);
        if self.cursor > max {
            self.cursor = max;
        }

        // 상태 메시지
        let msg = if result.moved.is_empty() {
            if result.skipped_active.is_empty() {
                "삭제할 항목이 없습니다".to_string()
            } else {
                format!(
                    "활성 세션 {}개 차단됨 (삭제 불가)",
                    result.skipped_active.len()
                )
            }
        } else {
            format!("{}개 세션을 휴지통으로 이동했습니다", result.moved.len())
        };
        self.status_message = Some(msg);

        // 삭제 확인 상태 초기화
        self.delete_titles.clear();
        self.delete_pending_ids.clear();
        self.delete_active_count = 0;
        self.state.selected_ids.clear();

        Ok(())
    }

    /// FR-14: 오래된 세션 선택 모달 열기 — 각 프리셋의 대상 수를 미리 계산해 표시.
    fn open_age_select(&mut self) {
        let now = std::time::SystemTime::now();
        self.age_counts = AGE_PRESET_DAYS
            .iter()
            .map(|&days| {
                let cutoff = now - Duration::from_secs(days * SECS_PER_DAY);
                self.state.older_than_ids(cutoff).len()
            })
            .collect();
        self.age_cursor = 0;
        self.mode = UiMode::AgeSelect;
    }

    /// 휴지통 화면 열기
    fn open_trash(&mut self) {
        self.trash_index = TrashIndex::load();
        self.trash_cursor = 0;
        self.trash_selected.clear();
        self.status_message = None;
        self.mode = UiMode::Trash;
    }

    /// 복구 실행
    fn execute_restore(&mut self) -> Result<()> {
        // 선택 없으면 커서 항목
        let target_ids: Vec<String> = if self.trash_selected.is_empty() {
            if let Some(entry) = self.current_trash_entry() {
                vec![entry.session_id.clone()]
            } else {
                return Ok(());
            }
        } else {
            self.trash_selected.iter().cloned().collect()
        };

        if target_ids.is_empty() {
            return Ok(());
        }

        let id_refs: Vec<&str> = target_ids.iter().map(|s| s.as_str()).collect();
        let result = restore_sessions(&id_refs)?;

        // 인덱스 갱신
        self.trash_index = TrashIndex::load();

        // 커서 보정
        let max = self.trash_index.entries.len().saturating_sub(1);
        if self.trash_cursor > max {
            self.trash_cursor = max;
        }

        self.trash_selected.clear();

        let msg = if result.restored.is_empty() {
            "복구 실패 — 로그를 확인하세요".to_string()
        } else {
            format!("{}개 세션을 복구했습니다", result.restored.len())
        };
        self.status_message = Some(msg);

        Ok(())
    }

    /// purge 확인 모달 열기
    fn open_purge_confirm(&mut self) {
        let target_ids: Vec<String> = if self.trash_selected.is_empty() {
            if let Some(entry) = self.current_trash_entry() {
                vec![entry.session_id.clone()]
            } else {
                return;
            }
        } else {
            self.trash_selected.iter().cloned().collect()
        };

        if target_ids.is_empty() {
            return;
        }

        // 제목 스냅샷
        let titles: Vec<String> = target_ids
            .iter()
            .filter_map(|sid| self.trash_index.entries.get(sid).map(|e| e.title.clone()))
            .collect();

        self.purge_titles = titles;
        self.purge_pending_ids = target_ids;
        self.purge_input.clear();
        self.mode = UiMode::PurgeConfirm;
    }

    /// purge 실행 (confirmed=true 게이트 포함)
    fn execute_purge(&mut self) -> Result<()> {
        let pending = self.purge_pending_ids.clone();
        let id_refs: Vec<&str> = pending.iter().map(|s| s.as_str()).collect();

        // confirmed=true 로 호출해야만 실제 삭제
        let result = purge_sessions(&id_refs, true)?;

        // 인덱스 갱신
        self.trash_index = TrashIndex::load();

        // 커서 보정
        let max = self.trash_index.entries.len().saturating_sub(1);
        if self.trash_cursor > max {
            self.trash_cursor = max;
        }

        self.trash_selected.clear();
        self.purge_pending_ids.clear();
        self.purge_titles.clear();

        let msg = if result.purged.is_empty() {
            "영구삭제 실패 — 로그를 확인하세요".to_string()
        } else {
            format!("{}개 세션을 영구삭제했습니다", result.purged.len())
        };
        self.status_message = Some(msg);

        Ok(())
    }

    /// 정렬 적용 후 커서 리셋
    fn apply_sort_and_reset_cursor(&mut self) {
        crate::service::apply_sort(&mut self.state.sessions, self.state.sort);
        self.cursor = 0;
    }

    /// 현재 커서가 가리키는 세션 참조 (facet_indices 경유)
    fn current_session(&self) -> Option<&crate::domain::Session> {
        let indices = crate::facet::facet_indices(&self.state);
        indices
            .get(self.cursor)
            .and_then(|&real_idx| self.state.sessions.get(real_idx))
    }

    /// 커서를 facet_indices 범위 내로 클램프
    fn clamp_cursor(&mut self) {
        let len = crate::facet::facet_indices(&self.state).len();
        if len == 0 {
            self.cursor = 0;
        } else if self.cursor >= len {
            self.cursor = len - 1;
        }
    }

    /// FR-15: facet 필터된 indices에서 cursor_identity 기준으로 cursor 복원
    fn restore_cursor_in_facet(&mut self) {
        if let Some(id) = &self.state.cursor_identity {
            let facet_indices = crate::facet::facet_indices(&self.state);
            for (i, &real_idx) in facet_indices.iter().enumerate() {
                if self.state.sessions[real_idx].session_id == *id {
                    self.cursor = i;
                    return;
                }
            }
            // fallback
            self.cursor = 0;
        }
    }

    /// 헤더 위에서 Space: 현재 화면에 보이는(필터된) 그룹 세션만 일괄 선택/해제.
    /// 검색으로 숨겨진 세션의 selected 상태는 건드리지 않는다 (BUG-01 수정).
    /// 핵심 로직은 `AppState::toggle_group_visible`에 위임해 단위 테스트 가능.
    fn toggle_group_selection(&mut self, cwd: &str) {
        let visible_ids = self.state.visible_group_ids(cwd);
        self.state.toggle_group_visible(&visible_ids);
    }

    /// 현재 휴지통 커서가 가리키는 항목 참조
    fn current_trash_entry(&self) -> Option<&crate::trash::TrashEntry> {
        let entries = self.trash_index.sorted_entries();
        entries.get(self.trash_cursor).copied()
    }

    // ── FR-08: 미리보기 헬퍼 ─────────────────────────────────────────────

    /// 미리보기 토글. 폭 부족 시 status_message 설정 후 반환.
    fn toggle_preview(&mut self, term_width: u16) {
        if !self.preview_open && term_width < PREVIEW_MIN_WIDTH {
            self.status_message = Some(format!(
                "터미널이 좁아 미리보기를 열 수 없습니다(≥{}칸 필요)",
                PREVIEW_MIN_WIDTH
            ));
            return;
        }
        self.preview_open = !self.preview_open;
        // 열 때 즉시 캐시 갱신
        if self.preview_open {
            self.refresh_preview_cache();
        }
    }

    /// 현재 세션이 캐시 키와 다르면 `read_preview`를 호출해 캐시를 갱신한다.
    /// 같은 키이면 재읽기 하지 않는다(캐시 hit).
    /// `preview_open`이 false이면 비용 없이 즉시 반환한다.
    fn refresh_preview_cache(&mut self) {
        if !self.preview_open {
            return;
        }
        match self.current_session() {
            Some(session) => {
                let key = PreviewCacheKey::Session(session.session_id.clone());
                // 캐시 hit 확인
                if let Some((ref cached_key, _)) = self.preview_cache {
                    if cached_key == &key {
                        return; // 같은 세션 — 재읽기 금지
                    }
                }
                // 캐시 miss — 읽기 실행
                let path = session.path.clone();
                let content = read_preview(&path, MAX_PREVIEW_LINES, MAX_PREVIEW_BYTES);
                self.preview_cache = Some((key, content));
            }
            None => {
                // 그룹 헤더 또는 빈 목록 — 빈 content 캐시
                if let Some((PreviewCacheKey::Header, _)) = self.preview_cache {
                    return; // 이미 헤더 캐시 — 재생성 불필요
                }
                self.preview_cache = Some((PreviewCacheKey::Header, PreviewContent::empty()));
            }
        }
    }

    /// 현재 미리보기 캐시의 content 참조를 반환.
    /// `preview_open`이 false이거나 캐시가 없으면 None.
    fn current_preview_content(&self) -> Option<&PreviewContent> {
        if !self.preview_open {
            return None;
        }
        self.preview_cache.as_ref().map(|(_, c)| c)
    }

    /// 현재 세션 표시 제목 반환 (미리보기 패널 타이틀용). 별칭 우선 (FR-06).
    fn current_session_title(&self) -> String {
        self.current_session()
            .map(|s| s.display_title().to_string())
            .unwrap_or_default()
    }

    /// 현재 세션의 작업 디렉토리(cwd) 전체 경로 반환 (미리보기 경로 헤더용 ①).
    /// 그룹 헤더 커서 등 세션이 아닐 땐 빈 문자열.
    fn current_session_cwd(&self) -> String {
        self.current_session()
            .map(|s| s.cwd.clone())
            .unwrap_or_default()
    }

    /// 현재 커서 위치의 세션 session_id 반환
    fn current_session_id(&self) -> Option<String> {
        let indices = crate::facet::facet_indices(&self.state);
        indices
            .get(self.cursor)
            .and_then(|&i| self.state.sessions.get(i))
            .map(|s| s.session_id.clone())
    }

    /// FS 변경 감지 시 세션 목록 재로드. cursor_identity로 커서 위치 복원.
    fn reload_sessions(&mut self) -> Result<()> {
        // 현재 커서 세션 id 저장
        self.state.cursor_identity = self.current_session_id();

        let service = crate::service::SessionService::new(self.config.clone());
        if let Ok(mut new_state) = crate::service::AppState::build(&service) {
            // 기존 UI 상태 이어받기
            new_state.facet = self.state.facet;
            new_state.launch_cwd = self.state.launch_cwd.clone();
            new_state.cursor_identity = self.state.cursor_identity.clone();
            new_state.sort = self.state.sort;
            new_state.search_query = self.state.search_query.clone();
            new_state.grouped = self.state.grouped;
            new_state.collapsed_projects = self.state.collapsed_projects.clone();
            self.state = new_state;
            // 커서 복원
            self.restore_cursor_in_facet();
        }
        // reload 실패 시 현재 상태 유지
        Ok(())
    }
}
