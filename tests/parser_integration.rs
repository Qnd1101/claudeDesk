use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::SystemTime;

// claudedesk 크레이트 모듈 접근
use claudedesk::data::FileMeta;
use claudedesk::parser::{extract_text_from_content, parse_session};

// ── 헬퍼 ──────────────────────────────────────────────────────────────────

fn fixture_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn make_meta(name: &str) -> FileMeta {
    let path = fixture_path(name);
    let meta = fs::metadata(&path).expect("픽스처 stat 실패");
    FileMeta {
        path,
        mtime: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        ctime: meta.created().unwrap_or(SystemTime::UNIX_EPOCH),
        size: meta.len(),
    }
}

fn sha256_file(path: &Path) -> String {
    let mut file = fs::File::open(path).expect("파일 열기 실패");
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).expect("읽기 실패");
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    hex::encode(hasher.finalize())
}

// ── 원본 불변 테스트 (FR §5.11, docs/03_DEV_KICKOFF §9.4) ────────────────

/// 픽스처 전체에 대해 파싱 전후 SHA-256이 동일한지 검증
#[test]
fn test_fixtures_immutable_after_parse() {
    let fixtures = [
        "normal_with_meta.jsonl",
        "block_array_content.jsonl",
        "corrupted_lines.jsonl",
        "empty.jsonl",
        "no_user_lines.jsonl",
        // 엣지 픽스처 (§5.11, FAIL-03)
        "emoji_and_unicode.jsonl",
        "over_scan_limit.jsonl",
        "null_content_fallback.jsonl",
        "surrogate.jsonl",
    ];

    for name in &fixtures {
        let path = fixture_path(name);
        let hash_before = sha256_file(&path);

        let meta = make_meta(name);
        let _ = parse_session(&meta); // 파싱 실행

        let hash_after = sha256_file(&path);

        assert_eq!(
            hash_before, hash_after,
            "픽스처 {name} 파싱 후 SHA-256 변경 — 원본 훼손!"
        );
    }
}

// ── 파서 유닛 테스트 ──────────────────────────────────────────────────────

/// 메타 줄 선행 → 올바른 user 제목 추출
#[test]
fn test_normal_with_meta_title() {
    let meta = make_meta("normal_with_meta.jsonl");
    let result = parse_session(&meta).expect("파싱 실패");
    assert_eq!(result.title, "claudeDesk PRD 재설계 요청");
}

/// 메타 줄 선행 → cwd 추출
#[test]
fn test_normal_with_meta_cwd() {
    let meta = make_meta("normal_with_meta.jsonl");
    let result = parse_session(&meta).expect("파싱 실패");
    assert_eq!(result.cwd, "/home/synth/project");
}

/// 메타 줄 선행 → 메시지 수 (user 2 + assistant 2 = 4)
#[test]
fn test_normal_with_meta_msg_count() {
    let meta = make_meta("normal_with_meta.jsonl");
    let result = parse_session(&meta).expect("파싱 실패");
    assert_eq!(result.msg_count, 4);
}

/// 블록 배열 content → 첫 text 블록 추출
#[test]
fn test_block_array_title() {
    let meta = make_meta("block_array_content.jsonl");
    let result = parse_session(&meta).expect("파싱 실패");
    assert_eq!(result.title, "블록 배열 content 첫 텍스트 추출 테스트");
}

/// 손상 줄 2개 → graceful skip, 크래시 0, 나머지 정상 추출
#[test]
fn test_corrupted_lines_skip() {
    let meta = make_meta("corrupted_lines.jsonl");
    let result = parse_session(&meta).expect("파싱 실패 (크래시 금지)");
    // 손상 줄이 스킵돼야 함 (2줄)
    assert!(result.skipped_lines >= 2, "손상 줄 스킵 카운트 부족");
    // 나머지 정상 줄에서 제목 추출
    assert_eq!(result.title, "손상 줄 스킵 후 정상 파싱");
}

/// 빈 파일 → Untitled Session, 크래시 0
#[test]
fn test_empty_file_fallback() {
    let meta = make_meta("empty.jsonl");
    let result = parse_session(&meta).expect("파싱 실패 (크래시 금지)");
    assert_eq!(result.title, "Untitled Session");
    assert_eq!(result.msg_count, 0);
}

/// user 줄 없는 파일 → Untitled Session 폴백
#[test]
fn test_no_user_fallback() {
    let meta = make_meta("no_user_lines.jsonl");
    let result = parse_session(&meta).expect("파싱 실패");
    assert_eq!(result.title, "Untitled Session");
}

// ── extract_text_from_content 유닛 테스트 ────────────────────────────────

#[test]
fn test_extract_string_content() {
    use serde_json::json;
    let v = json!("hello");
    assert_eq!(
        extract_text_from_content(Some(&v)),
        Some("hello".to_string())
    );
}

#[test]
fn test_extract_block_array_skips_non_text() {
    use serde_json::json;
    let v = json!([
        {"type": "tool_result", "content": "artifact"},
        {"type": "text", "text": "실제 텍스트"}
    ]);
    assert_eq!(
        extract_text_from_content(Some(&v)),
        Some("실제 텍스트".to_string())
    );
}

#[test]
fn test_extract_none_on_empty_array() {
    use serde_json::json;
    let v = json!([{"type": "tool_use"}]);
    assert_eq!(extract_text_from_content(Some(&v)), None);
}

// ── 폴더명 처리 유닛 테스트 ──────────────────────────────────────────────

#[test]
fn test_cwd_to_folder_roundtrip() {
    use claudedesk::config::cwd_to_folder_name;
    let cwd = "D:\\Dev\\claudeDesk";
    let folder = cwd_to_folder_name(cwd);
    assert_eq!(folder, "D--Dev-claudeDesk");
}

#[test]
fn test_is_subagent_path() {
    use claudedesk::config::is_subagent_path;
    let p = Path::new("/home/.claude/projects/D--Dev/abc/subagents/agent-1.jsonl");
    assert!(is_subagent_path(p));

    let p2 = Path::new("/home/.claude/projects/D--Dev/abc.jsonl");
    assert!(!is_subagent_path(p2));
}

// ── 엣지 픽스처 테스트 (§5.11 FAIL-03) ──────────────────────────────────────

/// 이모지/다국어 본문 → 크래시 없이 제목 추출 (FAIL-03)
#[test]
fn test_emoji_unicode_no_crash() {
    let meta = make_meta("emoji_and_unicode.jsonl");
    let result = parse_session(&meta).expect("이모지 픽스처 파싱 크래시 금지");
    // 제목이 비어 있지 않고(Untitled 또는 실제 텍스트), 크래시 없어야 함
    assert!(!result.title.is_empty());
    // 이모지를 포함한 첫 user 줄이 제목으로 추출돼야 함
    assert!(
        result.title.contains("이모지") || result.title.contains("🚀"),
        "이모지/다국어 제목 추출 실패: {}",
        result.title
    );
}

/// MAX_SCAN_LINES(64) 초과 메타 선행 → Untitled Session 경계 동작 (FAIL-03)
#[test]
fn test_over_scan_limit_untitled() {
    let meta = make_meta("over_scan_limit.jsonl");
    let result = parse_session(&meta).expect("over_scan_limit 파싱 크래시 금지");
    // 64줄 초과 탐색 포기 → Untitled
    assert_eq!(
        result.title, "Untitled Session",
        "MAX_SCAN_LINES 초과 시 Untitled 미반환: {}",
        result.title
    );
    // 메시지 수는 user+assistant 카운트 계속돼야 함
    assert!(
        result.msg_count >= 1,
        "over_scan 픽스처 메시지 수 0 (assistant 1개 이상 있어야 함)"
    );
}

/// lone(broken) surrogate 든 줄 → serde_json 파싱 실패로 graceful skip,
/// 크래시 0, 뒤 정상 줄에서 제목·cwd 추출 (§5.11 회귀 픽스처 세트)
#[test]
fn test_surrogate_lone_skip() {
    let meta = make_meta("surrogate.jsonl");
    let result = parse_session(&meta).expect("surrogate 픽스처 파싱 크래시 금지");
    // lone surrogate 줄이 graceful skip 돼야 함
    assert!(
        result.skipped_lines >= 1,
        "lone surrogate 줄 스킵 카운트 부족: {}",
        result.skipped_lines
    );
    // 스킵 후 다음 정상 user 줄이 제목이 돼야 함
    assert_eq!(
        result.title, "surrogate 줄 스킵 후 이 줄이 제목이 돼야 함",
        "surrogate 스킵 후 제목 추출 실패: {}",
        result.title
    );
    // cwd는 정상 줄에서 추출 (surrogate 줄이 첫 줄이지만 스킵되므로)
    assert_eq!(result.cwd, "/synthetic/surrogate");
    // 메시지 수: 정상 user 1 + assistant 1 = 2 (surrogate user 줄은 스킵)
    assert_eq!(
        result.msg_count, 2,
        "메시지 수 불일치: {}",
        result.msg_count
    );
}

/// message.content가 null인 user 줄 → 다음 user 줄로 폴백 (FAIL-03)
#[test]
fn test_null_content_fallback() {
    let meta = make_meta("null_content_fallback.jsonl");
    let result = parse_session(&meta).expect("null_content 파싱 크래시 금지");
    // 첫 user 줄 content=null → extract_text_from_content은 None 반환
    // → 다음 user 줄(두 번째 user)이 제목이 돼야 함
    assert_eq!(
        result.title, "null content 폴백 후 이 줄이 제목이 돼야 함",
        "null content 폴백 실패: {}",
        result.title
    );
    // 메시지 수: user 2 + assistant 1 = 3
    assert_eq!(result.msg_count, 3, "메시지 수 불일치");
}
