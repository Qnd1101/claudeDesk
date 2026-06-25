/// FR-08 미리보기 — 통합 테스트
///
/// 모든 테스트는 합성 픽스처만 사용한다. 실제 세션 파일 접근 금지.
use claudedesk::preview::{read_preview, MAX_PREVIEW_BYTES, MAX_PREVIEW_LINES};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;

// ── 헬퍼 ──────────────────────────────────────────────────────────────────

fn fixture_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// 파일 SHA-256 계산. 파일이 없거나 읽기 실패 시 None 반환(패닉 없음).
fn sha256_file(path: &Path) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = match file.read(&mut buf) {
            Ok(n) => n,
            Err(_) => return None,
        };
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(hex::encode(hasher.finalize()))
}

// ── 원본 불변 테스트 ──────────────────────────────────────────────────────

/// read_preview 호출 전후 픽스처 파일 SHA-256 불변 검증
#[test]
fn test_preview_does_not_modify_fixtures() {
    let fixtures = [
        "preview_multiturn.jsonl",
        "normal_with_meta.jsonl",
        "block_array_content.jsonl",
        "corrupted_lines.jsonl",
        "empty.jsonl",
        "large_single_line.jsonl",
    ];

    for name in &fixtures {
        let path = fixture_path(name);
        let hash_before = sha256_file(&path)
            .unwrap_or_else(|| panic!("픽스처 {name} SHA-256 계산 실패 (파일 없음?)"));

        let _ = read_preview(&path, MAX_PREVIEW_LINES, MAX_PREVIEW_BYTES);

        let hash_after =
            sha256_file(&path).unwrap_or_else(|| panic!("픽스처 {name} 읽기 후 SHA-256 계산 실패"));

        assert_eq!(
            hash_before, hash_after,
            "픽스처 {name}: read_preview 후 SHA-256 변경 — 원본 훼손!"
        );
    }
}

// ── 정상 멀티턴 픽스처 ────────────────────────────────────────────────────

/// 멀티턴 픽스처: turns 순서, role, text 정확성 검증
#[test]
fn test_multiturn_turns_order_and_roles() {
    let path = fixture_path("preview_multiturn.jsonl");
    let content = read_preview(&path, MAX_PREVIEW_LINES, MAX_PREVIEW_BYTES);

    // 픽스처에는 user 3턴 + assistant 3턴 = 6턴
    assert_eq!(
        content.turns.len(),
        6,
        "멀티턴 픽스처 turn 수 불일치: {}",
        content.turns.len()
    );

    // 첫 턴: user, String content
    assert_eq!(content.turns[0].role, "user");
    assert!(
        content.turns[0].text.contains("첫 번째 user 메시지"),
        "첫 번째 user 텍스트 불일치: {}",
        content.turns[0].text
    );

    // 두 번째 턴: assistant
    assert_eq!(content.turns[1].role, "assistant");
    assert!(content.turns[1].text.contains("첫 번째 assistant 응답"));

    // 세 번째 턴: user, 블록 배열 (text + tool_use)
    assert_eq!(content.turns[2].role, "user");
    assert!(
        content.turns[2].text.contains("두 번째 user"),
        "두 번째 user 블록 텍스트 불일치: {}",
        content.turns[2].text
    );
    assert!(
        content.turns[2].text.contains("[도구 호출: Read]"),
        "tool_use placeholder 누락: {}",
        content.turns[2].text
    );

    // 네 번째 턴: assistant, 블록 배열 (tool_result + text)
    assert_eq!(content.turns[3].role, "assistant");
    assert!(content.turns[3].text.contains("[도구 결과]"));
    assert!(content.turns[3].text.contains("파일을 읽었습니다"));

    assert!(!content.truncated);
    assert_eq!(content.skipped_lines, 0);
}

// ── max_lines 캡 테스트 ───────────────────────────────────────────────────

/// max_lines를 작게 줄 때 truncated=true이고 캡 초과분을 읽지 않음
#[test]
fn test_max_lines_cap_truncates() {
    let path = fixture_path("preview_multiturn.jsonl");
    // max_lines=2로 제한 (대화 턴 1~2개만 통과)
    let content = read_preview(&path, 2, MAX_PREVIEW_BYTES);

    assert!(
        content.truncated,
        "max_lines=2 로 6턴 픽스처를 읽었는데 truncated=false"
    );
    // turns는 1~2개 (max_lines=2줄 이하)
    assert!(
        content.turns.len() < 6,
        "max_lines=2 에서 6턴 전부 읽힘 — 캡 미작동"
    );
}

// ── max_bytes 캡 테스트 ───────────────────────────────────────────────────

/// max_bytes를 극히 작게 줄 때 truncated=true
#[test]
fn test_max_bytes_cap_truncates() {
    let path = fixture_path("preview_multiturn.jsonl");
    // 100 바이트만 허용 — agent-setting 줄 혼자 이미 초과할 수 있음
    let content = read_preview(&path, MAX_PREVIEW_LINES, 100);

    // turns가 0이거나 truncated인 경우 모두 허용 (파일 앞부분에 비-user 줄이 있을 수 있음)
    // 단, turns가 6개 전부이면 캡이 작동하지 않은 것
    let all_read = content.turns.len() == 6 && !content.truncated;
    assert!(!all_read, "max_bytes=100 에서 전체 6턴이 읽힘 — 캡 미작동");
}

// ── 깨진 줄 픽스처 ────────────────────────────────────────────────────────

/// corrupted_lines 픽스처: skip 카운트, 패닉 0
#[test]
fn test_corrupted_lines_skip_and_no_panic() {
    let path = fixture_path("corrupted_lines.jsonl");
    let content = read_preview(&path, MAX_PREVIEW_LINES, MAX_PREVIEW_BYTES);

    // 깨진 줄 2개 → skipped_lines >= 2
    assert!(
        content.skipped_lines >= 2,
        "corrupted_lines 픽스처 skipped_lines={} (≥2 기대)",
        content.skipped_lines
    );
    // 정상 줄에서 turns 추출 (user 1 + assistant 1 = 2)
    assert!(
        !content.turns.is_empty(),
        "corrupted 픽스처에서 turn을 하나도 추출 못함"
    );
    // 패닉: 테스트 자체가 완료되면 패닉 0 검증 완료
}

// ── 빈 파일 ───────────────────────────────────────────────────────────────

/// 빈 파일 → 빈 turns, truncated=false, 에러 아님
#[test]
fn test_empty_file_returns_empty_turns() {
    let path = fixture_path("empty.jsonl");
    let content = read_preview(&path, MAX_PREVIEW_LINES, MAX_PREVIEW_BYTES);

    assert!(content.turns.is_empty(), "빈 파일에서 turns 비어야 함");
    assert!(!content.truncated);
    assert_eq!(content.skipped_lines, 0);
}

// ── 없는 경로 ─────────────────────────────────────────────────────────────

/// 존재하지 않는 경로 → 빈 turns, 에러 아님(패닉 없음)
#[test]
fn test_nonexistent_path_returns_empty() {
    let path = Path::new("/nonexistent/path/that/does/not/exist.jsonl");
    let content = read_preview(path, MAX_PREVIEW_LINES, MAX_PREVIEW_BYTES);

    assert!(content.turns.is_empty(), "없는 경로에서 빈 turns 기대");
    assert!(!content.truncated);
}

// ── normal_with_meta 픽스처 ──────────────────────────────────────────────

/// normal_with_meta: user/assistant turns 추출, agent-setting/mode 줄 스킵
#[test]
fn test_normal_with_meta_turns() {
    let path = fixture_path("normal_with_meta.jsonl");
    let content = read_preview(&path, MAX_PREVIEW_LINES, MAX_PREVIEW_BYTES);

    // user 2 + assistant 2 = 4턴 (assistant 1 선행)
    assert_eq!(
        content.turns.len(),
        4,
        "normal_with_meta 픽스처 turn 수 불일치: {}",
        content.turns.len()
    );
    // 첫 턴이 assistant임을 확인 (픽스처 구조: assistant → user → assistant → user)
    assert_eq!(content.turns[0].role, "assistant");
    assert_eq!(content.turns[1].role, "user");
    assert!(content.turns[1].text.contains("claudeDesk PRD"));
}

// ── 블록 배열 + 비-text 블록 placeholder ─────────────────────────────────

/// block_array_content: text 블록 join, tool_result placeholder
#[test]
fn test_block_array_content_extraction() {
    let path = fixture_path("block_array_content.jsonl");
    let content = read_preview(&path, MAX_PREVIEW_LINES, MAX_PREVIEW_BYTES);

    // user 1 + assistant 1 = 2턴
    assert_eq!(content.turns.len(), 2);

    // user 턴: tool_result 블록은 [도구 결과]로, text 블록은 실제 텍스트로
    let user_text = &content.turns[0].text;
    assert!(
        user_text.contains("[도구 결과]"),
        "tool_result placeholder 누락: {}",
        user_text
    );
    assert!(
        user_text.contains("블록 배열 content 첫 텍스트 추출 테스트"),
        "text 블록 텍스트 누락: {}",
        user_text
    );
}

// ── [Critical] 거대 단일 줄 RAM 바운드 회귀 테스트 ───────────────────────

/// large_single_line.jsonl(~210KB 단일 user 줄): take() 하드 실링으로
/// ① 패닉 없이 완료 ② turns/바이트가 캡 내에 제한됨 ③ truncated=true 검증.
#[test]
fn test_large_single_line_ram_bound() {
    let path = fixture_path("large_single_line.jsonl");
    // max_bytes=32KB로 제한 — 파일 210KB 중 32KB만 읽혀야 함
    let max_bytes = 32 * 1024;
    let content = read_preview(&path, MAX_PREVIEW_LINES, max_bytes);

    // ① 패닉 없이 완료 (테스트 도달 시 자동 확인)
    // ② 거대 user 줄(210KB)은 캡(32KB)에 막혀 읽히지 않아야 함
    //    → turns가 비어 있거나, truncated=true 중 하나여야 함
    let bound_ok = content.turns.is_empty() || content.truncated;
    assert!(
        bound_ok,
        "거대 단일 줄이 캡 없이 전부 읽힘 — RAM 바운드 미작동. turns={}, truncated={}",
        content.turns.len(),
        content.truncated
    );

    // ③ 읽힌 turns의 텍스트 총 바이트가 max_bytes를 넘으면 안 됨
    let total_text_bytes: usize = content.turns.iter().map(|t| t.text.len()).sum();
    assert!(
        total_text_bytes <= max_bytes,
        "turns 텍스트 총 바이트({})가 max_bytes({})를 초과",
        total_text_bytes,
        max_bytes
    );
}

/// large_single_line.jsonl: 기본(64KB) max_bytes에서도 거대 줄이 skipped 처리됨
#[test]
fn test_large_single_line_skipped_with_default_cap() {
    let path = fixture_path("large_single_line.jsonl");
    // 기본 64KB 캡 — 210KB 줄은 per-line 가드(MAX_LINE_BYTES=256KB) 내이지만
    // take(64KB)에 걸려 잘린 불완전 JSON → skipped 또는 truncated
    let content = read_preview(&path, MAX_PREVIEW_LINES, MAX_PREVIEW_BYTES);

    // 거대 줄이 정상 텍스트로 turn에 들어오면 안 됨:
    // turns가 비어 있거나 truncated이거나 skipped_lines > 0 중 하나
    let handled = content.turns.is_empty() || content.truncated || content.skipped_lines > 0;
    assert!(
        handled,
        "거대 줄이 캡/스킵 없이 그대로 turn으로 들어옴 — per-line 가드 또는 take() 미작동"
    );
}

// ── [Low] 갭 테스트: null content user 줄 skip ───────────────────────────

/// null_content_fallback: content=null user 줄은 turn에 포함되지 않아야 함
#[test]
fn test_null_content_user_line_not_in_turns() {
    let path = fixture_path("null_content_fallback.jsonl");
    // 픽스처: user(content=null) → assistant → user(정상)
    let content = read_preview(&path, MAX_PREVIEW_LINES, MAX_PREVIEW_BYTES);

    // null content user 줄은 turn으로 만들어지지 않아야 함
    // 총 3줄(user null, assistant, user 정상) → 유효 turn: assistant 1 + user(정상) 1 = 2
    assert_eq!(
        content.turns.len(),
        2,
        "null content user 줄이 turn에 포함됨: turns={:?}",
        content.turns
    );

    // 첫 턴이 assistant여야 함 (null user 다음)
    assert_eq!(
        content.turns[0].role, "assistant",
        "첫 턴이 assistant가 아님: {}",
        content.turns[0].role
    );

    // 두 번째 턴이 정상 user 메시지여야 함
    assert_eq!(content.turns[1].role, "user");
    assert!(
        content.turns[1].text.contains("null content 폴백"),
        "두 번째 user turn 텍스트 불일치: {}",
        content.turns[1].text
    );
}

// ── [Low] 갭 테스트: 이모지/다국어 보존 ─────────────────────────────────

/// emoji_and_unicode: turn.text에 이모지·다국어가 정확히 보존되는지 검증
#[test]
fn test_emoji_and_unicode_preserved_in_turns() {
    let path = fixture_path("emoji_and_unicode.jsonl");
    let content = read_preview(&path, MAX_PREVIEW_LINES, MAX_PREVIEW_BYTES);

    // 픽스처: user(이모지) → assistant(이모지) → user(다국어)
    assert_eq!(
        content.turns.len(),
        3,
        "이모지 픽스처 turn 수 불일치: {}",
        content.turns.len()
    );

    // 첫 user 턴에 이모지 보존
    let first_user = &content.turns[0].text;
    assert!(
        first_user.contains("🚀"),
        "🚀 이모지 미보존: {}",
        first_user
    );
    assert!(
        first_user.contains("🎉"),
        "🎉 이모지 미보존: {}",
        first_user
    );

    // assistant 턴에 이모지 보존
    let assistant_text = &content.turns[1].text;
    assert!(
        assistant_text.contains("✅"),
        "✅ 이모지 미보존: {}",
        assistant_text
    );

    // 두 번째 user 턴에 다국어 보존
    let second_user = &content.turns[2].text;
    assert!(
        second_user.contains("こんにちは"),
        "일본어 미보존: {}",
        second_user
    );
    assert!(
        second_user.contains("안녕하세요"),
        "한국어 미보존: {}",
        second_user
    );
    assert!(
        second_user.contains("Héllo"),
        "라틴 확장 문자 미보존: {}",
        second_user
    );
}
