use anyhow::{Context, Result};
use std::process::Command;

use crate::config::Config;
use crate::data::discover_sessions;
use crate::domain::Session;
use crate::parser::{build_session, parse_session};

// ── 정렬 (FR-07) ─────────────────────────────────────────────────────────────

/// 정렬 키 (4종)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Modified,
    Created,
    Title,
    Messages,
}

impl SortKey {
    /// 다음 키로 순환 (modified→created→title→messages→modified)
    pub fn next(self) -> Self {
        match self {
            SortKey::Modified => SortKey::Created,
            SortKey::Created => SortKey::Title,
            SortKey::Title => SortKey::Messages,
            SortKey::Messages => SortKey::Modified,
        }
    }

    /// 표시 레이블
    pub fn label(self) -> &'static str {
        match self {
            SortKey::Modified => "Modified",
            SortKey::Created => "Created",
            SortKey::Title => "Title",
            SortKey::Messages => "Messages",
        }
    }
}

/// 정렬 방향
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDir {
    Desc,
    Asc,
}

impl SortDir {
    pub fn toggle(self) -> Self {
        match self {
            SortDir::Desc => SortDir::Asc,
            SortDir::Asc => SortDir::Desc,
        }
    }

    pub fn arrow(self) -> &'static str {
        match self {
            SortDir::Desc => "↓",
            SortDir::Asc => "↑",
        }
    }
}

/// 정렬 상태 (키 + 방향)
#[derive(Debug, Clone, Copy)]
pub struct SortState {
    pub key: SortKey,
    pub dir: SortDir,
}

impl Default for SortState {
    fn default() -> Self {
        // 기본: modified desc (FR-07)
        SortState {
            key: SortKey::Modified,
            dir: SortDir::Desc,
        }
    }
}

impl SortState {
    /// `s` 키: 정렬 키 순환 (방향은 새 키 첫 기본값 desc 유지)
    pub fn cycle_key(self) -> Self {
        SortState {
            key: self.key.next(),
            dir: SortDir::Desc,
        }
    }

    /// `S` 키: 방향 토글
    pub fn toggle_dir(self) -> Self {
        SortState {
            key: self.key,
            dir: self.dir.toggle(),
        }
    }

    /// 표시 문자열 (예: "Modified ↓")
    pub fn display(self) -> String {
        format!("{} {}", self.key.label(), self.dir.arrow())
    }
}

/// Vec<Session>에 정렬 적용 (in-place, 안정 정렬).
/// `sort_by`로 방향을 비교 내부에서 처리 — `reverse()`로 인한 동일 키값 순서 불안정 방지.
pub fn apply_sort(sessions: &mut [Session], sort: SortState) {
    use std::cmp::Ordering;

    // 방향에 따라 비교를 뒤집는 클로저
    let dir_cmp = |base: Ordering| -> Ordering {
        if sort.dir == SortDir::Desc {
            base.reverse()
        } else {
            base
        }
    };

    match sort.key {
        SortKey::Modified => {
            sessions.sort_by(|a, b| dir_cmp(a.modified.cmp(&b.modified)));
        }
        SortKey::Created => {
            sessions.sort_by(|a, b| dir_cmp(a.created.cmp(&b.created)));
        }
        SortKey::Title => {
            sessions.sort_by(|a, b| dir_cmp(a.title.cmp(&b.title)));
        }
        SortKey::Messages => {
            sessions.sort_by(|a, b| dir_cmp(a.msg_count.cmp(&b.msg_count)));
        }
    }
}

// ── 서비스 ────────────────────────────────────────────────────────────────────

pub struct SessionService {
    pub config: Config,
}

impl SessionService {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// 세션 목록 빌드: 디스커버리 → 파싱 → 정렬 적용
    pub fn load_sessions(&self, sort: SortState) -> Result<(Vec<Session>, ScanStats)> {
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

        apply_sort(&mut sessions, sort);

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
    /// 현재 정렬 상태 (FR-07)
    pub sort: SortState,
    /// 검색 쿼리 (None = 검색 비활성, Some("") = 빈 쿼리)
    pub search_query: Option<String>,
    /// 다중선택된 session_id 집합 (FR-04)
    pub selected_ids: std::collections::HashSet<String>,
}

impl AppState {
    pub fn build(service: &SessionService) -> Result<Self> {
        let sort = SortState::default();
        let (sessions, stats) = service.load_sessions(sort)?;
        Ok(AppState {
            sessions,
            stats,
            projects_root: service.config.projects_root.clone(),
            sort,
            search_query: None,
            selected_ids: std::collections::HashSet::new(),
        })
    }

    /// 현재 검색 쿼리로 필터된 세션 인덱스 목록 반환
    /// 원본 sessions Vec은 불변; 인덱스 뷰만 반환 (FR-05)
    pub fn filtered_indices(&self) -> Vec<usize> {
        let all = (0..self.sessions.len()).collect();
        match &self.search_query {
            None => all,
            Some(q) if q.is_empty() => all,
            Some(query) => {
                let q = query.to_lowercase();
                self.sessions
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.search_text.contains(&q))
                    .map(|(i, _)| i)
                    .collect()
            }
        }
    }
}

// ── resume ────────────────────────────────────────────────────────────────────

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

// ── 정렬 유닛 테스트 ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::build_search_text;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    /// 테스트 세션 생성 헬퍼.
    /// search_text는 build_search_text 공용 함수로 조립 — build_session과 동일 로직 보장.
    fn make_session(title: &str, modified_secs_ago: u64, msg_count: usize) -> Session {
        make_session_with_raw(title, None, modified_secs_ago, msg_count)
    }

    fn make_session_with_raw(
        title: &str,
        first_user_raw: Option<&str>,
        modified_secs_ago: u64,
        msg_count: usize,
    ) -> Session {
        let now = SystemTime::now();
        let modified = now - Duration::from_secs(modified_secs_ago);
        let created = modified;
        let search_text = build_search_text(title, first_user_raw, "/test");
        Session {
            session_id: title.to_string(),
            title: title.to_string(),
            cwd: "/test".to_string(),
            created,
            modified,
            msg_count,
            is_active: false,
            path: PathBuf::from("/test"),
            skipped_lines: 0,
            search_text,
        }
    }

    #[test]
    fn test_sort_modified_desc() {
        let mut sessions = vec![
            make_session("old", 3600, 1),
            make_session("new", 60, 2),
            make_session("mid", 1800, 3),
        ];
        apply_sort(
            &mut sessions,
            SortState {
                key: SortKey::Modified,
                dir: SortDir::Desc,
            },
        );
        assert_eq!(sessions[0].title, "new");
        assert_eq!(sessions[1].title, "mid");
        assert_eq!(sessions[2].title, "old");
    }

    #[test]
    fn test_sort_modified_asc() {
        let mut sessions = vec![make_session("old", 3600, 1), make_session("new", 60, 2)];
        apply_sort(
            &mut sessions,
            SortState {
                key: SortKey::Modified,
                dir: SortDir::Asc,
            },
        );
        assert_eq!(sessions[0].title, "old");
        assert_eq!(sessions[1].title, "new");
    }

    #[test]
    fn test_sort_title_asc() {
        let mut sessions = vec![
            make_session("Zebra", 100, 1),
            make_session("Alpha", 200, 2),
            make_session("Beta", 150, 3),
        ];
        apply_sort(
            &mut sessions,
            SortState {
                key: SortKey::Title,
                dir: SortDir::Asc,
            },
        );
        assert_eq!(sessions[0].title, "Alpha");
        assert_eq!(sessions[1].title, "Beta");
        assert_eq!(sessions[2].title, "Zebra");
    }

    #[test]
    fn test_sort_messages_desc() {
        let mut sessions = vec![
            make_session("few", 100, 5),
            make_session("many", 200, 100),
            make_session("mid", 150, 50),
        ];
        apply_sort(
            &mut sessions,
            SortState {
                key: SortKey::Messages,
                dir: SortDir::Desc,
            },
        );
        assert_eq!(sessions[0].title, "many");
        assert_eq!(sessions[1].title, "mid");
        assert_eq!(sessions[2].title, "few");
    }

    #[test]
    fn test_sort_key_cycle() {
        let s = SortState::default();
        assert_eq!(s.key, SortKey::Modified);
        let s2 = s.cycle_key();
        assert_eq!(s2.key, SortKey::Created);
        let s3 = s2.cycle_key();
        assert_eq!(s3.key, SortKey::Title);
        let s4 = s3.cycle_key();
        assert_eq!(s4.key, SortKey::Messages);
        let s5 = s4.cycle_key();
        assert_eq!(s5.key, SortKey::Modified);
    }

    #[test]
    fn test_sort_dir_toggle() {
        let s = SortState::default();
        assert_eq!(s.dir, SortDir::Desc);
        let s2 = s.toggle_dir();
        assert_eq!(s2.dir, SortDir::Asc);
    }

    #[test]
    fn test_filtered_indices_no_query() {
        let state = AppState {
            sessions: vec![make_session("hello", 100, 1), make_session("world", 200, 2)],
            stats: ScanStats::default(),
            projects_root: PathBuf::from("/tmp"),
            sort: SortState::default(),
            search_query: None,
            selected_ids: std::collections::HashSet::new(),
        };
        let idx = state.filtered_indices();
        assert_eq!(idx, vec![0, 1]);
    }

    #[test]
    fn test_filtered_indices_with_query() {
        let state = AppState {
            sessions: vec![
                make_session("Docker 설정", 100, 1),
                make_session("Python 디버그", 200, 2),
                make_session("Docker Compose", 300, 3),
            ],
            stats: ScanStats::default(),
            projects_root: PathBuf::from("/tmp"),
            sort: SortState::default(),
            search_query: Some("docker".to_string()),
            selected_ids: std::collections::HashSet::new(),
        };
        let idx = state.filtered_indices();
        assert_eq!(idx, vec![0, 2]);
    }

    #[test]
    fn test_filtered_indices_case_insensitive() {
        let state = AppState {
            sessions: vec![
                make_session("RUST 프로젝트", 100, 1),
                make_session("python 스크립트", 200, 2),
            ],
            stats: ScanStats::default(),
            projects_root: PathBuf::from("/tmp"),
            sort: SortState::default(),
            search_query: Some("rust".to_string()),
            selected_ids: std::collections::HashSet::new(),
        };
        let idx = state.filtered_indices();
        assert_eq!(idx, vec![0]);
    }

    #[test]
    fn test_sort_display() {
        let s = SortState::default();
        assert_eq!(s.display(), "Modified ↓");
        let s2 = s.toggle_dir();
        assert_eq!(s2.display(), "Modified ↑");
    }

    /// title이 80자 절단됐을 때 절단 전 전체 텍스트로 검색 가능한지 검증 ([1]+[6])
    #[test]
    fn test_search_finds_text_beyond_truncated_title() {
        // 100자 텍스트 → title은 80자 절단, first_user_raw는 전체 보유
        let long_raw = "a".repeat(80) + "뒷부분키워드";
        let truncated_title = &long_raw[..80]; // 80 ASCII bytes = 80 chars

        let session = make_session_with_raw(truncated_title, Some(&long_raw), 100, 1);

        // title에 없는 "뒷부분키워드"로 검색해도 매칭돼야 함
        let state = AppState {
            sessions: vec![session],
            stats: ScanStats::default(),
            projects_root: PathBuf::from("/tmp"),
            sort: SortState::default(),
            search_query: Some("뒷부분키워드".to_string()),
            selected_ids: std::collections::HashSet::new(),
        };
        let idx = state.filtered_indices();
        assert_eq!(
            idx,
            vec![0],
            "80자 절단 이후 텍스트로 검색 시 매칭되지 않음 — search_text에 first_user_raw 미포함"
        );
    }
}
