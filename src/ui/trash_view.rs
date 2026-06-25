/// 휴지통 화면 렌더 (FR-11, §2.7)
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::trash::TrashEntry;

use super::time::relative_time;
use crate::ui::list::safe_truncate;

/// 휴지통 화면 렌더
///
/// - entries: 표시할 항목 (삭제 시각 내림차순 정렬됨)
/// - cursor: 현재 커서 위치
/// - selected_ids: 다중선택된 session_id 집합
pub fn render_trash(
    f: &mut Frame,
    entries: &[&TrashEntry],
    cursor: usize,
    selected_ids: &std::collections::HashSet<String>,
) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 헤더
            Constraint::Min(1),    // 테이블
            Constraint::Length(1), // 상태바
        ])
        .split(area);

    // ── 헤더 ──────────────────────────────────────────────────────────────
    let header_line = Line::from(vec![
        Span::styled(
            " Trash ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("({} 항목)", entries.len()),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    f.render_widget(Paragraph::new(header_line), chunks[0]);

    // ── 빈 휴지통 ─────────────────────────────────────────────────────────
    if entries.is_empty() {
        let p = Paragraph::new("휴지통이 비어 있습니다.")
            .block(Block::default().borders(Borders::ALL).title(" Trash "))
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, chunks[1]);
        render_trash_statusbar(f, chunks[2]);
        return;
    }

    // ── 컬럼 폭 반응형 ────────────────────────────────────────────────────
    let term_width = area.width;
    let show_cwd = term_width >= 90;
    // 휴지통 화면에서는 메시지 수 미표시

    // ── 헤더 행 ───────────────────────────────────────────────────────────
    let mut header_cells = vec![
        Cell::from("  "), // 선택 마커
        Cell::from("Title"),
        Cell::from("삭제"),
    ];
    if show_cwd {
        header_cells.push(Cell::from("원본 경로"));
    }
    let table_header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(0);

    // ── 데이터 행 ─────────────────────────────────────────────────────────
    let rows: Vec<Row> = entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_sel_cursor = i == cursor;
            let is_checked = selected_ids.contains(&entry.session_id);

            // 마커: ✓ 다중선택, ▸ 커서
            let marker = match (is_sel_cursor, is_checked) {
                (true, true) => "▸✓",
                (true, false) => "▸ ",
                (false, true) => " ✓",
                (false, false) => "  ",
            };

            let title = safe_truncate(&entry.title, 36);
            let deleted_at = relative_time(&entry.deleted_at());

            let mut cells = vec![
                Cell::from(marker),
                Cell::from(title),
                Cell::from(deleted_at),
            ];

            if show_cwd {
                let cwd = safe_truncate(&entry.cwd, 28);
                cells.push(Cell::from(cwd));
            }

            let style = if is_sel_cursor {
                Style::default().add_modifier(Modifier::REVERSED)
            } else if is_checked {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            Row::new(cells).style(style)
        })
        .collect();

    // ── 컬럼 폭 제약 ──────────────────────────────────────────────────────
    let mut widths = vec![
        Constraint::Length(2),  // 마커
        Constraint::Min(20),    // 제목
        Constraint::Length(12), // 삭제 시각
    ];
    if show_cwd {
        widths.push(Constraint::Length(30)); // 원본 경로
    }

    let title_str = format!(" Trash ({}) ", entries.len());
    let table = Table::new(rows, widths)
        .header(table_header)
        .block(Block::default().borders(Borders::ALL).title(title_str))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("");

    let mut table_state = TableState::default();
    table_state.select(Some(cursor));

    f.render_stateful_widget(table, chunks[1], &mut table_state);

    render_trash_statusbar(f, chunks[2]);
}

fn render_trash_statusbar(f: &mut Frame, area: ratatui::layout::Rect) {
    let spans = vec![Span::styled(
        " Space 선택 · r 복구 · D 영구삭제 · Esc/T 닫기",
        Style::default().fg(Color::DarkGray),
    )];
    let status = Paragraph::new(Line::from(spans));
    f.render_widget(status, area);
}
