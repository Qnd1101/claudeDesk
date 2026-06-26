//! T11.3 색상 헬퍼 — color_enabled()==false(Mono/NO_COLOR)이면 색 미지정 Style 반환.
//!
//! 모든 색 사용처는 직접 `Style::default().fg(Color::X)`를 쓰는 대신 이 헬퍼를 거친다.
//! 이렇게 하면 `config.color_enabled()` 한 플래그만 바꿔 전역 컬러 on/off가 가능하다.
use ratatui::style::{Color, Modifier, Style};

/// fg 색을 조건부로 적용한다.
/// `enabled=false`이면 색 없는 `Style::default()` 반환.
#[inline]
pub fn cond_fg(enabled: bool, color: Color) -> Style {
    if enabled {
        Style::default().fg(color)
    } else {
        Style::default()
    }
}

/// fg 색 + BOLD를 조건부로 적용한다.
/// `enabled=false`이면 BOLD만 적용 (색 없음).
#[inline]
pub fn cond_fg_bold(enabled: bool, color: Color) -> Style {
    if enabled {
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    }
}
