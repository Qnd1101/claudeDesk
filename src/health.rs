use std::time::SystemTime;

/// 세션 정리 상태 분류 (FR-10 정리 기능)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Health {
    /// 정상: cwd 존재 · 메시지 있음 · stale 아님
    #[default]
    Active,
    /// 메시지 없음 (msg_count == 0)
    Empty,
    /// 미접근 (modified < stale_cutoff, 기본 90일 이상)
    Stale,
    /// 디렉토리 부재 (cwd 경로 삭제됨)
    Zombie,
}

impl Health {
    /// 상태를 표시용 라벨로 변환
    pub fn label(self) -> &'static str {
        match self {
            Health::Active => "Active",
            Health::Empty => "Empty",
            Health::Stale => "Stale",
            Health::Zombie => "Zombie",
        }
    }

    /// 정리 대상 여부 (Stale 또는 Zombie)
    pub fn is_cleanup(self) -> bool {
        matches!(self, Health::Stale | Health::Zombie)
    }
}

/// 세션을 정리 상태로 분류 (순수 함수)
///
/// 우선순위: Zombie > Empty > Stale > Active
/// 즉, cwd 부재를 가장 먼저 체크, 이후 메시지 유무, 마지막으로 접근 시간.
///
/// # Arguments
/// * `msg_count` - 메시지 수 (user + assistant)
/// * `cwd_exists` - 작업 디렉토리 존재 여부
/// * `modified` - 마지막 수정 시각
/// * `stale_cutoff` - stale 판정 기준 시각 (예: 현재 - 90일)
pub fn classify(
    msg_count: usize,
    cwd_exists: bool,
    modified: SystemTime,
    stale_cutoff: SystemTime,
) -> Health {
    if !cwd_exists {
        return Health::Zombie;
    }
    if msg_count == 0 {
        return Health::Empty;
    }
    if modified < stale_cutoff {
        return Health::Stale;
    }
    Health::Active
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn make_time(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn test_classify_zombie() {
        let result = classify(1, false, make_time(1000), make_time(500));
        assert_eq!(result, Health::Zombie);
    }

    #[test]
    fn test_classify_empty() {
        let result = classify(0, true, make_time(1000), make_time(500));
        assert_eq!(result, Health::Empty);
    }

    #[test]
    fn test_classify_stale() {
        let result = classify(5, true, make_time(100), make_time(500));
        assert_eq!(result, Health::Stale);
    }

    #[test]
    fn test_classify_active() {
        let result = classify(10, true, make_time(1000), make_time(500));
        assert_eq!(result, Health::Active);
    }

    #[test]
    fn test_health_label() {
        assert_eq!(Health::Active.label(), "Active");
        assert_eq!(Health::Empty.label(), "Empty");
        assert_eq!(Health::Stale.label(), "Stale");
        assert_eq!(Health::Zombie.label(), "Zombie");
    }

    #[test]
    fn test_health_is_cleanup() {
        assert!(!Health::Active.is_cleanup());
        assert!(!Health::Empty.is_cleanup());
        assert!(Health::Stale.is_cleanup());
        assert!(Health::Zombie.is_cleanup());
    }

    #[test]
    fn test_zombie_priority() {
        // cwd_exists=false이면 다른 조건과 무관하게 Zombie
        let result = classify(0, false, make_time(1000), make_time(500));
        assert_eq!(result, Health::Zombie);
    }

    #[test]
    fn test_empty_priority_over_stale() {
        // msg_count=0이면 modified 값과 무관하게 Empty
        let result = classify(0, true, make_time(100), make_time(500));
        assert_eq!(result, Health::Empty);
    }
}
