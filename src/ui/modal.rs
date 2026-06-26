/// 삭제 확인 모달 (FR-04, §2.4) + purge 2단계 확인 모달 (FR-11) + 별칭 편집 모달 (FR-06)
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::layout::centered_rect;

// ── 소프트 삭제 확인 모달 (§2.4) ────────────────────────────────────────────

/// 삭제 확인 모달에 전달할 데이터
pub struct DeleteConfirmData<'a> {
    /// 삭제할 세션 목록 (제목)
    pub titles: &'a [String],
    /// 활성 세션이라 스킵될 수 (표시용)
    pub active_count: usize,
}

/// 소프트 삭제 확인 모달 렌더 (FR-04 §2.4)
/// Enter = 확인, Esc = 취소
pub fn render_delete_confirm(f: &mut Frame, data: &DeleteConfirmData<'_>) {
    let list_lines = data.titles.len().min(5) as u16;
    let height = 9 + list_lines;
    let area = centered_rect(58, height, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " 삭제 확인 ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{}개 세션을 휴지통으로 이동합니다.", data.titles.len()),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    let show = data.titles.len().min(5);
    for title in data.titles.iter().take(show) {
        lines.push(Line::from(vec![
            Span::raw("    · "),
            Span::styled(title.as_str(), Style::default().fg(Color::White)),
        ]));
    }
    if data.titles.len() > 5 {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("... 외 {}개", data.titles.len() - 5),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    if data.active_count > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("⚠ 활성 세션 {}개는 제외됩니다(●).", data.active_count),
                Style::default().fg(Color::Red),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("      "),
        Span::styled(
            "[Enter] 휴지통 이동",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("    "),
        Span::styled("[Esc] 취소", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(""));

    let para = Paragraph::new(lines).alignment(Alignment::Left);
    f.render_widget(para, inner);
}

// ── purge 2단계 확인 모달 ────────────────────────────────────────────────────

/// purge 확인 모달에 전달할 데이터
pub struct PurgeConfirmData<'a> {
    pub titles: &'a [String],
    /// 사용자가 입력 중인 확인 문자열 ("DELETE" 타이핑)
    pub input: &'a str,
}

/// purge 확인 모달 렌더 (FR-11 §2.7 영구삭제 2단계)
/// "DELETE" 타이핑 후 Enter = 확인, Esc = 취소
pub fn render_purge_confirm(f: &mut Frame, data: &PurgeConfirmData<'_>) {
    let list_lines = data.titles.len().min(5) as u16;
    let height = 13 + list_lines;
    let area = centered_rect(60, height, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " 영구삭제 확인 ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "⚠ 영구삭제는 복구할 수 없습니다!",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{}개 세션을 완전히 삭제합니다:", data.titles.len()),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    let show = data.titles.len().min(5);
    for title in data.titles.iter().take(show) {
        lines.push(Line::from(vec![
            Span::raw("    · "),
            Span::styled(title.as_str(), Style::default().fg(Color::White)),
        ]));
    }
    if data.titles.len() > 5 {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("... 외 {}개", data.titles.len() - 5),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "확인하려면 DELETE 를 입력하세요:",
            Style::default().fg(Color::Yellow),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  > "),
        Span::styled(data.input, Style::default().fg(Color::White)),
        Span::styled("│", Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(""));

    let ready = data.input == "DELETE";
    lines.push(Line::from(vec![
        Span::raw("      "),
        if ready {
            Span::styled(
                "[Enter] 영구삭제",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                "[Enter] 영구삭제 (DELETE 입력 후)",
                Style::default().fg(Color::DarkGray),
            )
        },
        Span::raw("    "),
        Span::styled("[Esc] 취소", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(""));

    let para = Paragraph::new(lines).alignment(Alignment::Left);
    f.render_widget(para, inner);
}

// ── 별칭 편집 모달 (FR-06) ──────────────────────────────────────────────────

/// 별칭 편집 모달에 전달할 데이터
pub struct AliasEditData<'a> {
    /// 세션 원본 제목 (별칭 편집 전 표시용)
    pub original_title: &'a str,
    /// 현재 입력 중인 별칭 문자열
    pub input: &'a str,
}

/// 별칭 지정/편집 모달 렌더 (FR-06 §3.6)
/// Enter = 저장 (빈칸 = 별칭 삭제), Esc = 취소
pub fn render_alias_edit(f: &mut Frame, data: &AliasEditData<'_>) {
    let height = 9u16;
    let area = centered_rect(60, height, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " 별칭 지정/편집 ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  원본 제목: "),
            Span::styled(data.original_title, Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  > "),
            Span::styled(data.input, Style::default().fg(Color::White)),
            Span::styled("│", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "[Enter] 저장 (빈칸=별칭 삭제)",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("[Esc] 취소", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
    ];

    let para = Paragraph::new(lines).alignment(Alignment::Left);
    f.render_widget(para, inner);
}
