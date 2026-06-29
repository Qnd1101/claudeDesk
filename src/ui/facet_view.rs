//! FR-15 facet 2-pane 뷰 렌더
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::config::TimeFormat;
use crate::facet::{self, Facet};
use crate::health::Health;
use crate::preview::PreviewContent;
use crate::service::AppState;

use super::preview::render_preview;
use super::theme::{cond_fg, cond_fg_bold};
use super::time::format_time;

/// 2-pane facet 뷰 전체 렌더 (Normal 모드 진입점)
#[allow(clippy::too_many_arguments)]
pub fn render(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    cursor: usize,
    search_mode: bool,
    preview_open: bool,
    preview_content: Option<&PreviewContent>,
    preview_title: &str,
    preview_path: &str,
    color_enabled: bool,
    time_format: TimeFormat,
) {
    let width = area.width;

    if width < 90 {
        // single-pane: 좌측 목록만 전체 폭
        render_left(
            f,
            area,
            state,
            cursor,
            search_mode,
            color_enabled,
            time_format,
        );
    } else if width < 120 {
        // narrow 2-pane: 좌측 30% / 우측 70%
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);
        render_left(
            f,
            chunks[0],
            state,
            cursor,
            search_mode,
            color_enabled,
            time_format,
        );
        render_right(
            f,
            chunks[1],
            state,
            cursor,
            preview_open,
            preview_content,
            preview_title,
            preview_path,
            color_enabled,
        );
    } else {
        // full 2-pane: 좌측 25% / 우측 75%
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(area);
        render_left(
            f,
            chunks[0],
            state,
            cursor,
            search_mode,
            color_enabled,
            time_format,
        );
        render_right(
            f,
            chunks[1],
            state,
            cursor,
            preview_open,
            preview_content,
            preview_title,
            preview_path,
            color_enabled,
        );
    }
}

/// 좌측 패널 렌더 (facet 탭바 + 세션 목록)
fn render_left(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    cursor: usize,
    _search_mode: bool,
    color_enabled: bool,
    time_format: TimeFormat,
) {
    if area.height < 4 {
        return; // 최소 높이 확인
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // facet 탭바
            Constraint::Min(1),    // 세션 목록
        ])
        .split(area);

    let facet_area = layout[0];
    let list_area = layout[1];

    // 상단 facet 탭바
    render_facet_tabs(f, facet_area, state, color_enabled);

    // 하단 세션 목록
    render_session_list(f, list_area, state, cursor, color_enabled, time_format);
}

/// 우측 패널 렌더 (preview)
#[allow(clippy::too_many_arguments)]
fn render_right(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    cursor: usize,
    preview_open: bool,
    preview_content: Option<&PreviewContent>,
    preview_title: &str,
    preview_path: &str,
    color_enabled: bool,
) {
    // 현재 cursor에 해당하는 session_id 찾기
    let facet_indices = facet::facet_indices(state);

    if facet_indices.is_empty() || cursor >= facet_indices.len() {
        // 세션 없음
        let msg = Paragraph::new("세션을 선택하세요")
            .block(Block::default().borders(Borders::ALL).title(" Preview "))
            .style(cond_fg(color_enabled, Color::Yellow));
        f.render_widget(msg, area);
        return;
    }

    let session_idx = facet_indices[cursor];
    let session = &state.sessions[session_idx];

    // 헤더: session_id, cwd, msg_count, health
    let header_text = format!(
        "ID: {}  |  MSG: {}  |  Health: {}",
        &session.session_id[..session.session_id.len().min(8)],
        session.msg_count,
        session.health.label()
    );

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 헤더
            Constraint::Min(1),    // preview
        ])
        .split(area);

    // 헤더 렌더
    let header_para = Paragraph::new(header_text).style(cond_fg_bold(color_enabled, Color::Cyan));
    f.render_widget(header_para, layout[0]);

    // preview 렌더
    if preview_open {
        if let Some(content) = preview_content {
            render_preview(f, layout[1], content, preview_title, preview_path);
        } else {
            let msg = Paragraph::new(format!("CWD: {}", session.cwd))
                .block(Block::default().borders(Borders::ALL).title(" Session "))
                .style(cond_fg(color_enabled, Color::DarkGray));
            f.render_widget(msg, layout[1]);
        }
    } else {
        let msg = Paragraph::new(format!("CWD: {}", session.cwd))
            .block(Block::default().borders(Borders::ALL).title(" Session "))
            .style(cond_fg(color_enabled, Color::DarkGray));
        f.render_widget(msg, layout[1]);
    }
}

/// facet 탭바 렌더
fn render_facet_tabs(f: &mut Frame, area: Rect, state: &AppState, color_enabled: bool) {
    let counts = facet::counts(state);

    let mut spans = Vec::new();
    spans.push(Span::raw("  ")); // 좌측 여백

    for (idx, facet) in Facet::all().iter().enumerate() {
        let is_current = state.facet == *facet;
        let count = counts[idx];
        let digit = facet.to_digit();
        let label = format!("[{}:{}({})]", digit, facet.label(), count);

        let span_style = if is_current {
            cond_fg_bold(color_enabled, Color::Cyan)
        } else {
            Style::default()
        };

        spans.push(Span::styled(label, span_style));
        spans.push(Span::raw("  "));
    }

    let line = Line::from(spans);
    let para = Paragraph::new(line)
        .block(Block::default().borders(Borders::BOTTOM))
        .style(Style::default());
    f.render_widget(para, area);
}

/// 세션 목록 렌더
fn render_session_list(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    cursor: usize,
    color_enabled: bool,
    time_format: TimeFormat,
) {
    // facet 필터된 indices
    let facet_indices = facet::facet_indices(state);

    // 빈 목록 처리
    if facet_indices.is_empty() {
        let p = Paragraph::new("이 facet에서 세션이 없습니다")
            .block(Block::default().borders(Borders::ALL))
            .style(cond_fg(color_enabled, Color::Yellow));
        f.render_widget(p, area);
        return;
    }

    // 헤더 행
    let header_cells = vec![Cell::from(" "), Cell::from("Title"), Cell::from("Time")];
    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(0);

    // 데이터 행
    let rows: Vec<Row> = facet_indices
        .iter()
        .enumerate()
        .map(|(display_i, &real_idx)| {
            let session = &state.sessions[real_idx];
            let is_cursor = display_i == cursor;

            // health 아이콘 + 색상
            let (icon, health_style) = match session.health {
                Health::Active => ("●", cond_fg(color_enabled, Color::Green)),
                Health::Empty => ("○", cond_fg(color_enabled, Color::DarkGray)),
                Health::Stale => ("⏰", cond_fg(color_enabled, Color::Yellow)),
                Health::Zombie => ("💀", cond_fg(color_enabled, Color::Red)),
            };

            let marker = if is_cursor { ">" } else { " " };

            // marker(>) 또는 공백 + health 아이콘(색상 적용)
            let marker_span = Span::raw(marker);
            let icon_span = Span::styled(icon, health_style);
            let marker_text = Line::from(vec![marker_span, icon_span]);

            // 커서 행은 REVERSED 스타일 적용
            let style_override = if is_cursor {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            let title_text = safe_truncate(session.display_title(), 30);
            let time_text = format_time(&session.modified, time_format);

            let cells = vec![
                Cell::from(marker_text),
                Cell::from(title_text),
                Cell::from(time_text),
            ];

            Row::new(cells).style(style_override)
        })
        .collect();

    let widths = [
        Constraint::Length(2),
        Constraint::Min(10),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(format!(
            " {} ({}) ",
            state.facet.label(),
            facet_indices.len()
        )))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("");

    let mut table_state = TableState::default();
    table_state.select(Some(cursor));

    f.render_stateful_widget(table, area, &mut table_state);
}

/// 문자열을 지정된 폭으로 안전하게 자름 (유니코드 인식)
fn safe_truncate(s: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthStr;

    if s.width() <= max_width {
        return s.to_string();
    }

    let mut result = String::new();
    let mut width = 0;

    for c in s.chars() {
        let c_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
        if width + c_width > max_width {
            break;
        }
        result.push(c);
        width += c_width;
    }

    result
}
