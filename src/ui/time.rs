use std::time::{Duration, SystemTime};

/// SystemTime → 상대시간 문자열 (한국어)
pub fn relative_time(t: &SystemTime) -> String {
    let now = SystemTime::now();
    let diff = match now.duration_since(*t) {
        Ok(d) => d,
        Err(_) => Duration::from_secs(0), // 미래 시간 방어
    };

    let secs = diff.as_secs();

    if secs < 60 {
        "방금 전".to_string()
    } else if secs < 3600 {
        format!("{}분 전", secs / 60)
    } else if secs < 86400 {
        format!("{}시간 전", secs / 3600)
    } else if secs < 86400 * 2 {
        "어제".to_string()
    } else if secs < 86400 * 7 {
        format!("{}일 전", secs / 86400)
    } else if secs < 86400 * 30 {
        format!("{}주 전", secs / (86400 * 7))
    } else if secs < 86400 * 365 {
        format!("{}달 전", secs / (86400 * 30))
    } else {
        format!("{}년 전", secs / (86400 * 365))
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
        let t = SystemTime::now() - Duration::from_secs(90000);
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
}
