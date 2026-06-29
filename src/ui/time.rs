use chrono::{DateTime, Local, NaiveDate};
use std::time::SystemTime;

#[cfg(test)]
use std::time::Duration;

use crate::config::TimeFormat;

/// SystemTime → 상대시간 문자열 (한국어, TimeFormat::Relative용)
pub fn relative_time(t: &SystemTime) -> String {
    let now_local = Local::now();
    let t_local: DateTime<Local> = (*t).into();

    let now_date: NaiveDate = now_local.date_naive();
    let t_date: NaiveDate = t_local.date_naive();

    // 오늘이면 시간 단위 상대
    if t_date == now_date {
        let secs = now_local
            .signed_duration_since(t_local)
            .num_seconds()
            .max(0) as u64;
        return if secs < 60 {
            "방금 전".to_string()
        } else if secs < 3600 {
            format!("{}분 전", secs / 60)
        } else {
            format!("{}시간 전", secs / 3600)
        };
    }

    // 어제
    if t_date == now_date - chrono::Duration::days(1) {
        return "어제".to_string();
    }

    // 이전 날짜
    let days = (now_date - t_date).num_days().max(0) as u64;
    if days < 7 {
        format!("{}일 전", days)
    } else if days < 30 {
        format!("{}주 전", days / 7)
    } else if days < 365 {
        format!("{}달 전", days / 30)
    } else {
        format!("{}년 전", days / 365)
    }
}

/// SystemTime → 절대시간 문자열 로컬 타임존 (TimeFormat::Absolute용)
/// 예: "2026-06-26 12:40"
pub fn absolute_time(t: &SystemTime) -> String {
    let dt: DateTime<Local> = (*t).into();
    dt.format("%Y-%m-%d %H:%M").to_string()
}

/// TimeFormat에 따라 상대/절대 시간 중 선택해 반환.
pub fn format_time(t: &SystemTime, fmt: TimeFormat) -> String {
    match fmt {
        TimeFormat::Relative => relative_time(t),
        TimeFormat::Absolute => absolute_time(t),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_just_now() {
        let t = SystemTime::now() - Duration::from_secs(10);
        assert_eq!(relative_time(&t), "방금 전");
    }

    #[test]
    fn test_relative_minutes() {
        let t = SystemTime::now() - Duration::from_secs(130);
        assert_eq!(relative_time(&t), "2분 전");
    }

    #[test]
    fn test_relative_hours() {
        let t = SystemTime::now() - Duration::from_secs(3700);
        assert_eq!(relative_time(&t), "1시간 전");
    }

    #[test]
    fn test_relative_yesterday() {
        // 어제 정오 (달력 날짜 기준 명확히 어제)
        let yesterday_noon = Local::now()
            .date_naive()
            .pred_opt()
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();
        let t = SystemTime::from(yesterday_noon);
        assert_eq!(relative_time(&t), "어제");
    }

    #[test]
    fn test_relative_days() {
        let t = SystemTime::now() - Duration::from_secs(86400 * 3);
        assert_eq!(relative_time(&t), "3일 전");
    }

    #[test]
    fn test_future_time() {
        let t = SystemTime::now() + Duration::from_secs(100);
        assert_eq!(relative_time(&t), "방금 전");
    }

    #[test]
    fn test_absolute_time_format() {
        // 특정 에포크 시각의 포맷 구조 검증 (로컬 타임존이므로 정확한 값은 환경 의존)
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_750_000_000);
        let s = absolute_time(&t);
        // "YYYY-MM-DD HH:MM" 형식인지 확인
        assert_eq!(s.len(), 16, "absolute_time 형식 길이 불일치: '{s}'");
        assert!(s.contains('-'), "날짜 구분자 '-' 없음: '{s}'");
        assert!(s.contains(':'), "시간 구분자 ':' 없음: '{s}'");
    }

    #[test]
    fn test_format_time_relative() {
        let t = SystemTime::now() - Duration::from_secs(130);
        let s = format_time(&t, TimeFormat::Relative);
        assert_eq!(s, "2분 전");
    }

    #[test]
    fn test_format_time_absolute() {
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_750_000_000);
        let s = format_time(&t, TimeFormat::Absolute);
        // 길이 + 구분자 확인 (로컬 타임존 의존 — 값 자체는 환경마다 다름)
        assert_eq!(s.len(), 16, "absolute 형식 불일치: '{s}'");
    }
}
