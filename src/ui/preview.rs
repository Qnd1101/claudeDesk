/// FR-08 미리보기 패널 렌더러
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::preview::PreviewContent;

/// 미리보기 패널을 area에 렌더한다.
///
/// - `content`: `read_preview`가 반환한 대화 내용
/// - `session_title`: 패널 상단 타이틀에 표시할 세션 제목(짧게 잘라 사용)
pub fn render_preview(f: &mut Frame, area: Rect, content: &PreviewContent, session_title: &str) {
    // 타이틀: "Preview — 세션제목(최대 20자)" 형식
    let short_title = truncate_title(session_title, 20);
    let block_title = if short_title.is_empty() {
        " Preview ".to_string()
    } else {
        format!(" Preview — {} ", short_title)
    };

    // turns가 비어 있는 경우 (빈 파일, 그룹 헤더 커서 등)
    if content.turns.is_empty() {
        let msg = "미리보기 없음\n(세션에 대화 내용이 없습니다)";
        let p = Paragraph::new(msg)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(block_title)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .style(Style::default().fg(Color::DarkGray))
            .wrap(Wrap { trim: false });
        f.render_widget(p, area);
        return;
    }

    // 대화 턴을 Line 목록으로 변환
    let mut lines: Vec<Line> = vec![];

    for (i, turn) in content.turns.iter().enumerate() {
        // 턴 구분 빈 줄 (첫 턴 제외)
        if i > 0 {
            lines.push(Line::from(""));
        }

        // 역할 헤더
        let (role_symbol, role_label, role_color) = if turn.role == "user" {
            ("●", "user     ", Color::Cyan)
        } else {
            ("○", "assistant", Color::DarkGray)
        };

        lines.push(Line::from(vec![Span::styled(
            format!("{} {}", role_symbol, role_label),
            Style::default().fg(role_color).add_modifier(Modifier::BOLD),
        )]));

        // 텍스트 본문: 개행 기준으로 각 줄을 Line으로 변환
        for text_line in turn.text.lines() {
            lines.push(Line::from(vec![Span::styled(
                format!("  {}", text_line),
                Style::default().fg(Color::White),
            )]));
        }
    }

    // truncated 안내
    if content.truncated {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "  … (미리보기 일부 — 전체는 Enter로 이어하기)",
            Style::default().fg(Color::DarkGray),
        )]));
    }

    // 스킵된 줄 수 표시
    if content.skipped_lines > 0 {
        lines.push(Line::from(vec![Span::styled(
            format!("  [파싱 불가 {}줄 스킵]", content.skipped_lines),
            Style::default().fg(Color::DarkGray),
        )]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(block_title)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// 미리보기 패널 타이틀용 — chars 기준으로 max_chars자로 자름
fn truncate_title(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = chars[..max_chars].iter().collect();
        format!("{}…", truncated.trim_end())
    }
}
