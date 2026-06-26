//! T11.2 설정 화면 렌더 (FR-10 §2.6).
//! 기존 modal.rs 패턴(centered_rect, Clear, Block, Paragraph)과 동일 스타일.
//!
//! 키 조작 (렌더 전담; 실제 처리는 mod.rs App::handle_settings_key):
//!   ↑↓ / j k  : 항목 이동
//!   ← →        : 현재 항목 prev / next (enum 순환, 숫자 ±1)
//!   Enter       : Projects root = 경로 편집 진입 / Default sort = 방향 토글 / 나머지 = next()
//!   s           : 저장 (Config::save()) 후 닫기
//!   Esc         : 저장 없이 닫기 (임시 복사본 폐기)
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::layout::centered_rect;
use super::theme::{cond_fg, cond_fg_bold};
use crate::config::Config;

/// 설정 화면 렌더에 전달할 데이터
pub struct SettingsData<'a> {
    /// 편집용 임시 복사본
    pub draft: &'a Config,
    /// 커서 위치 (0=Projects root … 5=Theme)
    pub cursor: usize,
    /// Projects root 인라인 텍스트 편집 모드 여부
    pub path_editing: bool,
    /// 경로 편집 중인 버퍼 (path_editing=true일 때만 의미 있음)
    pub path_input: &'a str,
    /// 색상 활성 여부 (T11.3)
    pub color_enabled: bool,
}

/// 설정 항목 개수 (상수로 노출해 키 핸들러와 공유)
pub const SETTINGS_ROW_COUNT: usize = 6;

// 항목 레이블 (인덱스 순)
const LABELS: [&str; SETTINGS_ROW_COUNT] = [
    "Projects root",
    "Default sort",
    "Time format",
    "Resume mode",
    "Trash retention",
    "Theme",
];

/// 설정 화면 모달 렌더 (§2.6 목업 기준)
pub fn render_settings(f: &mut Frame, data: &SettingsData<'_>) {
    // path_editing 행 유무에 따라 높이 조정
    let height: u16 = if data.path_editing { 15 } else { 14 };
    let area = centered_rect(72, height, f.area());

    f.render_widget(Clear, area);

    let border_style = cond_fg(data.color_enabled, Color::Cyan);
    let title_style = cond_fg_bold(data.color_enabled, Color::Cyan);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Settings ", title_style))
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = vec![Line::from("")];

    // ── 항목 행 ────────────────────────────────────────────────────────────
    for (i, label) in LABELS.iter().enumerate() {
        let is_cursor = i == data.cursor;
        let marker = if is_cursor { "▸ " } else { "  " };

        let (value_str, options_str) = build_item_display(i, data);

        let marker_style = cond_fg(data.color_enabled, Color::Yellow);
        let label_style = if is_cursor {
            cond_fg_bold(data.color_enabled, Color::Yellow)
        } else {
            Style::default()
        };
        let value_style = if is_cursor {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let options_style = cond_fg(data.color_enabled, Color::DarkGray);

        let mut spans = vec![
            Span::styled(marker, marker_style),
            Span::styled(format!("{:<17}", label), label_style),
            Span::styled(format!("{:<18}", value_str), value_style),
        ];
        if !options_str.is_empty() {
            spans.push(Span::styled(options_str, options_style));
        }
        lines.push(Line::from(spans));
    }

    // ── 경로 편집 서브 행 ─────────────────────────────────────────────────
    if data.path_editing {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("경로: ", cond_fg(data.color_enabled, Color::Yellow)),
            Span::styled(
                data.path_input,
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("│", Style::default()),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Enter 확인 · Esc 취소",
                cond_fg(data.color_enabled, Color::DarkGray),
            ),
        ]));
    }

    // ── 푸터 ──────────────────────────────────────────────────────────────
    lines.push(Line::from(""));
    lines.push(build_footer_line(data.color_enabled));
    lines.push(Line::from(""));

    let para = Paragraph::new(lines).alignment(Alignment::Left);
    f.render_widget(para, inner);
}

/// 항목별 현재 값 문자열과 옵션 힌트 문자열 반환.
/// (value_str, options_str) — options_str이 빈 문자열이면 힌트 없음.
fn build_item_display(row: usize, data: &SettingsData<'_>) -> (String, String) {
    let d = data.draft;
    match row {
        0 => {
            // Projects root
            let path_str = if data.path_editing {
                data.path_input.to_string()
            } else {
                d.projects_root.to_string_lossy().to_string()
            };
            (
                path_str,
                if data.cursor == 0 {
                    "  Enter 편집".to_string()
                } else {
                    String::new()
                },
            )
        }
        1 => {
            // Default sort (key + dir)
            let val = format!(
                "{} {}",
                d.default_sort.key.label(),
                d.default_sort.dir.arrow()
            );
            let hint = if data.cursor == 1 {
                "  ◀ Modified/Created/Title/Messages ▶  Enter dir토글".to_string()
            } else {
                String::new()
            };
            (val, hint)
        }
        2 => {
            // Time format
            let val = d.time_format.label().to_string();
            let hint = if data.cursor == 2 {
                "  ◀ Relative / Absolute ▶".to_string()
            } else {
                String::new()
            };
            (val, hint)
        }
        3 => {
            // Resume mode
            let val = d.resume_mode.label().to_string();
            let hint = if data.cursor == 3 {
                "  ◀ Handoff / Spawn ▶".to_string()
            } else {
                String::new()
            };
            (val, hint)
        }
        4 => {
            // Trash retention
            let val = format!("{} days", d.trash_retention_days);
            let hint = if data.cursor == 4 {
                "  ← -1 day  → +1 day".to_string()
            } else {
                String::new()
            };
            (val, hint)
        }
        5 => {
            // Theme
            let val = d.theme.label().to_string();
            let hint = if data.cursor == 5 {
                "  ◀ Auto / Dark / Light / Mono ▶".to_string()
            } else {
                String::new()
            };
            (val, hint)
        }
        _ => (String::new(), String::new()),
    }
}

/// 하단 키 안내 행
fn build_footer_line(color_enabled: bool) -> Line<'static> {
    Line::from(vec![
        Span::raw(" "),
        Span::styled("↑↓ 이동", cond_fg(color_enabled, Color::Yellow)),
        Span::raw("  ·  "),
        Span::styled("←→ 변경", cond_fg(color_enabled, Color::Yellow)),
        Span::raw("  ·  "),
        Span::styled("Enter 확인/토글", cond_fg(color_enabled, Color::Yellow)),
        Span::raw("  ·  "),
        Span::styled("s 저장", cond_fg_bold(color_enabled, Color::Green)),
        Span::raw("  ·  "),
        Span::styled("Esc 닫기", cond_fg(color_enabled, Color::DarkGray)),
    ])
}

// ── 단위 테스트 ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CliOverrides, Config};
    use tempfile::TempDir;

    fn make_test_config() -> (Config, TempDir) {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        let cli = CliOverrides {
            config: Some(path),
            ..CliOverrides::default()
        };
        let cfg = Config::load(&cli).unwrap();
        (cfg, tmp)
    }

    #[test]
    fn test_build_item_display_projects_root() {
        let (cfg, _tmp) = make_test_config();
        let data = SettingsData {
            draft: &cfg,
            cursor: 0,
            path_editing: false,
            path_input: "",
            color_enabled: false,
        };
        let (val, hint) = build_item_display(0, &data);
        assert!(!val.is_empty(), "projects_root 값이 비어 있으면 안 됨");
        assert!(
            hint.contains("Enter"),
            "Projects root 힌트에 Enter 안내가 있어야 함"
        );
    }

    #[test]
    fn test_build_item_display_default_sort() {
        let (cfg, _tmp) = make_test_config();
        let data = SettingsData {
            draft: &cfg,
            cursor: 1,
            path_editing: false,
            path_input: "",
            color_enabled: false,
        };
        let (val, hint) = build_item_display(1, &data);
        // 기본값: Modified ↓
        assert!(val.contains("Modified"), "SortKey 표시 누락: {val}");
        assert!(
            val.contains('↓') || val.contains('↑'),
            "SortDir 화살표 누락: {val}"
        );
        assert!(hint.contains('▶'), "sort 항목 힌트 누락: {hint}");
    }

    #[test]
    fn test_build_item_display_time_format() {
        let (cfg, _tmp) = make_test_config();
        let data = SettingsData {
            draft: &cfg,
            cursor: 2,
            path_editing: false,
            path_input: "",
            color_enabled: false,
        };
        let (val, hint) = build_item_display(2, &data);
        assert_eq!(val.trim(), "Relative", "기본 TimeFormat 표시 불일치: {val}");
        assert!(
            hint.contains("Absolute"),
            "hint에 Absolute 포함 필요: {hint}"
        );
    }

    #[test]
    fn test_build_item_display_trash_retention() {
        let (cfg, _tmp) = make_test_config();
        let data = SettingsData {
            draft: &cfg,
            cursor: 4,
            path_editing: false,
            path_input: "",
            color_enabled: false,
        };
        let (val, hint) = build_item_display(4, &data);
        assert!(
            val.contains("30"),
            "trash_retention_days 기본값 30 표시 필요: {val}"
        );
        assert!(val.contains("days"), "unit 'days' 표시 필요: {val}");
        assert!(hint.contains("day"), "힌트에 day 안내 필요: {hint}");
    }

    #[test]
    fn test_build_item_display_theme() {
        let (cfg, _tmp) = make_test_config();
        let data = SettingsData {
            draft: &cfg,
            cursor: 5,
            path_editing: false,
            path_input: "",
            color_enabled: false,
        };
        let (val, hint) = build_item_display(5, &data);
        assert_eq!(val.trim(), "Auto", "기본 Theme 표시 불일치: {val}");
        assert!(hint.contains("Mono"), "Mono 힌트 필요: {hint}");
    }

    #[test]
    fn test_non_cursor_row_has_no_hint() {
        let (cfg, _tmp) = make_test_config();
        // cursor=0, row=2 → hint 없음
        let data = SettingsData {
            draft: &cfg,
            cursor: 0,
            path_editing: false,
            path_input: "",
            color_enabled: false,
        };
        let (_, hint) = build_item_display(2, &data);
        assert!(
            hint.is_empty(),
            "커서가 아닌 행에 힌트가 표시되면 안 됨: '{hint}'"
        );
    }

    #[test]
    fn settings_row_count_matches_labels() {
        assert_eq!(SETTINGS_ROW_COUNT, LABELS.len());
    }
}
