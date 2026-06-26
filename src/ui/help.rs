use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::layout::centered_rect;

pub fn render_help(f: &mut Frame) {
    let area = centered_rect(58, 34, f.area());

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
        // 검색 (FR-05)
        Line::from(vec![
            Span::styled("  /    ", Style::default().fg(Color::Cyan)),
            Span::raw("검색 진입 (제목·프로젝트 incremental 필터)"),
        ]),
        Line::from(vec![
            Span::styled("  Esc  ", Style::default().fg(Color::Cyan)),
            Span::raw("검색 해제 · 전체 목록 복귀"),
        ]),
        Line::from(""),
        // 정렬 (FR-07)
        Line::from(vec![
            Span::styled("  s    ", Style::default().fg(Color::Magenta)),
            Span::raw("정렬 키 순환  Modified → Created → Title → Messages"),
        ]),
        Line::from(vec![
            Span::styled("  S    ", Style::default().fg(Color::Magenta)),
            Span::raw("정렬 방향 토글  ↓(내림차순) ↔ ↑(오름차순)"),
        ]),
        Line::from(""),
        // 다중선택 + 삭제 (FR-04)
        Line::from(vec![
            Span::styled("  Space", Style::default().fg(Color::Red)),
            Span::raw("  다중선택 토글 (✓ 마커)"),
        ]),
        Line::from(vec![
            Span::styled("  a    ", Style::default().fg(Color::Red)),
            Span::raw("  전체선택/해제 토글"),
        ]),
        Line::from(vec![
            Span::styled("  Del/d", Style::default().fg(Color::Red)),
            Span::raw("  삭제 확인 모달 → 휴지통 이동"),
        ]),
        Line::from(vec![
            Span::styled("  o    ", Style::default().fg(Color::Red)),
            Span::raw("  오래된 세션 선택(기준일 이전) → d로 삭제 (FR-14)"),
        ]),
        Line::from(""),
        // 휴지통 (FR-11)
        Line::from(vec![
            Span::styled("  T    ", Style::default().fg(Color::Yellow)),
            Span::raw("휴지통 화면 열기"),
        ]),
        Line::from(vec![
            Span::raw("  [휴지통] "),
            Span::styled("r", Style::default().fg(Color::Green)),
            Span::raw(" 복구  "),
            Span::styled("D", Style::default().fg(Color::Red)),
            Span::raw(" 영구삭제(2단계)  "),
            Span::styled("Esc", Style::default().fg(Color::DarkGray)),
            Span::raw(" 닫기"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  g    ", Style::default().fg(Color::Blue)),
            Span::raw("평면/그룹 모드 토글"),
        ]),
        Line::from(vec![
            Span::styled("  Tab  ", Style::default().fg(Color::Blue)),
            Span::raw("현재 그룹 접기/펼치기 토글"),
        ]),
        Line::from(""),
        // 별칭 (FR-06)
        Line::from(vec![
            Span::styled("  n    ", Style::default().fg(Color::Cyan)),
            Span::raw("별칭 지정/편집 (빈칸 저장=삭제)"),
        ]),
        Line::from(""),
        // 미리보기 (FR-08)
        Line::from(vec![
            Span::styled("  p    ", Style::default().fg(Color::Cyan)),
            Span::raw("미리보기 패널 토글 (≥100칸 필요, Normal 모드 전용)"),
        ]),
        Line::from(""),
        // 설정 (FR-10)
        Line::from(vec![
            Span::styled("  ,    ", Style::default().fg(Color::Cyan)),
            Span::raw("설정 화면 열기 (테마·정렬·경로 등)"),
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
