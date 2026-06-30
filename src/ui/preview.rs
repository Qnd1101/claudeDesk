/// FR-08 미리보기 패널 렌더러
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::theme::Palette;
use crate::preview::PreviewContent;

/// 미리보기 패널을 area에 렌더한다.
///
/// - `content`: `read_preview`가 반환한 대화 내용
/// - `session_title`: 패널 상단 타이틀에 표시할 세션 제목(짧게 잘라 사용)
/// - `session_path`: 세션이 실행된 작업 디렉토리(cwd) 전체 경로. 빈 문자열이면 생략(①).
/// - `palette`: 색상 팔레트
pub fn render_preview(
    f: &mut Frame,
    area: Rect,
    content: &PreviewContent,
    session_title: &str,
    session_path: &str,
    palette: Palette,
) {
    // 타이틀: "Preview — 세션제목(최대 20자)" 형식
    let short_title = truncate_title(session_title, 20);
    let block_title = if short_title.is_empty() {
        " Preview ".to_string()
    } else {
        format!(" Preview — {} ", short_title)
    };

    // ① 풀 경로 메타 헤더: 세션이 어느 폴더에서 실행됐는지 한눈에.
    //    Wrap(trim:false)로 긴 경로도 잘리지 않고 줄바꿈 표시.
    let path_header: Vec<Line> = if session_path.is_empty() {
        Vec::new()
    } else {
        vec![
            Line::from(vec![Span::styled(
                session_path.to_string(),
                palette.fg_bold(palette.warning),
            )]),
            Line::from(""),
        ]
    };

    // turns가 비어 있는 경우 (빈 파일, 그룹 헤더 커서 등)
    if content.turns.is_empty() {
        let mut lines = path_header;
        lines.push(Line::from(vec![Span::styled(
            "미리보기 없음",
            palette.fg(palette.muted),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "(세션에 대화 내용이 없습니다)",
            palette.fg(palette.muted),
        )]));
        let p = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(block_title)
                    .border_style(palette.fg(palette.muted)),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(p, area);
        return;
    }

    // 대화 턴을 Line 목록으로 변환 (경로 헤더 먼저)
    let mut lines: Vec<Line> = path_header;

    for (i, turn) in content.turns.iter().enumerate() {
        // 턴 구분 빈 줄 (첫 턴 제외)
        if i > 0 {
            lines.push(Line::from(""));
        }

        // 역할 헤더
        let (role_symbol, role_label, role_style) = if turn.role == "user" {
            ("●", "user     ", palette.fg_bold(palette.user_msg))
        } else {
            ("○", "assistant", palette.fg_bold(palette.muted))
        };

        lines.push(Line::from(vec![Span::styled(
            format!("{} {}", role_symbol, role_label),
            role_style,
        )]));

        // 텍스트 본문: 개행 기준으로 각 줄을 Line으로 변환
        for text_line in turn.text.lines() {
            lines.push(Line::from(vec![Span::styled(
                format!("  {}", text_line),
                palette.fg(palette.body),
            )]));
        }
    }

    // truncated 안내
    if content.truncated {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "  … (미리보기 일부 — 전체는 Enter로 이어하기)",
            palette.fg(palette.muted),
        )]));
    }

    // 스킵된 줄 수 표시
    if content.skipped_lines > 0 {
        lines.push(Line::from(vec![Span::styled(
            format!("  [파싱 불가 {}줄 스킵]", content.skipped_lines),
            palette.fg(palette.muted),
        )]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(block_title)
        .border_style(palette.fg(palette.muted));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// 미리보기 패널 타이틀용 — chars 기준으로 max_chars자로 자름
fn truncate_title(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = chars[..max_chars].iter().collect();
        format!("{}…", truncated.trim_end())
    }
}
