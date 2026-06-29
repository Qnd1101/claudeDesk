//! FR-15 facet 필터 (Recent/Active/Cleanup/Project).
use crate::domain::Session;
use crate::health::Health;
use crate::service::AppState;
use serde::{Deserialize, Serialize};

/// 좌측 필터 탭 열거형. serde는 lowercase(recent/active/cleanup/project) 직렬화.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Facet {
    #[default]
    Recent,
    Active,
    Cleanup,
    Project,
}

impl Facet {
    /// 다음 facet 순환 (Recent→Active→Cleanup→Project→Recent)
    pub fn next(self) -> Self {
        match self {
            Facet::Recent => Facet::Active,
            Facet::Active => Facet::Cleanup,
            Facet::Cleanup => Facet::Project,
            Facet::Project => Facet::Recent,
        }
    }

    /// 이전 facet 순환 (역방향)
    pub fn prev(self) -> Self {
        match self {
            Facet::Recent => Facet::Project,
            Facet::Active => Facet::Recent,
            Facet::Cleanup => Facet::Active,
            Facet::Project => Facet::Cleanup,
        }
    }

    /// UI 표시 레이블
    pub fn label(self) -> &'static str {
        match self {
            Facet::Recent => "Recent",
            Facet::Active => "Active",
            Facet::Cleanup => "Cleanup",
            Facet::Project => "Project",
        }
    }

    /// 문자열 파싱 (--facet 인자용). lowercase.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "recent" => Some(Facet::Recent),
            "active" => Some(Facet::Active),
            "cleanup" => Some(Facet::Cleanup),
            "project" => Some(Facet::Project),
            _ => None,
        }
    }

    /// 모든 facet 순서 (탭바 표시)
    pub fn all() -> [Facet; 4] {
        [Facet::Recent, Facet::Active, Facet::Cleanup, Facet::Project]
    }

    /// 숫자 키 매핑: 1~4 → 각 facet (없으면 None)
    pub fn from_digit(d: u32) -> Option<Self> {
        match d {
            1 => Some(Facet::Recent),
            2 => Some(Facet::Active),
            3 => Some(Facet::Cleanup),
            4 => Some(Facet::Project),
            _ => None,
        }
    }

    /// facet → 숫자 (1~4)
    pub fn to_digit(self) -> u32 {
        match self {
            Facet::Recent => 1,
            Facet::Active => 2,
            Facet::Cleanup => 3,
            Facet::Project => 4,
        }
    }

    /// 숫자 키 매핑 (1~4 → index)
    pub fn index(self) -> usize {
        self.to_digit() as usize
    }
}

impl std::fmt::Display for Facet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// 한 세션이 해당 facet에 속하는지 판단 (순수 술어).
///
/// # Arguments
/// * `facet` - 대상 facet
/// * `s` - 세션
/// * `launch_cwd` - claudedesk 실행 디렉토리 (Project 필터용)
pub fn matches(facet: Facet, s: &Session, launch_cwd: &str) -> bool {
    match facet {
        Facet::Recent => true, // 전체 (정렬은 호출측)
        Facet::Active => s.health == Health::Active,
        Facet::Cleanup => s.health.is_cleanup(),
        Facet::Project => cwd_is_under(&s.cwd, launch_cwd),
    }
}

/// 경로가 base의 하위인지 판정 (Windows 안전하게 경로 정규화 후 prefix 비교).
///
/// 예:
/// - `cwd_is_under("D:\\MyProject\\src", "D:\\MyProject")` → true
/// - `cwd_is_under("D:\\MyProject", "D:\\MyProject")` → true (같음)
/// - `cwd_is_under("D:\\MyProject2", "D:\\MyProject")` → false (경계 확인)
fn cwd_is_under(cwd: &str, base: &str) -> bool {
    // Windows: 대소문자 무시 + 경로 분리자 정규화
    let normalize = |p: &str| -> String { p.to_lowercase().replace('/', "\\") };

    let norm_cwd = normalize(cwd);
    let norm_base = normalize(base);

    // base가 cwd와 정확히 같으면 true
    if norm_cwd == norm_base {
        return true;
    }

    // base가 cwd의 prefix인지 확인 (경계: 경로 분리자)
    if norm_cwd.starts_with(&norm_base) {
        return norm_cwd[norm_base.len()..].starts_with('\\');
    }

    false
}

/// 탭 뱃지용 4-facet 카운트 (검색 무시, 전체 기준).
pub fn counts(state: &AppState) -> [usize; 4] {
    let mut result = [0usize; 4];
    for (i, facet) in Facet::all().iter().enumerate() {
        result[i] = state
            .sessions
            .iter()
            .filter(|s| matches(*facet, s, &state.launch_cwd))
            .count();
    }
    result
}

/// 현재 facet에 맞는 세션 indices (검색 필터 포함)
pub fn facet_indices(state: &AppState) -> Vec<usize> {
    state
        .sessions
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            // facet 필터
            matches(state.facet, s, &state.launch_cwd)
            // 검색 필터 (기존 filtered_indices 로직과 동일)
            && state.search_query.as_deref()
                .map(|q| q.is_empty() || s.search_text.to_lowercase().contains(&q.to_lowercase()))
                .unwrap_or(true)
        })
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn make_session(cwd: &str) -> Session {
        Session {
            session_id: "test".to_string(),
            title: "Test".to_string(),
            cwd: cwd.to_string(),
            created: SystemTime::now(),
            modified: SystemTime::now(),
            msg_count: 10,
            is_active: false,
            path: std::path::PathBuf::from("test.jsonl"),
            skipped_lines: 0,
            alias: None,
            search_text: "test".to_string(),
            health: Health::Active,
        }
    }

    #[test]
    fn facet_next_cycles() {
        assert_eq!(Facet::Recent.next(), Facet::Active);
        assert_eq!(Facet::Active.next(), Facet::Cleanup);
        assert_eq!(Facet::Cleanup.next(), Facet::Project);
        assert_eq!(Facet::Project.next(), Facet::Recent);
    }

    #[test]
    fn facet_parse_recent() {
        assert_eq!(Facet::parse("recent"), Some(Facet::Recent));
    }

    #[test]
    fn matches_recent_returns_true() {
        let s = make_session("D:\\test");
        assert!(matches(Facet::Recent, &s, "D:\\"));
    }

    #[test]
    fn facet_index_mapping() {
        assert_eq!(Facet::Recent.index(), 1);
        assert_eq!(Facet::Active.index(), 2);
        assert_eq!(Facet::Cleanup.index(), 3);
        assert_eq!(Facet::Project.index(), 4);
    }

    #[test]
    fn cwd_is_under_windows_paths() {
        // 정확히 같음
        assert!(cwd_is_under("D:\\MyProject", "D:\\MyProject"));
        // 하위 폴더
        assert!(cwd_is_under("D:\\MyProject\\src", "D:\\MyProject"));
        // 경계 확인: 유사 이름이지만 다른 폴더
        assert!(!cwd_is_under("D:\\MyProject2", "D:\\MyProject"));
    }

    #[test]
    fn cwd_is_under_case_insensitive() {
        assert!(cwd_is_under("D:\\MYPROJECT\\SRC", "d:\\myproject"));
        assert!(cwd_is_under("d:\\MyProject\\src", "D:\\MYPROJECT"));
    }

    #[test]
    fn cwd_is_under_forward_slash_normalization() {
        assert!(cwd_is_under("D:/MyProject/src", "D:\\MyProject"));
        assert!(cwd_is_under("D:\\MyProject\\src", "D:/MyProject"));
    }
}
