/// 삭제 확인 모달 (FR-04, §2.4) + purge 2단계 확인 모달 (FR-11) + 별칭 편집 모달 (FR-06)
/// + 오래된 세션 선택 모달 (FR-14)
///
/// T11.3: 모든 모달이 `color_enabled` 필드를 통해 Mono/NO_COLOR를 지원한다.
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::layout::centered_rect;
use super::theme::Palette;

// ── 소프트 삭제 확인 모달 (§2.4) ────────────────────────────────────────────

/// 삭제 확인 모달에 전달할 데이터
pub struct DeleteConfirmData<'a> {
    /// 삭제할 세션 목록 (제목)
    pub titles: &'a [String],
    /// 활성 세션이라 스킵될 수 (표시용)
    pub active_count: usize,
    /// T11.3 색상 팔레트
    pub palette: Palette,
}

/// 소프트 삭제 확인 모달 렌더 (FR-04 §2.4)
/// Enter = 확인, Esc = 취소
pub fn render_delete_confirm(f: &mut Frame, data: &DeleteConfirmData<'_>) {
    let palette = data.palette;
    let list_lines = data.titles.len().min(5) as u16;
    let height = 9 + list_lines;
    let area = centered_rect(58, height, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " 삭제 확인 ",
            palette.fg_bold(palette.warning),
        ))
        .border_style(palette.fg(palette.warning));

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
            Span::raw(title.as_str()),
        ]));
    }
    if data.titles.len() > 5 {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("... 외 {}개", data.titles.len() - 5),
                palette.fg(palette.muted),
            ),
        ]));
    }

    if data.active_count > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("⚠ 활성 세션 {}개는 제외됩니다(●).", data.active_count),
                palette.fg(palette.danger),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("      "),
        Span::styled("[Enter] 휴지통 이동", palette.fg_bold(palette.active)),
        Span::raw("    "),
        Span::styled("[Esc] 취소", palette.fg(palette.muted)),
    ]));
    lines.push(Line::from(""));

    let para = Paragraph::new(lines).alignment(Alignment::Left);
    f.render_widget(para, inner);
}

// ── 오래된 세션 선택 모달 (FR-14) ───────────────────────────────────────────

/// 오래된 세션 선택 모달에 전달할 데이터
pub struct AgeSelectData<'a> {
    /// (기준 일수, 해당 기준 이전 대상 세션 수) 목록
    pub options: &'a [(u64, usize)],
    /// 현재 커서가 가리키는 옵션 인덱스
    pub cursor: usize,
    /// T11.3 색상 팔레트
    pub palette: Palette,
}

/// 오래된 세션 선택 모달 렌더 (FR-14)
/// ↑↓ = 기준 선택, Enter = 해당 기준 이전 세션을 다중선택에 추가(삭제 아님), Esc = 취소
pub fn render_age_select(f: &mut Frame, data: &AgeSelectData<'_>) {
    let palette = data.palette;
    let height = 9 + data.options.len() as u16;
    let area = centered_rect(56, height, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " 오래된 세션 선택 ",
            palette.fg_bold(palette.warning),
        ))
        .border_style(palette.fg(palette.warning));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::raw("기준일 이전에 수정된 세션을 한 번에 선택합니다."),
        ]),
        Line::from(""),
    ];

    for (i, (days, count)) in data.options.iter().enumerate() {
        let is_cur = i == data.cursor;
        let marker = if is_cur { "▸ " } else { "  " };
        let label_style = if is_cur {
            palette.fg_bold(palette.warning)
        } else {
            Style::default()
        };
        // 일수 우측정렬(3자리)로 열 정렬을 맞춘다.
        let label = format!("{:>3}일 이전", days);
        let count_style = if *count == 0 {
            palette.fg(palette.muted)
        } else {
            palette.fg(palette.accent)
        };
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(marker, palette.fg(palette.warning)),
            Span::styled(label, label_style),
            Span::raw("   "),
            Span::styled(format!("{}개", count), count_style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("선택 후 ", palette.fg(palette.muted)),
        Span::styled("d", palette.fg_bold(palette.danger)),
        Span::styled(" 로 삭제 확인 — 자동 삭제 아님", palette.fg(palette.muted)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled("[↑↓] 기준 선택", palette.fg(palette.warning)),
        Span::raw("  "),
        Span::styled("[Enter] 선택 적용", palette.fg_bold(palette.active)),
        Span::raw("  "),
        Span::styled("[Esc] 취소", palette.fg(palette.muted)),
    ]));

    let para = Paragraph::new(lines).alignment(Alignment::Left);
    f.render_widget(para, inner);
}

// ── purge 2단계 확인 모달 ────────────────────────────────────────────────────

/// purge 확인 모달에 전달할 데이터
pub struct PurgeConfirmData<'a> {
    pub titles: &'a [String],
    /// 사용자가 입력 중인 확인 문자열 ("DELETE" 타이핑)
    pub input: &'a str,
    /// T11.3 색상 팔레트
    pub palette: Palette,
}

/// purge 확인 모달 렌더 (FR-11 §2.7 영구삭제 2단계)
/// "DELETE" 타이핑 후 Enter = 확인, Esc = 취소
pub fn render_purge_confirm(f: &mut Frame, data: &PurgeConfirmData<'_>) {
    let palette = data.palette;
    let list_lines = data.titles.len().min(5) as u16;
    let height = 13 + list_lines;
    let area = centered_rect(60, height, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " 영구삭제 확인 ",
            palette.fg_bold(palette.danger),
        ))
        .border_style(palette.fg(palette.danger));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "⚠ 영구삭제는 복구할 수 없습니다!",
                palette.fg_bold(palette.danger),
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
            Span::raw(title.as_str()),
        ]));
    }
    if data.titles.len() > 5 {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("... 외 {}개", data.titles.len() - 5),
                palette.fg(palette.muted),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "확인하려면 DELETE 를 입력하세요:",
            palette.fg(palette.warning),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  > "),
        Span::raw(data.input),
        Span::raw("│"),
    ]));
    lines.push(Line::from(""));

    let ready = data.input == "DELETE";
    lines.push(Line::from(vec![
        Span::raw("      "),
        if ready {
            Span::styled("[Enter] 영구삭제", palette.fg_bold(palette.danger))
        } else {
            Span::styled(
                "[Enter] 영구삭제 (DELETE 입력 후)",
                palette.fg(palette.muted),
            )
        },
        Span::raw("    "),
        Span::styled("[Esc] 취소", palette.fg(palette.muted)),
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
    /// T11.3 색상 팔레트
    pub palette: Palette,
}

/// 별칭 지정/편집 모달 렌더 (FR-06 §3.6)
/// Enter = 저장 (빈칸 = 별칭 삭제), Esc = 취소
pub fn render_alias_edit(f: &mut Frame, data: &AliasEditData<'_>) {
    let palette = data.palette;
    let height = 9u16;
    let area = centered_rect(60, height, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " 별칭 지정/편집 ",
            palette.fg_bold(palette.accent),
        ))
        .border_style(palette.fg(palette.accent));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  원본 제목: "),
            Span::styled(data.original_title, palette.fg(palette.muted)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  > "),
            Span::raw(data.input),
            Span::raw("│"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "[Enter] 저장 (빈칸=별칭 삭제)",
                palette.fg_bold(palette.active),
            ),
            Span::raw("  "),
            Span::styled("[Esc] 취소", palette.fg(palette.muted)),
        ]),
        Line::from(""),
    ];

    let para = Paragraph::new(lines).alignment(Alignment::Left);
    f.render_widget(para, inner);
}
