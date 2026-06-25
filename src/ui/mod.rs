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

pub struct App {
    state: AppState,
    selected: usize,
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
            selected: 0,
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
                    render_list(f, &self.state, self.selected);
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

        match code {
            // 종료
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(true);
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(true);
            }

            // 이동
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.state.sessions.is_empty() && self.selected < self.state.sessions.len() - 1
                {
                    self.selected += 1;
                }
            }
            KeyCode::Home => {
                self.selected = 0;
            }
            KeyCode::End => {
                if !self.state.sessions.is_empty() {
                    self.selected = self.state.sessions.len() - 1;
                }
            }
            KeyCode::PageUp => {
                self.selected = self.selected.saturating_sub(10);
            }
            KeyCode::PageDown => {
                let max = self.state.sessions.len().saturating_sub(1);
                self.selected = (self.selected + 10).min(max);
            }

            // 도움말
            KeyCode::Char('?') => {
                self.show_help = true;
            }

            // Resume
            KeyCode::Enter => {
                if let Some(session) = self.state.sessions.get(self.selected) {
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
}
