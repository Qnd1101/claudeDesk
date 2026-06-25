use anyhow::{Context, Result};
use std::process::Command;

use crate::config::Config;
use crate::data::discover_sessions;
use crate::domain::Session;
use crate::parser::{build_session, parse_session};

pub struct SessionService {
    pub config: Config,
}

impl SessionService {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// 세션 목록 빌드: 디스커버리 → 파싱 → 정렬(modified desc)
    pub fn load_sessions(&self) -> Result<(Vec<Session>, ScanStats)> {
        let mut stats = ScanStats::default();

        let file_metas =
            discover_sessions(&self.config.projects_root).context("세션 파일 탐색 실패")?;

        let mut sessions = Vec::with_capacity(file_metas.len());

        for meta in &file_metas {
            match parse_session(meta) {
                Ok(result) => {
                    stats.skipped_lines += result.skipped_lines;
                    let session = build_session(meta, result, self.config.active_window_secs);
                    sessions.push(session);
                }
                Err(e) => {
                    stats.skipped_files += 1;
                    if self.config.verbose {
                        eprintln!("파싱 실패 {}: {}", meta.path.display(), e);
                    }
                }
            }
        }

        // 기본 정렬: modified desc
        sessions.sort_by_key(|s| std::cmp::Reverse(s.modified));

        Ok((sessions, stats))
    }
}

/// 스캔 통계 (FR-12)
#[derive(Debug, Default, Clone)]
pub struct ScanStats {
    pub skipped_lines: usize,
    pub skipped_files: usize,
}

/// 앱 전체 상태
pub struct AppState {
    pub sessions: Vec<Session>,
    pub stats: ScanStats,
    pub projects_root: std::path::PathBuf,
}

impl AppState {
    pub fn build(service: &SessionService) -> Result<Self> {
        let (sessions, stats) = service.load_sessions()?;
        Ok(AppState {
            sessions,
            stats,
            projects_root: service.config.projects_root.clone(),
        })
    }
}

/// resume 서비스: Enter 시 호출
/// - alt-screen 복원 후 child process로 claude --resume 실행
/// - claude 미발견 시 명령 출력 안내
pub fn resume_session(session: &Session) -> ResumeResult {
    // claude PATH 확인
    let claude_available = which_claude().is_some();

    if !claude_available {
        return ResumeResult::NotFound {
            cwd: session.cwd.clone(),
            session_id: session.session_id.clone(),
        };
    }

    ResumeResult::Ready {
        cwd: session.cwd.clone(),
        session_id: session.session_id.clone(),
    }
}

pub enum ResumeResult {
    Ready { cwd: String, session_id: String },
    NotFound { cwd: String, session_id: String },
}

/// 실제 프로세스 실행 (TUI 종료 후 호출)
pub fn exec_resume(cwd: &str, session_id: &str) -> Result<()> {
    let cwd_path = if cwd.is_empty() {
        std::env::current_dir().unwrap_or_default()
    } else {
        std::path::PathBuf::from(cwd)
    };

    let mut cmd = Command::new("claude");
    cmd.arg("--resume").arg(session_id).current_dir(&cwd_path);

    // stdio 상속: 부모 터미널 그대로 사용
    let mut child = cmd
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .context("claude 실행 실패")?;

    child.wait().context("claude 프로세스 대기 실패")?;
    Ok(())
}

/// claude 바이너리가 PATH에 있는지 확인
pub fn which_claude() -> Option<std::path::PathBuf> {
    // which 크레이트 없이 직접 탐색
    let path_var = std::env::var("PATH").unwrap_or_default();
    let extensions = if cfg!(windows) {
        vec!["cmd", "exe", "bat", ""]
    } else {
        vec![""]
    };

    for dir in std::env::split_paths(&path_var) {
        for ext in &extensions {
            let candidate = if ext.is_empty() {
                dir.join("claude")
            } else {
                dir.join(format!("claude.{}", ext))
            };
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
