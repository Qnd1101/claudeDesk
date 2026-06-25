use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_help(f: &mut Frame) {
    let area = centered_rect(48, 16, f.area());

    f.render_widget(Clear, area);

    let block = Block::default().borders(Borders::ALL).title(Span::styled(
        " Keys ",
        Style::default().add_modifier(Modifier::BOLD),
    ));

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ↑/k  ", Style::default().fg(Color::Yellow)),
            Span::raw("위로 이동          "),
            Span::styled("↓/j  ", Style::default().fg(Color::Yellow)),
            Span::raw("아래로 이동"),
        ]),
        Line::from(vec![
            Span::styled("  PgUp ", Style::default().fg(Color::Yellow)),
            Span::raw("페이지 위           "),
            Span::styled("PgDn ", Style::default().fg(Color::Yellow)),
            Span::raw("페이지 아래"),
        ]),
        Line::from(vec![
            Span::styled("  Home ", Style::default().fg(Color::Yellow)),
            Span::raw("처음               "),
            Span::styled("End  ", Style::default().fg(Color::Yellow)),
            Span::raw("끝"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(Color::Green)),
            Span::raw("  세션 이어하기 (resume)"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ?    ", Style::default().fg(Color::Cyan)),
            Span::raw("도움말 토글        "),
            Span::styled("q/Esc", Style::default().fg(Color::Red)),
            Span::raw("  종료"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+C ", Style::default().fg(Color::Red)),
            Span::raw("강제 종료"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  [M2] 검색(/), 정렬(s), 삭제(Del) — 추후 구현",
            Style::default().fg(Color::DarkGray),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Esc / ? 로 닫기",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

/// 중앙 정렬 Rect (percent_x, height in lines)
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let popup_width = width.min(area.width);
    let popup_height = height.min(area.height);

    // Layout을 이용해 수직/수평 중앙
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(popup_height)) / 2),
            Constraint::Length(popup_height),
            Constraint::Min(0),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((area.width.saturating_sub(popup_width)) / 2),
            Constraint::Length(popup_width),
            Constraint::Min(0),
        ])
        .split(vertical[1]);

    horizontal[1]
}
