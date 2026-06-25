mod help;
mod layout;
mod list;
mod modal;
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

use crate::service::{exec_resume, resume_session, AppState, ResumeResult};
use crate::trash::{purge_sessions, restore_sessions, soft_delete_sessions, TrashIndex};
use help::render_help;
use list::render_list;
use modal::{render_delete_confirm, render_purge_confirm, DeleteConfirmData, PurgeConfirmData};
use trash_view::render_trash;

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
}

pub struct App {
    state: AppState,
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

    // ── 상태 메시지 ───────────────────────────────────────────────────────
    /// 임시 상태 메시지 (작업 결과 표시용)
    status_message: Option<String>,
}

impl App {
    pub fn new(state: AppState) -> Self {
        App {
            state,
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

            status_message: None,
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
                if self.show_help {
                    render_help(f);
                } else {
                    match &self.mode {
                        UiMode::Trash => {
                            let entries = self.trash_index.sorted_entries();
                            render_trash(f, &entries, self.trash_cursor, &self.trash_selected);
                        }
                        UiMode::DeleteConfirm => {
                            render_list(
                                f,
                                &self.state,
                                self.cursor,
                                false,
                                &self.state.selected_ids.clone(),
                                self.status_message.as_deref(),
                            );
                            let data = DeleteConfirmData {
                                titles: &self.delete_titles,
                                active_count: self.delete_active_count,
                            };
                            render_delete_confirm(f, &data);
                        }
                        UiMode::PurgeConfirm => {
                            let entries = self.trash_index.sorted_entries();
                            render_trash(f, &entries, self.trash_cursor, &self.trash_selected);
                            let data = PurgeConfirmData {
                                titles: &self.purge_titles,
                                input: &self.purge_input,
                            };
                            render_purge_confirm(f, &data);
                        }
                        _ => {
                            let search_mode = self.mode == UiMode::Search;
                            render_list(
                                f,
                                &self.state,
                                self.cursor,
                                search_mode,
                                &self.state.selected_ids.clone(),
                                self.status_message.as_deref(),
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
                let count = self.state.filtered_indices().len();
                if count > 0 && self.cursor < count - 1 {
                    self.cursor += 1;
                }
            }
            KeyCode::Home => {
                self.cursor = 0;
            }
            KeyCode::End => {
                let count = self.state.filtered_indices().len();
                if count > 0 {
                    self.cursor = count - 1;
                }
            }
            KeyCode::PageUp => {
                self.cursor = self.cursor.saturating_sub(10);
            }
            KeyCode::PageDown => {
                let max = self.state.filtered_indices().len().saturating_sub(1);
                self.cursor = (self.cursor + 10).min(max);
            }

            // 도움말
            KeyCode::Char('?') => {
                self.show_help = true;
            }

            // ── M2: 다중선택 (Space, FR-04) ─────────────────────────────
            KeyCode::Char(' ') => {
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
                let visible_ids: Vec<String> = self
                    .state
                    .filtered_indices()
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

            // Resume
            KeyCode::Enter => {
                if let Some(session) = self.current_session() {
                    match resume_session(session) {
                        ResumeResult::Ready { cwd, session_id } => {
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
                let count = self.state.filtered_indices().len();
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
        let max = self.state.filtered_indices().len().saturating_sub(1);
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

    /// 현재 커서가 가리키는 세션 참조 (필터 인덱스 경유)
    fn current_session(&self) -> Option<&crate::domain::Session> {
        let indices = self.state.filtered_indices();
        let real_idx = indices.get(self.cursor)?;
        self.state.sessions.get(*real_idx)
    }

    /// 현재 휴지통 커서가 가리키는 항목 참조
    fn current_trash_entry(&self) -> Option<&crate::trash::TrashEntry> {
        let entries = self.trash_index.sorted_entries();
        entries.get(self.trash_cursor).copied()
    }
}
