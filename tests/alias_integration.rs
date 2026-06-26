/// FR-06 별칭 통합 테스트
///
/// 합성 픽스처만 사용 (실 cwd·세션 본문 금지 §5.11).
/// 임시 디렉토리 사용 — 실 홈 디렉토리 불변 보장.
use std::io::Write;
use std::path::Path;
use std::time::SystemTime;

use claudedesk::alias::{load_alias_from, save_alias_to, AliasStore};
use claudedesk::data::FileMeta;
use claudedesk::parser::{build_search_text, build_session, parse_session};
use tempfile::TempDir;

// ── SHA-256 헬퍼 ─────────────────────────────────────────────────────────────

fn sha256_file(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let data = std::fs::read(path).unwrap();
    hex::encode(Sha256::digest(&data))
}

// ── 픽스처 헬퍼 ──────────────────────────────────────────────────────────────

fn synth_jsonl(session_id: &str, title: &str) -> String {
    format!(
        "{{\"type\":\"user\",\"sessionId\":\"{session_id}\",\"cwd\":\"/synth/project\",\
         \"message\":{{\"content\":\"{title}\"}}}}\n"
    )
}

fn make_fixture(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

// ═══════════════════════════════════════════════════════════════════════════════
// 1. 별칭 설정 → 재로드 → display_title / 검색 매칭 확인
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_alias_set_reload_display_title_and_search() {
    let tmp = TempDir::new().unwrap();
    let meta_path = tmp.path().join("meta.json");
    let jsonl_dir = tmp.path().join("sessions");
    std::fs::create_dir_all(&jsonl_dir).unwrap();

    let session_id = "4bf02f8c-2370-4906-b145-2518877fe1e6";
    let alias_text = "결제 모듈 리팩터";

    // 합성 JSONL 생성
    let content = synth_jsonl(session_id, "Payment refactor initial work");
    let jsonl_path = make_fixture(&jsonl_dir, &format!("{session_id}.jsonl"), &content);

    // 별칭 설정 + 저장
    let mut store = AliasStore::default();
    store.set(session_id, alias_text);
    save_alias_to(&store, &meta_path).unwrap();

    // 재로드
    let loaded_store = load_alias_from(&meta_path);
    assert_eq!(loaded_store.get(session_id), Some(alias_text));

    // 파싱 + 별칭 주입 → display_title 확인
    let meta = FileMeta {
        path: jsonl_path.clone(),
        mtime: SystemTime::UNIX_EPOCH,
        ctime: SystemTime::UNIX_EPOCH,
        size: 0,
    };
    let result = parse_session(&meta).unwrap();
    let alias = loaded_store.get(session_id);
    let session = build_session(&meta, result, 300, alias);

    assert_eq!(
        session.display_title(),
        alias_text,
        "display_title이 별칭을 반환하지 않음"
    );
    assert_eq!(
        session.title, "Payment refactor initial work",
        "도출 title이 변경됨 — 별칭 삭제 시 복원 불가"
    );

    // 검색 텍스트에 별칭 포함 여부
    assert!(
        session.search_text.contains("결제 모듈 리팩터"),
        "search_text에 별칭이 없음: {}",
        session.search_text
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 2. 원본 JSONL SHA-256 불변: 별칭 set/save 전후 픽스처 SHA 동일 단언
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_alias_save_does_not_touch_jsonl_sha() {
    let tmp = TempDir::new().unwrap();
    let meta_path = tmp.path().join("meta.json");

    let session_id = "sha-test-session-001";
    let content = synth_jsonl(session_id, "SHA 불변 테스트 세션");
    let jsonl_path = make_fixture(tmp.path(), &format!("{session_id}.jsonl"), &content);

    // JSONL SHA (별칭 저장 전)
    let hash_before = sha256_file(&jsonl_path);

    // 별칭 저장 (meta.json 사이드카만 변경해야 함)
    let mut store = AliasStore::default();
    store.set(session_id, "SHA 안전 별칭");
    save_alias_to(&store, &meta_path).unwrap();

    // JSONL SHA (별칭 저장 후) — 동일해야 함
    let hash_after = sha256_file(&jsonl_path);
    assert_eq!(
        hash_before, hash_after,
        "별칭 저장 후 JSONL SHA 변경 — 원본 훼손! (사이드카만 변경돼야 함)"
    );

    // meta.json은 존재하고 JSONL과 다른 파일이어야 함
    assert!(meta_path.exists(), "meta.json이 생성되지 않음");
    assert_ne!(
        sha256_file(&meta_path),
        hash_before,
        "meta.json이 JSONL과 동일한 내용 — 경로 오류 가능성"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 3. 소프트 삭제 → 복구 후 별칭 보존 (alias 키 = session_id 불변)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_alias_preserved_through_soft_delete_and_restore() {
    let tmp = TempDir::new().unwrap();
    let meta_path = tmp.path().join("meta.json");
    let trash_dir = tmp.path().join("trash");
    std::fs::create_dir_all(&trash_dir).unwrap();

    let session_id = "restore-alias-session-abc";
    let content = synth_jsonl(session_id, "별칭 보존 테스트");
    let jsonl_path = make_fixture(tmp.path(), &format!("{session_id}.jsonl"), &content);

    // 별칭 설정
    let mut store = AliasStore::default();
    store.set(session_id, "복구 후에도 살아남는 별칭");
    save_alias_to(&store, &meta_path).unwrap();

    // 소프트 삭제 시뮬레이션 (JSONL 이동)
    let trash_path = trash_dir.join(format!("{session_id}_1000.jsonl"));
    std::fs::rename(&jsonl_path, &trash_path).unwrap();
    assert!(!jsonl_path.exists(), "소프트 삭제 후 원본이 남아있음");

    // 별칭 store는 건드리지 않음 — 재로드해도 별칭 보존
    let store_after_delete = load_alias_from(&meta_path);
    assert_eq!(
        store_after_delete.get(session_id),
        Some("복구 후에도 살아남는 별칭"),
        "소프트 삭제 후 alias store에서 별칭이 사라짐"
    );

    // 복구 시뮬레이션 (JSONL 원위치)
    std::fs::rename(&trash_path, &jsonl_path).unwrap();
    assert!(jsonl_path.exists(), "복구 후 원본 없음");

    // 복구 후에도 별칭 보존
    let store_after_restore = load_alias_from(&meta_path);
    assert_eq!(
        store_after_restore.get(session_id),
        Some("복구 후에도 살아남는 별칭"),
        "복구 후 alias store에서 별칭이 사라짐"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 4. build_search_text 별칭 결합 확인 (§5.2)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_build_search_text_alias_combined() {
    // 별칭 있을 때
    let text = build_search_text("My Session", None, "/proj", Some("결제 모듈"));
    assert!(
        text.contains("결제 모듈"),
        "별칭이 search_text에 없음: {text}"
    );
    assert!(text.contains("my session"), "title 소문자가 없음: {text}");
    assert!(text.contains("/proj"), "cwd가 없음: {text}");

    // 별칭 없을 때 (None vs Some("") 동일해야 함)
    let text_none = build_search_text("My Session", None, "/proj", None);
    let text_empty = build_search_text("My Session", None, "/proj", Some(""));
    assert_eq!(text_none, text_empty, "빈 별칭이 None과 다른 결과 생성");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 5. 기존 픽스처 SHA 불변 회귀 (별칭 모듈 추가 후에도 원본 불변)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_existing_fixtures_sha_unchanged_after_alias_module_added() {
    let fixtures_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");

    let fixture_names = [
        "normal_with_meta.jsonl",
        "block_array_content.jsonl",
        "corrupted_lines.jsonl",
        "empty.jsonl",
    ];

    for name in &fixture_names {
        let path = fixtures_dir.join(name);
        let hash_before = sha256_file(&path);

        let meta = FileMeta {
            path: path.clone(),
            mtime: SystemTime::UNIX_EPOCH,
            ctime: SystemTime::UNIX_EPOCH,
            size: 0,
        };
        // 별칭 없이 파싱 (기존 동작)
        let _ = parse_session(&meta);

        let hash_after = sha256_file(&path);
        assert_eq!(
            hash_before, hash_after,
            "픽스처 {name} — alias 모듈 추가 후 SHA 변경. 원본 훼손!"
        );
    }
}
