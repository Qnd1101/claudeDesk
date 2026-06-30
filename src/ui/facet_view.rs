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
    status_message: Option<&str>,
) {
    let width = area.width;

    // <90: 단일 패널(목록 전체 폭). 그 외엔 2-pane이며 좌측 비율만 폭에 따라 달라진다.
    if width < 90 {
        render_left(
            f,
            area,
            state,
            cursor,
            search_mode,
            color_enabled,
            time_format,
            status_message,
        );
        return;
    }

    let left_pct = if width < 120 { 30 } else { 25 };
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_pct),
            Constraint::Percentage(100 - left_pct),
        ])
        .split(area);
    render_left(
        f,
        chunks[0],
        state,
        cursor,
        search_mode,
        color_enabled,
        time_format,
        status_message,
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

/// 좌측 패널 렌더 (facet 탭바 + 세션 목록 + 상태바)
#[allow(clippy::too_many_arguments)]
fn render_left(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    cursor: usize,
    search_mode: bool,
    color_enabled: bool,
    time_format: TimeFormat,
    status_message: Option<&str>,
) {
    if area.height < 5 {
        return; // 최소 높이 확인
    }

    let constraints: Vec<Constraint> = if search_mode {
        vec![
            Constraint::Length(2), // facet 탭바 (탭 + 정보줄)
            Constraint::Length(1), // 검색바
            Constraint::Min(1),    // 세션 목록
            Constraint::Length(1), // 상태바
        ]
    } else {
        vec![
            Constraint::Length(2), // facet 탭바 (탭 + 정보줄)
            Constraint::Min(1),    // 세션 목록
            Constraint::Length(1), // 상태바
        ]
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let facet_area = layout[0];
    let (search_area_opt, list_area, status_area) = if search_mode {
        (Some(layout[1]), layout[2], layout[3])
    } else {
        (None, layout[1], layout[2])
    };

    // 상단 facet 탭바 (탭 + Sort 정보)
    render_facet_tabs(f, facet_area, state, color_enabled);

    // 검색바 (search_mode 진입 시) — 3구역: [/] [쿼리│] [N건 · Esc 취소]
    if let Some(search_area) = search_area_opt {
        let query = state.search_query.as_deref().unwrap_or("");
        let match_count = facet::facet_indices(state).len();
        let suffix = format!("({}건 · Esc 취소)", match_count);
        let suffix_width = suffix.chars().count() as u16;

        let bar_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(suffix_width),
            ])
            .split(search_area);

        f.render_widget(
            Paragraph::new(Span::styled("/", cond_fg_bold(color_enabled, Color::Green))),
            bar_chunks[0],
        );
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(query.to_string()),
                Span::styled("│", Style::default()),
            ])),
            bar_chunks[1],
        );
        f.render_widget(
            Paragraph::new(Span::styled(
                suffix,
                cond_fg(color_enabled, Color::DarkGray),
            )),
            bar_chunks[2],
        );
    }

    // 세션 목록
    let active_query = if search_mode {
        state.search_query.as_deref().unwrap_or("")
    } else {
        ""
    };
    render_session_list(
        f,
        list_area,
        state,
        cursor,
        color_enabled,
        time_format,
        active_query,
    );

    // 하단 상태바
    render_left_statusbar(
        f,
        status_area,
        state,
        search_mode,
        status_message,
        color_enabled,
    );
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
    let facet_indices = facet::facet_indices(state);

    if facet_indices.is_empty() || cursor >= facet_indices.len() {
        let msg = Paragraph::new("세션을 선택하세요")
            .block(Block::default().borders(Borders::ALL).title(" Preview "))
            .style(cond_fg(color_enabled, Color::Yellow));
        f.render_widget(msg, area);
        return;
    }

    let session_idx = facet_indices[cursor];
    let session = &state.sessions[session_idx];

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

    let header_para = Paragraph::new(header_text).style(cond_fg_bold(color_enabled, Color::Cyan));
    f.render_widget(header_para, layout[0]);

    // preview가 열렸고 콘텐츠가 준비됐을 때만 스트리밍 미리보기, 아니면 세션 요약 fallback.
    match (preview_open, preview_content) {
        (true, Some(content)) => {
            render_preview(f, layout[1], content, preview_title, preview_path);
        }
        _ => {
            let msg = Paragraph::new(format!("CWD: {}", session.cwd))
                .block(Block::default().borders(Borders::ALL).title(" Session "))
                .style(cond_fg(color_enabled, Color::DarkGray));
            f.render_widget(msg, layout[1]);
        }
    }
}

/// facet 탭바 렌더 — 줄 1: 탭, 줄 2: Sort·세션 수 정보
fn render_facet_tabs(f: &mut Frame, area: Rect, state: &AppState, color_enabled: bool) {
    let counts = facet::counts(state);

    // 줄 1: facet 탭
    let mut spans = Vec::new();
    spans.push(Span::raw("  "));
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

    // 줄 2: Sort 상태 + 세션 수
    let info_line = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!(
                "Sort: {}  |  {}세션",
                state.sort.display(),
                state.sessions.len()
            ),
            cond_fg(color_enabled, Color::DarkGray),
        ),
    ]);

    let para = Paragraph::new(vec![Line::from(spans), info_line]);
    f.render_widget(para, area);
}

/// 좌측 패널 하단 상태바 렌더 — 키 힌트 또는 임시 메시지
fn render_left_statusbar(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    search_mode: bool,
    status_message: Option<&str>,
    color_enabled: bool,
) {
    let line = if let Some(msg) = status_message {
        Line::from(Span::styled(
            format!(" {msg} "),
            cond_fg(color_enabled, Color::Green),
        ))
    } else if search_mode {
        Line::from(Span::styled(
            " ↑↓/jk 이동  Enter 이어하기  Esc 취소",
            cond_fg(color_enabled, Color::DarkGray),
        ))
    } else {
        let hint = if state.grouped {
            " Enter 이어하기  / 검색  s 정렬  g 그룹  Tab 접기/펼치기  Del 삭제  ? 도움말  q 종료"
        } else {
            " Enter 이어하기  / 검색  s 정렬  g 그룹  Del 삭제  T 휴지통  ? 도움말  q 종료"
        };
        Line::from(Span::styled(hint, cond_fg(color_enabled, Color::DarkGray)))
    };
    f.render_widget(Paragraph::new(line), area);
}

/// 세션 목록 렌더
fn render_session_list(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    cursor: usize,
    color_enabled: bool,
    time_format: TimeFormat,
    search_query: &str,
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

            // health 아이콘 + 색상 (NO_COLOR/mono 시 emoji→ASCII fallback)
            let (icon, health_style) = match session.health {
                Health::Active => ("●", cond_fg(color_enabled, Color::Green)),
                Health::Empty => ("○", cond_fg(color_enabled, Color::DarkGray)),
                Health::Stale => (
                    if color_enabled { "⏰" } else { "~" },
                    cond_fg(color_enabled, Color::Yellow),
                ),
                Health::Zombie => (
                    if color_enabled { "💀" } else { "!" },
                    cond_fg(color_enabled, Color::Red),
                ),
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

            let title_cell = if search_query.is_empty() {
                Cell::from(title_text)
            } else {
                Cell::from(highlight_query(&title_text, search_query))
            };

            let cells = vec![Cell::from(marker_text), title_cell, Cell::from(time_text)];

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

/// 검색 쿼리와 일치하는 부분에 밑줄 강조를 적용한 Line 반환 (색 무관, UNDERLINED).
/// 일치 없으면 plain text Line 반환.
fn highlight_query(text: &str, query: &str) -> Line<'static> {
    if query.is_empty() {
        return Line::from(text.to_string());
    }
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();
    if let Some(byte_pos) = lower_text.find(lower_query.as_str()) {
        let end_pos = byte_pos + query.len();
        if text.is_char_boundary(byte_pos) && text.is_char_boundary(end_pos) {
            let before = text[..byte_pos].to_string();
            let matched = text[byte_pos..end_pos].to_string();
            let after = text[end_pos..].to_string();
            return Line::from(vec![
                Span::raw(before),
                Span::styled(matched, Style::default().add_modifier(Modifier::UNDERLINED)),
                Span::raw(after),
            ]);
        }
    }
    Line::from(text.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_empty_query_returns_single_span() {
        let line = highlight_query("hello world", "");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content, "hello world");
    }

    #[test]
    fn highlight_no_match_returns_single_span() {
        let line = highlight_query("hello world", "xyz");
        assert_eq!(line.spans.len(), 1);
    }

    #[test]
    fn highlight_match_splits_three_spans() {
        let line = highlight_query("hello world", "world");
        assert_eq!(line.spans.len(), 3);
        assert_eq!(line.spans[0].content, "hello ");
        assert_eq!(line.spans[1].content, "world");
        assert_eq!(line.spans[2].content, "");
        // 밑줄 스타일 확인
        assert!(line.spans[1]
            .style
            .add_modifier
            .contains(Modifier::UNDERLINED));
    }

    #[test]
    fn highlight_case_insensitive_preserves_original_case() {
        let line = highlight_query("Hello World", "hello");
        assert_eq!(line.spans.len(), 3);
        assert_eq!(line.spans[1].content, "Hello"); // 원본 대소문자 유지
    }

    #[test]
    fn highlight_match_at_start() {
        let line = highlight_query("docker build", "docker");
        assert_eq!(line.spans[0].content, ""); // before = empty
        assert_eq!(line.spans[1].content, "docker");
    }

    #[test]
    fn highlight_match_at_end() {
        let line = highlight_query("cargo build", "build");
        assert_eq!(line.spans[0].content, "cargo ");
        assert_eq!(line.spans[1].content, "build");
        assert_eq!(line.spans[2].content, "");
    }
}
