use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};
use std::collections::HashSet;
use unicode_width::UnicodeWidthStr;

use crate::config::TimeFormat;
use crate::preview::PreviewContent;
use crate::service::{AppState, DisplayRow};

use super::preview::render_preview;
use super::theme::{cond_fg, cond_fg_bold};
use super::time::format_time;

/// 미리보기 패널을 활성화하기 위한 최소 터미널 폭(칸)
pub const PREVIEW_MIN_WIDTH: u16 = 100;

/// 메인 리스트 렌더.
/// - `search_mode`: true이면 검색 입력바 추가 렌더.
/// - `selected_ids`: 다중선택된 session_id 집합 (✓ 마커 표시용).
/// - `status_message`: 작업 결과 임시 메시지 (None이면 기본 키힌트).
/// - `preview_open`: 미리보기 패널 열림 여부.
/// - `preview_content`: 미리보기 내용 (None이면 미리보기 미렌더).
/// - `preview_title`: 미리보기 패널 타이틀에 쓸 세션 제목.
/// - `color_enabled`: T11.3 색상 활성 여부 (false=Mono/NO_COLOR).
/// - `time_format`: T11.2 시간 표시 형식 (Relative=상대시간/Absolute=절대시간).
#[allow(clippy::too_many_arguments)]
pub fn render_list(
    f: &mut Frame,
    state: &AppState,
    cursor: usize,
    search_mode: bool,
    selected_ids: &HashSet<String>,
    status_message: Option<&str>,
    preview_open: bool,
    preview_content: Option<&PreviewContent>,
    preview_title: &str,
    preview_path: &str,
    color_enabled: bool,
    time_format: TimeFormat,
) {
    let full_area = f.area();

    // ── FR-08: 가로 분할 (폭 가드) ───────────────────────────────────────
    // preview_open이고 실제 폭이 PREVIEW_MIN_WIDTH 이상일 때만 분할.
    // 한번 열린 뒤 터미널이 좁아져도 패닉 없이 리스트만 렌더.
    let (list_area, preview_area_opt) =
        if preview_open && full_area.width >= PREVIEW_MIN_WIDTH && preview_content.is_some() {
            // 60% 리스트 / 40% 미리보기
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(full_area);
            (chunks[0], Some(chunks[1]))
        } else {
            (full_area, None)
        };

    let area = list_area;

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
        Span::styled(" claudeDesk ", cond_fg_bold(color_enabled, Color::Cyan)),
        Span::styled(
            format!("v{}", env!("CARGO_PKG_VERSION")),
            cond_fg(color_enabled, Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled(
            format!("Sort: {}", state.sort.display()),
            cond_fg(color_enabled, Color::Yellow),
        ),
        if !selected_ids.is_empty() {
            Span::styled(
                format!("  [{}개 선택]", selected_ids.len()),
                cond_fg_bold(color_enabled, Color::Cyan),
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
                cond_fg_bold(color_enabled, Color::Green),
            ))),
            bar_chunks[0],
        );

        let query_line = Line::from(vec![
            Span::styled(query, Style::default()),
            Span::styled("│", Style::default()),
        ]);
        f.render_widget(Paragraph::new(query_line), bar_chunks[1]);

        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                suffix,
                cond_fg(color_enabled, Color::DarkGray),
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
            .style(cond_fg(color_enabled, Color::Yellow));
        f.render_widget(p, table_chunk);

        render_statusbar(
            f,
            status_chunk,
            state,
            0,
            search_mode,
            selected_ids,
            status_message,
            preview_open && preview_area_opt.is_some(),
            "",
            color_enabled,
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
            .style(cond_fg(color_enabled, Color::Yellow));
        f.render_widget(p, table_chunk);
        render_statusbar(
            f,
            status_chunk,
            state,
            0,
            search_mode,
            selected_ids,
            status_message,
            preview_open && preview_area_opt.is_some(),
            "",
            color_enabled,
        );
        return;
    }

    // 컬럼 폭 반응형
    let term_width = area.width;
    let (show_project, show_msgs) = if state.grouped {
        (false, term_width >= 80)
    } else if term_width >= 80 {
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

    // 데이터 행 (display_rows 기반)
    let display_rows = state.display_rows();

    let rows: Vec<Row> = display_rows
        .iter()
        .enumerate()
        .map(|(display_i, row)| {
            let is_cursor = display_i == cursor;
            match row {
                DisplayRow::Header {
                    project_name,
                    count,
                    collapsed,
                    ..
                } => {
                    let arrow = if *collapsed { "▸" } else { "▾" };
                    let title_text = format!("{} {}  ({})", arrow, project_name, count);
                    let style = if is_cursor {
                        cond_fg_bold(color_enabled, Color::Cyan).add_modifier(Modifier::REVERSED)
                    } else {
                        cond_fg_bold(color_enabled, Color::Cyan)
                    };
                    let mut cells = vec![Cell::from(""), Cell::from(title_text)];
                    if show_project {
                        cells.push(Cell::from(""));
                    }
                    cells.push(Cell::from(""));
                    if show_msgs {
                        cells.push(Cell::from(""));
                    }
                    Row::new(cells).style(style)
                }
                DisplayRow::Session(real_i) => {
                    let session = &state.sessions[*real_i];
                    let is_checked = selected_ids.contains(&session.session_id);

                    let marker = match (is_cursor, is_checked, session.is_active) {
                        (true, true, _) => "▸✓",
                        (true, false, true) => "▸●",
                        (true, false, false) => "▸ ",
                        (false, true, _) => " ✓",
                        (false, false, true) => " ●",
                        (false, false, false) => "  ",
                    };

                    // FR-06: display_title() 우선 + 별칭 마커 (§5.7: 텍스트 마커 필수)
                    let display = session.display_title();
                    let has_alias = session.alias.is_some();
                    let alias_marker = if has_alias { "~ " } else { "" };
                    let title_text = if state.grouped {
                        let trunc_w = if has_alias { 36 } else { 38 };
                        format!("  {}{}", alias_marker, safe_truncate(display, trunc_w))
                    } else {
                        let trunc_w = if has_alias { 38 } else { 40 };
                        format!("{}{}", alias_marker, safe_truncate(display, trunc_w))
                    };

                    let modified = format_time(&session.modified, time_format);

                    let mut cells = vec![Cell::from(marker), Cell::from(title_text)];

                    if show_project {
                        let project = safe_truncate(session.project_name(), 20);
                        cells.push(Cell::from(project));
                    }

                    cells.push(Cell::from(modified));

                    if show_msgs {
                        cells.push(Cell::from(session.msg_count.to_string()));
                    }

                    let style = if is_cursor {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else if is_checked {
                        // ✓ 마커가 있으므로 색 없이도 선택 상태 식별 가능 (§5.7)
                        cond_fg(color_enabled, Color::Cyan)
                    } else if session.is_active {
                        // ● 마커가 있으므로 색 없이도 활성 상태 식별 가능 (§5.7)
                        cond_fg(color_enabled, Color::Green)
                    } else {
                        Style::default()
                    };

                    Row::new(cells).style(style)
                }
            }
        })
        .collect();

    // 컬럼 폭 제약
    let widths = build_widths(show_project, show_msgs);

    let session_count = indices.len();
    let title_str = if search_mode {
        format!(" Sessions ({}/{}) ", session_count, state.sessions.len())
    } else if state.grouped {
        format!(" Sessions ({}) [그룹] ", session_count)
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
        preview_open && preview_area_opt.is_some(),
        preview_path,
        color_enabled,
    );

    // ── FR-08: 미리보기 패널 렌더 ────────────────────────────────────────
    if let (Some(preview_area), Some(content)) = (preview_area_opt, preview_content) {
        render_preview(f, preview_area, content, preview_title, preview_path);
    }
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

#[allow(clippy::too_many_arguments)]
fn render_statusbar(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    state: &AppState,
    _cursor: usize,
    search_mode: bool,
    selected_ids: &HashSet<String>,
    status_message: Option<&str>,
    preview_active: bool,
    cursor_path: &str,
    color_enabled: bool,
) {
    let mut spans = vec![];

    // 임시 상태 메시지가 있으면 우선 표시
    if let Some(msg) = status_message {
        spans.push(Span::styled(
            format!(" {} ", msg),
            cond_fg(color_enabled, Color::Green),
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
            cond_fg(color_enabled, Color::Red),
        ));
        spans.push(Span::raw("| "));
    }

    // 세션 수
    spans.push(Span::styled(
        format!(" {}개 세션 ", state.sessions.len()),
        cond_fg(color_enabled, Color::DarkGray),
    ));
    spans.push(Span::raw("| "));

    // 미리보기 활성 표시
    if preview_active {
        spans.push(Span::styled(
            "[미리보기] ",
            cond_fg(color_enabled, Color::Cyan),
        ));
    }

    // 키 힌트
    if search_mode {
        spans.push(Span::styled(
            " ↑↓ 이동  Enter 이어하기  Esc 검색 취소",
            cond_fg(color_enabled, Color::DarkGray),
        ));
    } else if !selected_ids.is_empty() {
        // 다중선택 활성 시 선택 관련 키힌트
        spans.push(Span::styled(
            format!(
                " {}개 선택됨  Space 선택토글  a 전체선택/해제  Del 삭제  Esc 선택 해제",
                selected_ids.len()
            ),
            cond_fg(color_enabled, Color::Cyan),
        ));
    } else {
        // ① 커서 세션의 작업 폴더 풀경로(중간 생략)를 힌트 앞에 항상 노출.
        //    미리보기를 열지 않아도 "어느 폴더에서 쓴 세션인지" 바로 보이게.
        if !cursor_path.is_empty() {
            spans.push(Span::styled(
                format!(" {} ", middle_truncate(cursor_path, 50)),
                cond_fg_bold(color_enabled, Color::Yellow),
            ));
            spans.push(Span::raw("| "));
        }
        let mut hint = " ↑↓/jk 이동  Enter 이어하기  n 별칭  / 검색  s 정렬  g 그룹".to_string();
        if state.grouped {
            hint.push_str("  Tab 접기/펼치기");
        }
        hint.push_str(
            "  p 미리보기  Space 선택  a 전체선택  o 오래된선택  , 설정  Del 삭제  T 휴지통  ? 도움말  q 종료",
        );
        spans.push(Span::styled(hint, cond_fg(color_enabled, Color::DarkGray)));
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
    // 말줄임표 실제 폭 계산 (East Asian Ambiguous 대응)
    const ELLIPSIS: char = '…';
    let mut ellipsis_buf = [0u8; 4];
    let ellipsis_width = UnicodeWidthStr::width(ELLIPSIS.encode_utf8(&mut ellipsis_buf));

    let budget = max_width.saturating_sub(ellipsis_width);
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
    result.push(ELLIPSIS);
    result
}

/// 경로 등 양끝이 모두 중요한 문자열을 중간 생략(`앞…뒤`)으로 줄인다(①).
/// max_width 이하면 원본 그대로. 폭은 유니코드 표시폭 기준.
pub fn middle_truncate(s: &str, max_width: usize) -> String {
    if UnicodeWidthStr::width(s) <= max_width {
        return s.to_string();
    }
    if max_width <= 1 {
        return "…".to_string();
    }
    // 말줄임표 실제 폭 계산 (East Asian Ambiguous 대응)
    const ELLIPSIS: char = '…';
    let mut ellipsis_buf = [0u8; 4];
    let ellipsis_width = UnicodeWidthStr::width(ELLIPSIS.encode_utf8(&mut ellipsis_buf));

    let budget = max_width.saturating_sub(ellipsis_width);
    let tail_budget = budget.div_ceil(2);
    let head_budget = budget - tail_budget;

    let take_prefix = |budget: usize| -> String {
        let mut width = 0usize;
        let mut out = String::new();
        for c in s.chars() {
            let cw = UnicodeWidthStr::width(c.encode_utf8(&mut [0u8; 4]));
            if width + cw > budget {
                break;
            }
            width += cw;
            out.push(c);
        }
        out
    };
    let take_suffix = |budget: usize| -> String {
        let mut width = 0usize;
        let mut rev = String::new();
        for c in s.chars().rev() {
            let cw = UnicodeWidthStr::width(c.encode_utf8(&mut [0u8; 4]));
            if width + cw > budget {
                break;
            }
            width += cw;
            rev.push(c);
        }
        rev.chars().rev().collect()
    };

    format!(
        "{}{}{}",
        take_prefix(head_budget),
        ELLIPSIS,
        take_suffix(tail_budget)
    )
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

    #[test]
    fn test_middle_truncate_no_truncate() {
        let s = "/home/user/proj";
        assert_eq!(middle_truncate(s, 50), s);
    }

    #[test]
    fn test_middle_truncate_keeps_both_ends() {
        // 앞 루트와 뒤 leaf 모두 보존 + 폭 예산 준수
        let s = "/Users/minjun/Dev/some/very/deep/nested/claudeDesk";
        let t = middle_truncate(s, 20);
        assert!(UnicodeWidthStr::width(t.as_str()) <= 20);
        assert!(t.contains('…'));
        assert!(t.starts_with('/')); // 앞부분(루트) 보존
        assert!(t.ends_with("Desk")); // 뒷부분(leaf) 보존
    }
}
