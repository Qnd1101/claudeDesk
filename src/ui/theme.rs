//! 색상 테마 헬퍼 — Palette로 Light/Dark/Mono를 통합 관리.
use ratatui::style::{Color, Modifier, Style};

use crate::config::Theme;

/// 렌더 전체에 전달하는 팔레트 (Copy — struct 필드에 직접 보관 가능).
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Palette {
    /// 색상 활성 여부 (false=Mono/NO_COLOR)
    pub enabled: bool,
    /// 테두리·헤더·타이틀 강조 (Cyan / Blue)
    pub accent: Color,
    /// 활성 세션 ● (Green / DarkGreen)
    pub active: Color,
    /// 비활성·보조 텍스트 (DarkGray / DarkGray)
    pub muted: Color,
    /// 경고·Stale·탭 선택 (Yellow / DarkYellow)
    pub warning: Color,
    /// 오류·Zombie·삭제 위험 (Red / DarkRed)
    pub danger: Color,
    /// 확인·저장·Enter (Green / DarkGreen)
    pub success: Color,
    /// 정렬 키 (Magenta / DarkMagenta)
    pub sort: Color,
    /// 그룹 모드 (Blue / DarkBlue)
    pub group: Color,
    /// 탐색 키 힌트 (Yellow / DarkYellow)
    pub nav: Color,
    /// 강조 배경 위 텍스트 (White / Black)
    pub body: Color,
    /// 미리보기 user 메시지 (Cyan / Blue)
    pub user_msg: Color,
}

impl Palette {
    pub fn dark() -> Self {
        Palette {
            enabled: true,
            accent: Color::Cyan,
            active: Color::Green,
            muted: Color::DarkGray,
            warning: Color::Yellow,
            danger: Color::Red,
            success: Color::Green,
            sort: Color::Magenta,
            group: Color::Blue,
            nav: Color::Yellow,
            body: Color::White,
            user_msg: Color::Cyan,
        }
    }

    pub fn light() -> Self {
        Palette {
            enabled: true,
            accent: Color::Blue,
            active: Color::Green,
            muted: Color::DarkGray,
            warning: Color::Yellow,
            danger: Color::Red,
            success: Color::Green,
            sort: Color::Magenta,
            group: Color::Blue,
            nav: Color::Yellow,
            body: Color::Black,
            user_msg: Color::Blue,
        }
    }

    pub fn mono() -> Self {
        Palette {
            enabled: false,
            accent: Color::Reset,
            active: Color::Reset,
            muted: Color::Reset,
            warning: Color::Reset,
            danger: Color::Reset,
            success: Color::Reset,
            sort: Color::Reset,
            group: Color::Reset,
            nav: Color::Reset,
            body: Color::Reset,
            user_msg: Color::Reset,
        }
    }

    pub fn from_theme(theme: Theme) -> Self {
        match theme {
            Theme::Auto | Theme::Dark => Self::dark(),
            Theme::Light => Self::light(),
            Theme::Mono => Self::mono(),
        }
    }

    /// fg 색을 조건부로 적용 (enabled=false이면 색 없는 Style).
    #[inline]
    pub fn fg(&self, color: Color) -> Style {
        if self.enabled {
            Style::default().fg(color)
        } else {
            Style::default()
        }
    }

    /// fg 색 + BOLD 조건부 적용.
    #[inline]
    pub fn fg_bold(&self, color: Color) -> Style {
        if self.enabled {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().add_modifier(Modifier::BOLD)
        }
    }
}

// 하위 호환: 기존 cond_fg / cond_fg_bold 시그니처를 유지하는 래퍼.
// (단계적 마이그레이션 완료 후 제거 가능)
#[allow(dead_code)]
#[inline]
pub fn cond_fg(enabled: bool, color: Color) -> Style {
    if enabled {
        Style::default().fg(color)
    } else {
        Style::default()
    }
}

#[allow(dead_code)]
#[inline]
pub fn cond_fg_bold(enabled: bool, color: Color) -> Style {
    if enabled {
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    }
}
