use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::service::AppState;

use super::time::relative_time;

pub fn render_list(f: &mut Frame, state: &AppState, selected: usize) {
    let area = f.area();

    // 레이아웃: 헤더 1줄 + 테이블 본문 + 상태바 1줄
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 헤더
            Constraint::Min(1),    // 테이블
            Constraint::Length(1), // 상태바
        ])
        .split(area);

    // 헤더 (타이틀 + RAM은 MVP에서 생략)
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
    ]);
    f.render_widget(Paragraph::new(header_line), chunks[0]);

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
        f.render_widget(p, chunks[1]);

        render_statusbar(f, chunks[2], state, 0);
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

    // 데이터 행
    let rows: Vec<Row> = state
        .sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let is_sel = i == selected;

            // 마커: 선택(▸) + 활성(●)
            let marker = if is_sel && session.is_active {
                "▸●"
            } else if is_sel {
                "▸ "
            } else if session.is_active {
                " ●"
            } else {
                "  "
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

            let style = if is_sel {
                Style::default().add_modifier(Modifier::REVERSED)
            } else if session.is_active {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            Row::new(cells).style(style)
        })
        .collect();

    // 컬럼 폭 제약
    let widths = build_widths(show_project, show_msgs, term_width);

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Sessions ({}) ", state.sessions.len())),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("");

    let mut table_state = TableState::default();
    table_state.select(Some(selected));

    f.render_stateful_widget(table, chunks[1], &mut table_state);

    render_statusbar(f, chunks[2], state, selected);
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

fn build_widths(show_project: bool, show_msgs: bool, term_width: u16) -> Vec<Constraint> {
    let _ = term_width;
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
    _selected: usize,
) {
    let mut spans = vec![];

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
    spans.push(Span::styled(
        " ↑↓/jk 이동  Enter 이어하기  ? 도움말  q 종료",
        Style::default().fg(Color::DarkGray),
    ));

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
