use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};
use std::collections::HashSet;
use unicode_width::UnicodeWidthStr;

use crate::service::AppState;

use super::time::relative_time;

/// 메인 리스트 렌더.
/// - `search_mode`: true이면 검색 입력바 추가 렌더.
/// - `selected_ids`: 다중선택된 session_id 집합 (✓ 마커 표시용).
/// - `status_message`: 작업 결과 임시 메시지 (None이면 기본 키힌트).
pub fn render_list(
    f: &mut Frame,
    state: &AppState,
    cursor: usize,
    search_mode: bool,
    selected_ids: &HashSet<String>,
    status_message: Option<&str>,
) {
    let area = f.area();

    // 레이아웃: 헤더 1줄 + [검색바 1줄 if search_mode] + 테이블 본문 + 상태바 1줄
    let constraints = if search_mode {
        vec![
            Constraint::Length(1), // 헤더
            Constraint::Length(1), // 검색바
            Constraint::Min(1),    // 테이블
            Constraint::Length(1), // 상태바
        ]
    } else {
        vec![
            Constraint::Length(1), // 헤더
            Constraint::Min(1),    // 테이블
            Constraint::Length(1), // 상태바
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    // 청크 인덱스 계산
    let (search_chunk, table_chunk, status_chunk) = if search_mode {
        (Some(chunks[1]), chunks[2], chunks[3])
    } else {
        (None, chunks[1], chunks[2])
    };

    // ── 헤더 ──────────────────────────────────────────────────────────────
    let header_line = Line::from(vec![
        Span::styled(
            " claudeDesk ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled(
            format!("Sort: {}", state.sort.display()),
            Style::default().fg(Color::Yellow),
        ),
        if !selected_ids.is_empty() {
            Span::styled(
                format!("  [{}개 선택]", selected_ids.len()),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw("")
        },
    ]);
    f.render_widget(Paragraph::new(header_line), chunks[0]);

    // ── 검색바 ────────────────────────────────────────────────────────────
    if let Some(chunk) = search_chunk {
        let query = state.search_query.as_deref().unwrap_or("");
        let match_count = state.filtered_indices().len();
        let suffix = format!("({} matches · Esc 취소)", match_count);
        let prefix_width = 2u16; // " /" 폭
        let suffix_width = suffix.chars().count() as u16;
        let bar_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(prefix_width),
                Constraint::Min(0),
                Constraint::Length(suffix_width),
            ])
            .split(chunk);

        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " /",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ))),
            bar_chunks[0],
        );

        let query_line = Line::from(vec![
            Span::styled(query, Style::default().fg(Color::White)),
            Span::styled("│", Style::default().fg(Color::White)),
        ]);
        f.render_widget(Paragraph::new(query_line), bar_chunks[1]);

        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                suffix,
                Style::default().fg(Color::DarkGray),
            ))),
            bar_chunks[2],
        );
    }

    // ── 필터된 인덱스 목록 ────────────────────────────────────────────────
    let indices = state.filtered_indices();

    // 빈 목록 처리
    if state.sessions.is_empty() {
        let empty_msg = if state.projects_root.exists() {
            "세션이 없습니다. Claude Code를 실행해 세션을 생성하세요."
        } else {
            "세션 경로를 찾을 수 없습니다. ~/.claude/projects/ 를 확인하세요."
        };
        let p = Paragraph::new(empty_msg)
            .block(Block::default().borders(Borders::ALL).title(" Sessions "))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(p, table_chunk);

        render_statusbar(
            f,
            status_chunk,
            state,
            0,
            search_mode,
            selected_ids,
            status_message,
        );
        return;
    }

    // 검색 결과 없음
    if indices.is_empty() {
        let p = Paragraph::new("검색 결과 없음")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Sessions (0) "),
            )
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(p, table_chunk);
        render_statusbar(
            f,
            status_chunk,
            state,
            0,
            search_mode,
            selected_ids,
            status_message,
        );
        return;
    }

    // 컬럼 폭 반응형
    let term_width = area.width;
    let (show_project, show_msgs) = if term_width >= 80 {
        (true, true)
    } else if term_width >= 60 {
        (true, false)
    } else {
        (false, false)
    };

    // 헤더 행
    let header_cells = build_header_cells(show_project, show_msgs);
    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(0);

    // 데이터 행 (필터된 인덱스로만)
    let rows: Vec<Row> = indices
        .iter()
        .enumerate()
        .map(|(display_i, &real_i)| {
            let session = &state.sessions[real_i];
            let is_sel_cursor = display_i == cursor;
            let is_checked = selected_ids.contains(&session.session_id);

            // 마커: ▸ 커서, ● 활성, ✓ 다중선택
            let marker = match (is_sel_cursor, is_checked, session.is_active) {
                (true, true, _) => "▸✓",
                (true, false, true) => "▸●",
                (true, false, false) => "▸ ",
                (false, true, _) => " ✓",
                (false, false, true) => " ●",
                (false, false, false) => "  ",
            };

            let title = safe_truncate(&session.title, 40);
            let modified = relative_time(&session.modified);

            let mut cells = vec![Cell::from(marker), Cell::from(title)];

            if show_project {
                let project = safe_truncate(session.project_name(), 20);
                cells.push(Cell::from(project));
            }

            cells.push(Cell::from(modified));

            if show_msgs {
                cells.push(Cell::from(session.msg_count.to_string()));
            }

            let style = if is_sel_cursor {
                Style::default().add_modifier(Modifier::REVERSED)
            } else if is_checked {
                Style::default().fg(Color::Cyan)
            } else if session.is_active {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            Row::new(cells).style(style)
        })
        .collect();

    // 컬럼 폭 제약
    let widths = build_widths(show_project, show_msgs);

    let title_str = if search_mode {
        format!(" Sessions ({}/{}) ", indices.len(), state.sessions.len())
    } else {
        format!(" Sessions ({}) ", state.sessions.len())
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title_str))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("");

    let mut table_state = TableState::default();
    table_state.select(Some(cursor));

    f.render_stateful_widget(table, table_chunk, &mut table_state);

    render_statusbar(
        f,
        status_chunk,
        state,
        cursor,
        search_mode,
        selected_ids,
        status_message,
    );
}

fn build_header_cells(show_project: bool, show_msgs: bool) -> Vec<Cell<'static>> {
    let mut cells = vec![Cell::from("  "), Cell::from("Title")];
    if show_project {
        cells.push(Cell::from("Project"));
    }
    cells.push(Cell::from("Modified"));
    if show_msgs {
        cells.push(Cell::from("Msgs"));
    }
    cells
}

fn build_widths(show_project: bool, show_msgs: bool) -> Vec<Constraint> {
    let mut constraints = vec![
        Constraint::Length(2), // 마커
        Constraint::Min(20),   // 제목
    ];
    if show_project {
        constraints.push(Constraint::Length(22)); // 프로젝트
    }
    constraints.push(Constraint::Length(10)); // 수정시각
    if show_msgs {
        constraints.push(Constraint::Length(5)); // 메시지 수
    }
    constraints
}

fn render_statusbar(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    state: &AppState,
    _cursor: usize,
    search_mode: bool,
    selected_ids: &HashSet<String>,
    status_message: Option<&str>,
) {
    let mut spans = vec![];

    // 임시 상태 메시지가 있으면 우선 표시
    if let Some(msg) = status_message {
        spans.push(Span::styled(
            format!(" {} ", msg),
            Style::default().fg(Color::Green),
        ));
        let status = Paragraph::new(Line::from(spans));
        f.render_widget(status, area);
        return;
    }

    // 스킵 카운트 (FR-12)
    if state.stats.skipped_lines > 0 || state.stats.skipped_files > 0 {
        spans.push(Span::styled(
            format!(
                " ! Skipped: {}줄 {}파일 ",
                state.stats.skipped_lines, state.stats.skipped_files
            ),
            Style::default().fg(Color::Red),
        ));
        spans.push(Span::raw("| "));
    }

    // 세션 수
    spans.push(Span::styled(
        format!(" {}개 세션 ", state.sessions.len()),
        Style::default().fg(Color::DarkGray),
    ));
    spans.push(Span::raw("| "));

    // 키 힌트
    if search_mode {
        spans.push(Span::styled(
            " ↑↓ 이동  Enter 이어하기  Esc 검색 취소",
            Style::default().fg(Color::DarkGray),
        ));
    } else if !selected_ids.is_empty() {
        // 다중선택 활성 시 선택 관련 키힌트
        spans.push(Span::styled(
            format!(
                " {}개 선택됨  Space 선택토글  a 전체선택/해제  Del 삭제  Esc 선택 해제",
                selected_ids.len()
            ),
            Style::default().fg(Color::Cyan),
        ));
    } else {
        spans.push(Span::styled(
            " ↑↓/jk 이동  Enter 이어하기  / 검색  s 정렬  Space 선택  a 전체선택  Del 삭제  T 휴지통  ? 도움말  q 종료",
            Style::default().fg(Color::DarkGray),
        ));
    }

    let status = Paragraph::new(Line::from(spans));
    f.render_widget(status, area);
}

/// unicode-width 기반 안전 말줄임. 전체가 들어가면 그대로, 넘치면 말줄임표(…) 자리를 확보해 자른다.
pub fn safe_truncate(s: &str, max_width: usize) -> String {
    if UnicodeWidthStr::width(s) <= max_width {
        return s.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    // 말줄임표 폭(1)을 뺀 예산만큼 채운다
    let budget = max_width - 1;
    let mut width = 0usize;
    let mut result = String::new();
    for c in s.chars() {
        let cw = UnicodeWidthStr::width(c.encode_utf8(&mut [0u8; 4]));
        if width + cw > budget {
            break;
        }
        width += cw;
        result.push(c);
    }
    result.push('…');
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_truncate_ascii() {
        let s = "hello world";
        assert_eq!(safe_truncate(s, 5), "hell…");
    }

    #[test]
    fn test_safe_truncate_korean() {
        // 한글은 폭 2
        let s = "안녕하세요";
        let t = safe_truncate(s, 6);
        // "안녕" = 4, "하" = 2 → 총 6, 다음 "세" 추가 시 8 > 6
        assert!(UnicodeWidthStr::width(t.as_str()) <= 7); // 말줄임 포함
    }

    #[test]
    fn test_safe_truncate_no_truncate() {
        let s = "short";
        assert_eq!(safe_truncate(s, 20), "short");
    }
}
