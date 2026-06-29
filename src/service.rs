use anyhow::{Context, Result};
use std::process::Command;

use crate::config::Config;
use crate::data::discover_sessions;
use crate::domain::Session;
use crate::parser::{build_search_text, build_session, parse_session};

// ── 정렬 (FR-07) ─────────────────────────────────────────────────────────────
// SortKey, SortDir は config.rs で定義 (serde + FR-10 設定レイヤー)。
// ここでは re-export して後方互換性を維持する。
pub use crate::config::{SortDir, SortKey};

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
            sessions.sort_by(|a, b| dir_cmp(a.display_title().cmp(b.display_title())));
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

    /// 세션 목록 빌드: 디스커버리 → 파싱 → 별칭 주입 → 정렬 적용
    pub fn load_sessions(
        &self,
        sort: SortState,
        aliases: &crate::alias::AliasStore,
    ) -> Result<(Vec<Session>, ScanStats)> {
        let mut stats = ScanStats::default();

        let file_metas =
            discover_sessions(&self.config.projects_root).context("세션 파일 탐색 실패")?;

        let mut sessions = Vec::with_capacity(file_metas.len());

        for meta in &file_metas {
            match parse_session(meta) {
                Ok(result) => {
                    stats.skipped_lines += result.skipped_lines;
                    // file_stem = session_id 로 별칭 조회 (parser가 alias 모듈 미의존 — 레이어 분리)
                    let session_id = meta.path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    let alias = aliases.get(session_id);
                    let session =
                        build_session(meta, result, self.config.active_window_secs, alias);
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

/// 그룹 모드 표시 행 (FR-09)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayRow {
    /// 프로젝트 그룹 헤더
    Header {
        cwd: String,
        project_name: String,
        count: usize,
        collapsed: bool,
    },
    /// 세션 행 (sessions Vec 내 실제 인덱스)
    Session(usize),
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
    /// 그룹 모드 활성 여부 (FR-09, 기본 false)
    pub grouped: bool,
    /// 접힌 프로젝트 cwd 집합 (FR-09)
    pub collapsed_projects: std::collections::HashSet<String>,
    /// 별칭 사이드카 (FR-06). 편집 시 메모리 갱신 + 즉시 save.
    pub aliases: crate::alias::AliasStore,
    /// 현재 좌측 facet (FR-15, 기본 config.default_facet)
    pub facet: crate::facet::Facet,
    /// 실행 디렉토리 (Project facet/우측 기본용, FR-15)
    pub launch_cwd: String,
}

impl AppState {
    pub fn build(service: &SessionService) -> Result<Self> {
        let aliases = crate::alias::AliasStore::load();
        // FR-10 T11.2 배선: config.default_sort를 초기 정렬 기본값으로 사용
        let sort = SortState {
            key: service.config.default_sort.key,
            dir: service.config.default_sort.dir,
        };
        let (sessions, stats) = service.load_sessions(sort, &aliases)?;
        let launch_cwd = std::env::current_dir()?
            .to_string_lossy()
            .to_string();
        Ok(AppState {
            sessions,
            stats,
            projects_root: service.config.projects_root.clone(),
            sort,
            search_query: None,
            selected_ids: std::collections::HashSet::new(),
            grouped: false,
            collapsed_projects: std::collections::HashSet::new(),
            aliases,
            facet: service.config.default_facet,
            launch_cwd,
        })
    }

    /// 현재 모드(평면/그룹)에 따른 표시 행 목록 반환 (FR-09)
    pub fn display_rows(&self) -> Vec<DisplayRow> {
        use crate::domain::project_name_of;

        let indices = self.filtered_indices();

        if !self.grouped {
            return indices.into_iter().map(DisplayRow::Session).collect();
        }

        let mut group_order: Vec<String> = Vec::new();
        let mut groups: std::collections::HashMap<String, (std::time::SystemTime, Vec<usize>)> =
            std::collections::HashMap::new();

        for &idx in &indices {
            let session = &self.sessions[idx];
            let cwd = session.cwd.clone();
            if !groups.contains_key(&cwd) {
                group_order.push(cwd.clone());
                groups.insert(cwd.clone(), (session.modified, vec![idx]));
            } else {
                let entry = groups.get_mut(&cwd).unwrap();
                if session.modified > entry.0 {
                    entry.0 = session.modified;
                }
                entry.1.push(idx);
            }
        }

        group_order.sort_by(|a, b| {
            let ta = groups[a].0;
            let tb = groups[b].0;
            tb.cmp(&ta)
        });

        let mut rows = Vec::new();
        for cwd in &group_order {
            let (_, session_indices) = &groups[cwd];
            let count = session_indices.len();
            let collapsed = self.collapsed_projects.contains(cwd);
            let pname = project_name_of(cwd).to_string();

            rows.push(DisplayRow::Header {
                cwd: cwd.clone(),
                project_name: pname,
                count,
                collapsed,
            });

            if !collapsed {
                for &idx in session_indices {
                    rows.push(DisplayRow::Session(idx));
                }
            }
        }

        rows
    }

    /// 헤더 Space 키 동작 핵심 로직: visible_ids(필터된 그룹 세션 id 슬라이스)를
    /// 기준으로 selected_ids를 대칭 토글한다.
    ///
    /// - 전부 선택돼 있으면 visible_ids만 해제 (숨겨진 세션은 건드리지 않음)
    /// - 아니면 visible_ids 전체 선택
    ///
    /// UI 레이어(`toggle_group_selection`)와 테스트가 모두 이 메서드를 공유한다 (BUG-01).
    pub fn toggle_group_visible(&mut self, visible_ids: &[String]) {
        if visible_ids.is_empty() {
            return;
        }
        let all_selected = visible_ids
            .iter()
            .all(|sid| self.selected_ids.contains(sid));

        if all_selected {
            for sid in visible_ids {
                self.selected_ids.remove(sid);
            }
        } else {
            for sid in visible_ids {
                self.selected_ids.insert(sid.clone());
            }
        }
    }

    /// 지정 cwd 그룹에서 현재 검색 필터에 보이는 세션 id 목록 반환 (BUG-01 테스트 지원)
    pub fn visible_group_ids(&self, cwd: &str) -> Vec<String> {
        self.filtered_indices()
            .iter()
            .filter_map(|&i| self.sessions.get(i))
            .filter(|s| s.cwd == cwd)
            .map(|s| s.session_id.clone())
            .collect()
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

    /// FR-14: cutoff 시각보다 이전에 수정된, 현재 필터에 보이는 비활성 세션 id 목록.
    /// 활성 세션(최근 수정 휴리스틱)은 정의상 cutoff 이후라 빠지지만 방어적으로 `!is_active` 가드.
    /// 검색 필터(filtered_indices) 범위에서만 동작 — `a`(전체선택)와 동일 스코프.
    pub fn older_than_ids(&self, cutoff: std::time::SystemTime) -> Vec<String> {
        self.filtered_indices()
            .iter()
            .filter_map(|&i| self.sessions.get(i))
            .filter(|s| !s.is_active && s.modified < cutoff)
            .map(|s| s.session_id.clone())
            .collect()
    }

    /// FR-14: cutoff 이전 비활성 세션을 selected_ids에 추가(기존 선택은 보존).
    /// 반환: 대상 세션 수. 삭제는 하지 않는다 — 기존 `d`→삭제확인 흐름으로 위임(안전핀).
    pub fn select_older_than(&mut self, cutoff: std::time::SystemTime) -> usize {
        let ids = self.older_than_ids(cutoff);
        for id in &ids {
            self.selected_ids.insert(id.clone());
        }
        ids.len()
    }

    /// 별칭 설정 + 사이드카 원자적 저장 + 해당 세션 메모리 갱신 (FR-06).
    /// first_user_raw는 메모리에 없으므로 None으로 재조립 (의도적 트레이드오프 — plan §3.5).
    pub fn set_alias(&mut self, session_id: &str, new_alias: &str) -> anyhow::Result<()> {
        self.aliases.set(session_id, new_alias);
        self.aliases.save()?;
        // 소유권 충돌 방지: get 결과를 String으로 복사한 뒤 세션 갱신
        let alias_val = self.aliases.get(session_id).map(|s| s.to_string());
        if let Some(s) = self
            .sessions
            .iter_mut()
            .find(|s| s.session_id == session_id)
        {
            let title = s.title.clone();
            let cwd = s.cwd.clone();
            s.alias = alias_val.clone();
            s.search_text = build_search_text(&title, None, &cwd, alias_val.as_deref());
        }
        Ok(())
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
        let search_text = build_search_text(title, first_user_raw, "/test", None);
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
            alias: None,
            search_text,
            health: crate::health::Health::Active,
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
            grouped: false,
            collapsed_projects: std::collections::HashSet::new(),
            aliases: crate::alias::AliasStore::default(),
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
            grouped: false,
            collapsed_projects: std::collections::HashSet::new(),
            aliases: crate::alias::AliasStore::default(),
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
            grouped: false,
            collapsed_projects: std::collections::HashSet::new(),
            aliases: crate::alias::AliasStore::default(),
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

    fn make_session_with_cwd(
        title: &str,
        cwd: &str,
        modified_secs_ago: u64,
        msg_count: usize,
    ) -> Session {
        let now = SystemTime::now();
        let modified = now - Duration::from_secs(modified_secs_ago);
        let created = modified;
        let search_text = build_search_text(title, None, cwd, None);
        Session {
            session_id: title.to_string(),
            title: title.to_string(),
            cwd: cwd.to_string(),
            created,
            modified,
            msg_count,
            is_active: false,
            path: PathBuf::from(cwd),
            skipped_lines: 0,
            alias: None,
            search_text,
            health: crate::health::Health::Active,
        }
    }

    /// FR-06: Title 정렬이 도출 title이 아닌 display_title(별칭 우선) 기준인지 (LOW-2 회귀 방지)
    #[test]
    fn test_sort_title_uses_display_title() {
        let mut zebra = make_session("Zebra", 100, 1);
        zebra.alias = Some("Aardvark".to_string()); // display_title = "Aardvark"
        let apple = make_session("Apple", 200, 1); // 별칭 없음 → display_title = "Apple"
        let mut sessions = vec![zebra, apple];
        apply_sort(
            &mut sessions,
            SortState {
                key: SortKey::Title,
                dir: SortDir::Asc,
            },
        );
        // 별칭 "Aardvark" < "Apple" → 별칭 단 Zebra 세션이 먼저 와야 한다
        assert_eq!(
            sessions[0].title, "Zebra",
            "Title 정렬이 display_title(별칭) 기준이 아님"
        );
        assert_eq!(sessions[1].title, "Apple");
    }

    #[test]
    fn test_display_rows_flat_mode() {
        let state = AppState {
            sessions: vec![make_session("A", 100, 1), make_session("B", 200, 2)],
            stats: ScanStats::default(),
            projects_root: PathBuf::from("/tmp"),
            sort: SortState::default(),
            search_query: None,
            selected_ids: std::collections::HashSet::new(),
            grouped: false,
            collapsed_projects: std::collections::HashSet::new(),
            aliases: crate::alias::AliasStore::default(),
        };
        let rows = state.display_rows();
        assert_eq!(rows, vec![DisplayRow::Session(0), DisplayRow::Session(1)]);
    }

    #[test]
    fn test_display_rows_grouped_headers_before_sessions() {
        let state = AppState {
            sessions: vec![
                make_session_with_cwd("S1", "/proj/alpha", 100, 1),
                make_session_with_cwd("S2", "/proj/beta", 200, 2),
            ],
            stats: ScanStats::default(),
            projects_root: PathBuf::from("/tmp"),
            sort: SortState::default(),
            search_query: None,
            selected_ids: std::collections::HashSet::new(),
            grouped: true,
            collapsed_projects: std::collections::HashSet::new(),
            aliases: crate::alias::AliasStore::default(),
        };
        let rows = state.display_rows();
        // alpha (100 secs ago) is more recent -> alpha first
        assert_eq!(rows.len(), 4);
        match &rows[0] {
            DisplayRow::Header { project_name, .. } => assert_eq!(project_name, "alpha"),
            _ => panic!("Expected Header for alpha first"),
        }
        assert_eq!(rows[1], DisplayRow::Session(0));
        match &rows[2] {
            DisplayRow::Header { project_name, .. } => assert_eq!(project_name, "beta"),
            _ => panic!("Expected Header for beta second"),
        }
        assert_eq!(rows[3], DisplayRow::Session(1));
    }

    #[test]
    fn test_display_rows_collapsed_hides_sessions() {
        let mut collapsed = std::collections::HashSet::new();
        collapsed.insert("/proj/alpha".to_string());
        let state = AppState {
            sessions: vec![
                make_session_with_cwd("S1", "/proj/alpha", 100, 1),
                make_session_with_cwd("S2", "/proj/beta", 200, 2),
            ],
            stats: ScanStats::default(),
            projects_root: PathBuf::from("/tmp"),
            sort: SortState::default(),
            search_query: None,
            selected_ids: std::collections::HashSet::new(),
            grouped: true,
            collapsed_projects: collapsed,
            aliases: crate::alias::AliasStore::default(),
        };
        let rows = state.display_rows();
        assert_eq!(rows.len(), 3);
        match &rows[0] {
            DisplayRow::Header {
                project_name,
                collapsed,
                ..
            } => {
                assert_eq!(project_name, "alpha");
                assert!(*collapsed);
            }
            _ => panic!("Expected Header for alpha"),
        }
        match &rows[1] {
            DisplayRow::Header {
                project_name,
                collapsed,
                ..
            } => {
                assert_eq!(project_name, "beta");
                assert!(!*collapsed);
            }
            _ => panic!("Expected Header for beta"),
        }
        assert_eq!(rows[2], DisplayRow::Session(1));
    }

    #[test]
    fn test_display_rows_group_order_by_recent_modified() {
        let state = AppState {
            sessions: vec![
                make_session_with_cwd("S1", "/proj/alpha", 50, 1),
                make_session_with_cwd("S2", "/proj/beta", 300, 2),
            ],
            stats: ScanStats::default(),
            projects_root: PathBuf::from("/tmp"),
            sort: SortState::default(),
            search_query: None,
            selected_ids: std::collections::HashSet::new(),
            grouped: true,
            collapsed_projects: std::collections::HashSet::new(),
            aliases: crate::alias::AliasStore::default(),
        };
        let rows = state.display_rows();
        match &rows[0] {
            DisplayRow::Header { project_name, .. } => assert_eq!(project_name, "alpha"),
            _ => panic!("Expected Header"),
        }
    }

    #[test]
    fn test_display_rows_grouped_with_search() {
        let state = AppState {
            sessions: vec![
                make_session_with_cwd("Docker setup", "/proj/alpha", 100, 1),
                make_session_with_cwd("Python debug", "/proj/beta", 200, 2),
            ],
            stats: ScanStats::default(),
            projects_root: PathBuf::from("/tmp"),
            sort: SortState::default(),
            search_query: Some("docker".to_string()),
            selected_ids: std::collections::HashSet::new(),
            grouped: true,
            collapsed_projects: std::collections::HashSet::new(),
            aliases: crate::alias::AliasStore::default(),
        };
        let rows = state.display_rows();
        assert_eq!(rows.len(), 2);
        match &rows[0] {
            DisplayRow::Header {
                project_name,
                count,
                ..
            } => {
                assert_eq!(project_name, "alpha");
                assert_eq!(*count, 1);
            }
            _ => panic!("Expected Header"),
        }
        assert_eq!(rows[1], DisplayRow::Session(0));
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
            grouped: false,
            collapsed_projects: std::collections::HashSet::new(),
            aliases: crate::alias::AliasStore::default(),
        };
        let idx = state.filtered_indices();
        assert_eq!(
            idx,
            vec![0],
            "80자 절단 이후 텍스트로 검색 시 매칭되지 않음 — search_text에 first_user_raw 미포함"
        );
    }

    // ── BUG-01 회귀 테스트: toggle_group_visible 대칭성 ───────────────────────

    /// 헬퍼: 주어진 세션 id들이 모두 selected_ids에 포함돼 있는 상태의 AppState 생성.
    fn state_with_selected(
        sessions: Vec<Session>,
        selected: &[&str],
        search_query: Option<String>,
    ) -> AppState {
        let mut selected_ids = std::collections::HashSet::new();
        for &sid in selected {
            selected_ids.insert(sid.to_string());
        }
        AppState {
            sessions,
            stats: ScanStats::default(),
            projects_root: PathBuf::from("/tmp"),
            sort: SortState::default(),
            search_query,
            selected_ids,
            grouped: true,
            collapsed_projects: std::collections::HashSet::new(),
            aliases: crate::alias::AliasStore::default(),
        }
    }

    /// BUG-01 시나리오 1: 검색으로 그룹의 일부만 visible할 때,
    /// 헤더 토글이 visible 세션만 선택하고 hidden 세션의 selected 상태는 보존됨.
    #[test]
    fn test_toggle_group_visible_preserves_hidden_selected() {
        // alpha 그룹: "Docker A"(visible, 검색 매칭), "Python B"(hidden, 검색 미매칭)
        // Python B는 이미 selected 상태 — 토글 후에도 유지돼야 한다.
        let sessions = vec![
            make_session_with_cwd("Docker A", "/proj/alpha", 100, 1),
            make_session_with_cwd("Python B", "/proj/alpha", 200, 2),
        ];
        let mut state = state_with_selected(
            sessions,
            &["Python B"], // hidden 세션 미리 선택
            Some("docker".to_string()),
        );

        let visible = state.visible_group_ids("/proj/alpha");
        assert_eq!(
            visible,
            vec!["Docker A"],
            "검색 필터 후 visible은 Docker A만"
        );

        // 헤더 Space → visible(Docker A)만 선택돼야 함
        state.toggle_group_visible(&visible);
        assert!(
            state.selected_ids.contains("Docker A"),
            "Docker A가 선택돼야 함"
        );
        assert!(
            state.selected_ids.contains("Python B"),
            "숨겨진 Python B의 selected 상태가 보존돼야 함 (BUG-01)"
        );
    }

    /// BUG-01 시나리오 2: 전부 visible + 전부 선택 상태에서 토글 → visible 전부 해제.
    /// 단, 다른 그룹의 selected 상태는 건드리지 않음.
    #[test]
    fn test_toggle_group_visible_deselects_all_visible() {
        let sessions = vec![
            make_session_with_cwd("S1", "/proj/alpha", 100, 1),
            make_session_with_cwd("S2", "/proj/alpha", 200, 2),
            make_session_with_cwd("S3", "/proj/beta", 300, 3),
        ];
        // alpha 둘 다 선택, beta도 선택
        let mut state = state_with_selected(sessions, &["S1", "S2", "S3"], None);

        let visible = state.visible_group_ids("/proj/alpha");
        assert_eq!(visible.len(), 2);

        // 전부 선택 상태 → 해제
        state.toggle_group_visible(&visible);
        assert!(!state.selected_ids.contains("S1"), "S1 해제돼야 함");
        assert!(!state.selected_ids.contains("S2"), "S2 해제돼야 함");
        assert!(
            state.selected_ids.contains("S3"),
            "다른 그룹 S3의 selected 상태는 유지돼야 함"
        );
    }

    /// BUG-01 시나리오 3: 일부만 선택 상태에서 토글 → visible 전부 선택 (기존 선택 유지).
    #[test]
    fn test_toggle_group_visible_selects_all_when_partial() {
        let sessions = vec![
            make_session_with_cwd("S1", "/proj/alpha", 100, 1),
            make_session_with_cwd("S2", "/proj/alpha", 200, 2),
        ];
        // S1만 선택된 부분 선택 상태
        let mut state = state_with_selected(sessions, &["S1"], None);

        let visible = state.visible_group_ids("/proj/alpha");
        assert_eq!(visible.len(), 2);

        // 일부 선택 → 전부 선택
        state.toggle_group_visible(&visible);
        assert!(state.selected_ids.contains("S1"), "S1 선택 유지");
        assert!(state.selected_ids.contains("S2"), "S2도 새로 선택됨");
    }

    /// BUG-01 해제 대칭성: 검색 중 visible 세션만 해제하고 hidden은 보존.
    /// (구 버전은 group_ids 전체를 해제해 hidden 세션도 풀렸음)
    #[test]
    fn test_toggle_group_visible_deselect_only_visible_not_hidden() {
        // alpha: "Docker A"(visible), "Python B"(hidden). 둘 다 선택.
        let sessions = vec![
            make_session_with_cwd("Docker A", "/proj/alpha", 100, 1),
            make_session_with_cwd("Python B", "/proj/alpha", 200, 2),
        ];
        let mut state = state_with_selected(
            sessions,
            &["Docker A", "Python B"],
            Some("docker".to_string()), // Python B는 검색에 안 걸림
        );

        let visible = state.visible_group_ids("/proj/alpha");
        assert_eq!(visible, vec!["Docker A"]);

        // visible인 Docker A만 선택된 상태 → 해제 (all_selected=true, visible=["Docker A"])
        state.toggle_group_visible(&visible);

        assert!(
            !state.selected_ids.contains("Docker A"),
            "Docker A는 해제돼야 함"
        );
        assert!(
            state.selected_ids.contains("Python B"),
            "숨겨진 Python B는 구 버전처럼 같이 해제되면 안 됨 (BUG-01 핵심)"
        );
    }

    // ── FR-14: 날짜 기준 오래된 세션 선택 ────────────────────────────────────

    const DAY: u64 = 86_400;

    /// 평면 모드 기본 AppState (검색 없음, 선택 없음)
    fn plain_state(sessions: Vec<Session>) -> AppState {
        AppState {
            sessions,
            stats: ScanStats::default(),
            projects_root: PathBuf::from("/tmp"),
            sort: SortState::default(),
            search_query: None,
            selected_ids: std::collections::HashSet::new(),
            grouped: false,
            collapsed_projects: std::collections::HashSet::new(),
            aliases: crate::alias::AliasStore::default(),
        }
    }

    /// cutoff(30일 전) 이전 세션만 대상이 되고, 최근 세션은 빠진다.
    #[test]
    fn test_older_than_ids_selects_only_old() {
        let state = plain_state(vec![
            make_session("old", 100 * DAY, 1),   // 100일 전 → 대상
            make_session("recent", 10 * DAY, 1), // 10일 전 → 제외
        ]);
        let cutoff = SystemTime::now() - Duration::from_secs(30 * DAY);
        let ids = state.older_than_ids(cutoff);
        assert_eq!(ids, vec!["old"]);
    }

    /// 활성 세션은 cutoff 이전이라도 방어적으로 제외된다.
    #[test]
    fn test_older_than_ids_excludes_active() {
        let mut active = make_session("active-old", 100 * DAY, 1);
        active.is_active = true; // 비정상 케이스(오래됐는데 활성) — 그래도 제외돼야
        let state = plain_state(vec![active, make_session("dead-old", 100 * DAY, 1)]);
        let cutoff = SystemTime::now() - Duration::from_secs(30 * DAY);
        let ids = state.older_than_ids(cutoff);
        assert_eq!(ids, vec!["dead-old"], "활성 세션이 선택 대상에 포함됨");
    }

    /// select_older_than은 selected_ids에 추가하고 기존 선택을 보존한다(삭제는 안 함).
    #[test]
    fn test_select_older_than_adds_and_preserves() {
        let mut state = plain_state(vec![
            make_session("old1", 90 * DAY, 1),
            make_session("old2", 60 * DAY, 1),
            make_session("recent", 5 * DAY, 1),
        ]);
        state.selected_ids.insert("recent".to_string()); // 기존 선택 보존돼야
        let cutoff = SystemTime::now() - Duration::from_secs(30 * DAY);
        let n = state.select_older_than(cutoff);
        assert_eq!(n, 2, "30일 이전 2개가 대상이어야");
        assert!(state.selected_ids.contains("old1"));
        assert!(state.selected_ids.contains("old2"));
        assert!(
            state.selected_ids.contains("recent"),
            "기존 선택(recent)이 보존돼야"
        );
        // 세션 목록은 변하지 않음(삭제 아님)
        assert_eq!(state.sessions.len(), 3);
    }

    /// 검색 필터가 켜져 있으면 보이는(매칭) 세션 중에서만 대상이 잡힌다.
    #[test]
    fn test_older_than_ids_respects_search_filter() {
        let mut state = plain_state(vec![
            make_session("docker-old", 100 * DAY, 1),
            make_session("python-old", 100 * DAY, 1),
        ]);
        state.search_query = Some("docker".to_string());
        let cutoff = SystemTime::now() - Duration::from_secs(30 * DAY);
        let ids = state.older_than_ids(cutoff);
        assert_eq!(ids, vec!["docker-old"], "검색에 가려진 세션은 대상 제외");
    }
}
