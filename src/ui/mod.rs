mod help;
mod list;
mod time;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::time::Duration;

use crate::service::{exec_resume, resume_session, AppState, ResumeResult};
use help::render_help;
use list::render_list;

/// UI 모드
#[derive(Debug, Clone, PartialEq, Eq)]
enum UiMode {
    /// 일반 목록 모드
    Normal,
    /// 검색 모드 (/ 진입)
    Search,
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
                    render_list(f, &self.state, self.cursor, self.mode == UiMode::Search);
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

        // ── 검색 모드 ─────────────────────────────────────────────────────
        if self.mode == UiMode::Search {
            return self.handle_search_key(code);
        }

        // ── 일반 모드 ─────────────────────────────────────────────────────
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

    /// 검색 모드 키 처리
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
}
