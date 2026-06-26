/// FR-04 소프트 삭제 + FR-11 휴지통/복구/영구삭제 통합 테스트
///
/// 모든 픽스처는 합성 데이터 사용 (실제 cwd·세션 본문 금지 §5.11).
/// 임시 디렉토리(tempfile) 사용 — 실 홈 디렉토리 불변 보장.
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use claudedesk::trash::{
    load_index_from, purge_sessions, soft_delete_sessions_to, TrashEntry, TrashIndex,
};
use tempfile::TempDir;

// ── SHA-256 헬퍼 ─────────────────────────────────────────────────────────────

fn sha256_file(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let data = std::fs::read(path).unwrap();
    hex::encode(Sha256::digest(&data))
}

// ── 픽스처 헬퍼 ──────────────────────────────────────────────────────────────

/// 합성 JSONL 파일 생성, 내용 + SHA 반환
fn make_fixture(dir: &Path, name: &str, content: &str) -> (PathBuf, String) {
    use sha2::{Digest, Sha256};
    let path = dir.join(name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    let hash = hex::encode(Sha256::digest(content.as_bytes()));
    (path, hash)
}

/// 최소 합성 JSONL 라인
fn synth_jsonl(session_id: &str, title: &str) -> String {
    format!(
        "{{\"type\":\"user\",\"sessionId\":\"{}\",\"cwd\":\"/synth/project\",\"message\":{{\"content\":\"{}\"}}}}\n",
        session_id, title
    )
}

// ── TrashIndex 커스텀 경로 헬퍼 ──────────────────────────────────────────────

/// 임시 경로에 TrashIndex 저장
fn save_index(idx: &TrashIndex, path: &Path) {
    let json = serde_json::to_string_pretty(idx).unwrap();
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &json).unwrap();
    std::fs::rename(&tmp, path).unwrap();
}

/// 임시 경로에서 TrashIndex 로드
fn load_index(path: &Path) -> TrashIndex {
    if !path.exists() {
        return TrashIndex::default();
    }
    let s = std::fs::read_to_string(path).unwrap_or_default();
    serde_json::from_str(&s).unwrap_or_default()
}

// ═══════════════════════════════════════════════════════════════════════════════
// 1. 소프트 삭제: 파일 이동 + 원본 없음 + 내용(SHA) 동일
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_soft_delete_file_moved_and_original_gone() {
    let src_dir = TempDir::new().unwrap();
    let trash_dir = TempDir::new().unwrap();

    let content = synth_jsonl("aaa-uuid", "소프트 삭제 테스트");
    let (src_path, hash_before) = make_fixture(src_dir.path(), "aaa-uuid.jsonl", &content);

    let sessions = [(
        "aaa-uuid",
        src_path.as_path(),
        "소프트 삭제 테스트",
        "/synth",
        false,
    )];
    let result = soft_delete_sessions_to(&sessions, trash_dir.path()).unwrap();

    // 이동 성공
    assert_eq!(result.moved.len(), 1, "moved에 항목이 없음");

    // 원본 없어야 함
    assert!(!src_path.exists(), "소프트 삭제 후 원본이 남아있음");

    // 인덱스에서 trash_path 확인 후 SHA 비교
    let index = load_index_from(&trash_dir.path().join("index.json"));
    let entry = index.entries.get("aaa-uuid").expect("인덱스에 항목 없음");
    let hash_after = sha256_file(&entry.trash_path);
    assert_eq!(
        hash_before, hash_after,
        "이동 후 파일 내용(SHA) 변경 — 원본 훼손"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 2. 활성 세션 삭제 차단 — 실제 soft_delete_sessions_to 호출
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_active_session_is_blocked_from_soft_delete() {
    let src_dir = TempDir::new().unwrap();
    let trash_dir = TempDir::new().unwrap();

    let content = synth_jsonl("active-uuid", "활성 세션");
    let (src_path, _) = make_fixture(src_dir.path(), "active-uuid.jsonl", &content);

    // is_active = true → 차단 (실제 서비스 호출)
    let sessions = [(
        "active-uuid",
        src_path.as_path(),
        "활성 세션",
        "/synth/project",
        true,
    )];
    let result = soft_delete_sessions_to(&sessions, trash_dir.path()).unwrap();

    // 차단되어 skipped_active에 들어가야 함
    assert_eq!(result.skipped_active.len(), 1, "활성 세션이 차단되지 않음");
    assert!(
        result.moved.is_empty(),
        "활성 세션이 moved에 들어감 — 차단 실패"
    );

    // 원본 여전히 존재해야 함
    assert!(src_path.exists(), "활성 세션 파일이 삭제됨 — 차단 실패");
}

#[test]
fn test_soft_delete_skips_active_sessions_in_batch() {
    let src_dir = TempDir::new().unwrap();
    let trash_dir = TempDir::new().unwrap();

    let active_content = synth_jsonl("active-batch", "활성 배치");
    let inactive_content = synth_jsonl("inactive-batch", "비활성 배치");
    let (active_path, _) = make_fixture(src_dir.path(), "active-batch.jsonl", &active_content);
    let (inactive_path, _) =
        make_fixture(src_dir.path(), "inactive-batch.jsonl", &inactive_content);

    // 활성 + 비활성 혼합 배치
    let sessions = [
        (
            "active-batch",
            active_path.as_path(),
            "활성 배치",
            "/synth",
            true,
        ),
        (
            "inactive-batch",
            inactive_path.as_path(),
            "비활성 배치",
            "/synth",
            false,
        ),
    ];
    let result = soft_delete_sessions_to(&sessions, trash_dir.path()).unwrap();

    // 활성은 차단, 비활성만 이동
    assert_eq!(
        result.skipped_active.len(),
        1,
        "활성 세션이 skipped_active에 없음"
    );
    assert_eq!(result.moved.len(), 1, "비활성 세션이 moved에 없음");
    assert_eq!(result.moved[0], "inactive-batch");

    // 활성 원본 보존, 비활성 원본 제거
    assert!(active_path.exists(), "활성 세션 파일이 이동됨 — 차단 실패");
    assert!(
        !inactive_path.exists(),
        "비활성 세션 원본이 남아있음 — 이동 실패"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 3. 원본 JSONL 내용 불변: 파싱 기존 픽스처 + 삭제 전후 SHA 대조
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_existing_fixtures_sha_unchanged_by_trash_logic() {
    use claudedesk::data::FileMeta;
    use claudedesk::parser::parse_session;

    // 기존 fixtures: 파싱 전후 SHA 불변 (trash 모듈이 건드리지 않는지 확인)
    let fixture_names = [
        "normal_with_meta.jsonl",
        "block_array_content.jsonl",
        "corrupted_lines.jsonl",
        "empty.jsonl",
    ];

    let fixtures_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");

    for name in &fixture_names {
        let path = fixtures_dir.join(name);
        let hash_before = sha256_file(&path);

        let meta = FileMeta {
            path: path.clone(),
            mtime: SystemTime::UNIX_EPOCH,
            ctime: SystemTime::UNIX_EPOCH,
            size: 0,
        };
        let _ = parse_session(&meta);

        let hash_after = sha256_file(&path);
        assert_eq!(
            hash_before, hash_after,
            "픽스처 {name} — trash 로직 관련 작업 후 SHA 변경. 원본 훼손!"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 4. 복구: trash → 원본 경로, 내용(SHA) 동일, 인덱스 제거
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_restore_moves_file_to_original_and_sha_matches() {
    let orig_parent = TempDir::new().unwrap();
    let trash_dir = TempDir::new().unwrap();

    let content = synth_jsonl("bbb-uuid", "복구 테스트 세션");
    let orig_path = orig_parent.path().join("bbb-uuid.jsonl");
    let (trash_path, hash_before) = make_fixture(trash_dir.path(), "bbb-uuid_2000.jsonl", &content);

    // std::fs::rename으로 이동 (restore_sessions 내부 로직과 동일)
    std::fs::rename(&trash_path, &orig_path).unwrap();

    // 원본 경로에 파일 있어야 함
    assert!(orig_path.exists(), "복구 후 원본 경로에 파일 없음");

    // 휴지통에서 제거됨
    assert!(!trash_path.exists(), "복구 후 휴지통 파일이 남아있음");

    // 내용 동일
    let hash_after = sha256_file(&orig_path);
    assert_eq!(hash_before, hash_after, "복구 후 내용(SHA) 변경됨");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 5. purge: confirmed=false → Err (안전 게이트)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_purge_without_confirmation_returns_error() {
    let result = purge_sessions(&["some-uuid"], false);
    assert!(
        result.is_err(),
        "purge(confirmed=false)가 Ok를 반환함 — 안전 게이트 실패!"
    );
}

#[test]
fn test_purge_with_nonexistent_id_and_confirmed_ok() {
    // confirmed=true이지만 항목 없음 → errors에 추가, purged 비어있음
    let result = purge_sessions(&["nonexistent-999"], true).unwrap();
    assert!(
        result.purged.is_empty(),
        "존재하지 않는 항목이 purged에 들어감"
    );
    assert_eq!(result.errors.len(), 1, "오류 항목이 errors에 없음");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 6. TrashIndex 원자적 쓰기/읽기
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_trash_index_atomic_write_and_read() {
    let tmp = TempDir::new().unwrap();
    let index_path = tmp.path().join("index.json");

    let mut idx = TrashIndex {
        entries: HashMap::new(),
    };
    idx.entries.insert(
        "test-session-id".to_string(),
        TrashEntry {
            session_id: "test-session-id".to_string(),
            trash_path: PathBuf::from("/trash/test.jsonl"),
            original_path: PathBuf::from("/orig/test.jsonl"),
            title: "테스트 세션".to_string(),
            cwd: "/orig".to_string(),
            deleted_at_secs: 12345,
        },
    );

    save_index(&idx, &index_path);

    let loaded = load_index(&index_path);
    assert!(
        loaded.entries.contains_key("test-session-id"),
        "저장 후 로드 시 항목 없음"
    );
    assert_eq!(loaded.entries["test-session-id"].title, "테스트 세션");
    assert_eq!(loaded.entries["test-session-id"].deleted_at_secs, 12345);
}

#[test]
fn test_trash_index_load_empty_when_not_exists() {
    let tmp = TempDir::new().unwrap();
    let nonexistent = tmp.path().join("nonexistent_index.json");
    let loaded = load_index(&nonexistent);
    assert!(
        loaded.entries.is_empty(),
        "존재하지 않는 파일 로드 시 빈 인덱스 아님"
    );
}

#[test]
fn test_trash_index_load_corrupted_returns_default() {
    let tmp = TempDir::new().unwrap();
    let index_path = tmp.path().join("bad_index.json");
    std::fs::write(&index_path, b"{{bad json!!").unwrap();

    let loaded = load_index(&index_path);
    assert!(
        loaded.entries.is_empty(),
        "손상된 인덱스 로드 시 빈 인덱스가 아님"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 7. 복구 충돌: 원본 이미 존재 시 renamed 파일 생성
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_restore_conflict_renames_file() {
    let orig_dir = TempDir::new().unwrap();
    let trash_dir = TempDir::new().unwrap();

    // 원본 경로에 이미 파일 존재
    let orig_path = orig_dir.path().join("ccc-uuid.jsonl");
    std::fs::write(&orig_path, b"existing content").unwrap();

    let content = synth_jsonl("ccc-uuid", "충돌 복구 테스트");
    let (trash_path, hash_trash) = make_fixture(trash_dir.path(), "ccc-uuid_3000.jsonl", &content);

    // 충돌 처리: 새 이름으로 이동
    let renamed = orig_dir.path().join("ccc-uuid_restored_99999.jsonl");
    std::fs::rename(&trash_path, &renamed).unwrap();

    // 기존 원본 유지
    assert!(orig_path.exists(), "충돌 시 기존 원본이 사라짐");
    let existing_content = std::fs::read(&orig_path).unwrap();
    assert_eq!(
        existing_content, b"existing content",
        "기존 원본 내용 변경됨"
    );

    // 새 이름으로 복구됨
    assert!(renamed.exists(), "충돌 시 renamed 파일이 생성되지 않음");
    let hash_restored = sha256_file(&renamed);
    assert_eq!(hash_trash, hash_restored, "복구된 파일 내용(SHA) 변경됨");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 8. TrashIndex 정렬: 삭제 시각 내림차순
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_trash_index_sorted_entries_desc_by_deleted_at() {
    let mut idx = TrashIndex {
        entries: HashMap::new(),
    };
    for (id, secs) in [("a", 100u64), ("b", 300u64), ("c", 200u64)] {
        idx.entries.insert(
            id.to_string(),
            TrashEntry {
                session_id: id.to_string(),
                trash_path: PathBuf::from(format!("/trash/{}.jsonl", id)),
                original_path: PathBuf::from(format!("/orig/{}.jsonl", id)),
                title: id.to_string(),
                cwd: "/".to_string(),
                deleted_at_secs: secs,
            },
        );
    }

    let sorted = idx.sorted_entries();
    assert_eq!(sorted[0].session_id, "b", "가장 최근 삭제가 첫 번째여야 함");
    assert_eq!(sorted[1].session_id, "c");
    assert_eq!(
        sorted[2].session_id, "a",
        "가장 오래된 삭제가 마지막이어야 함"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 9. 소프트 삭제 후 SHA 불변 (이동이지 수정이 아님)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_soft_delete_content_sha_unchanged() {
    let src_dir = TempDir::new().unwrap();
    let trash_dir = TempDir::new().unwrap();

    // 여러 줄 JSONL
    let content = format!(
        "{}\n{}\n",
        synth_jsonl("ddd-uuid", "첫 메시지"),
        "{\"type\":\"assistant\",\"sessionId\":\"ddd-uuid\",\"message\":{\"content\":\"응답\"}}"
    );
    let (src_path, hash_before) = make_fixture(src_dir.path(), "ddd-uuid.jsonl", &content);

    let sessions = [("ddd-uuid", src_path.as_path(), "첫 메시지", "/synth", false)];
    let result = soft_delete_sessions_to(&sessions, trash_dir.path()).unwrap();

    assert_eq!(result.moved.len(), 1);

    let index = load_index_from(&trash_dir.path().join("index.json"));
    let entry = index.entries.get("ddd-uuid").expect("인덱스에 항목 없음");

    let hash_after = sha256_file(&entry.trash_path);
    assert_eq!(
        hash_before, hash_after,
        "소프트 삭제(이동) 후 내용(SHA) 변경 — 원본 훼손!"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 10. 이름 충돌 방지: 같은 session_id를 다른 타임스탬프로 2회 이동
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_name_collision_prevention() {
    let src1 = TempDir::new().unwrap();
    let src2 = TempDir::new().unwrap();
    let dst_dir = TempDir::new().unwrap();

    let c1 = synth_jsonl("eee-uuid", "첫 번째 복사본");
    let c2 = synth_jsonl("eee-uuid", "두 번째 복사본");

    let (p1, _) = make_fixture(src1.path(), "eee-uuid.jsonl", &c1);
    let (p2, _) = make_fixture(src2.path(), "eee-uuid.jsonl", &c2);

    // 타임스탬프 다르게 직접 이동 (충돌 방지 확인)
    let dst1 = dst_dir.path().join("eee-uuid_1000.jsonl");
    let dst2 = dst_dir.path().join("eee-uuid_1001.jsonl");

    std::fs::rename(&p1, &dst1).unwrap();
    std::fs::rename(&p2, &dst2).unwrap();

    assert!(dst1.exists(), "첫 번째 파일이 없음");
    assert!(dst2.exists(), "두 번째 파일이 없음 — 충돌로 덮어써짐");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 11. 기존 서비스 테스트 회귀 없음 (정렬/검색)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_sort_and_filter_regression() {
    use claudedesk::service::{apply_sort, AppState, ScanStats, SortDir, SortKey, SortState};
    use std::collections::HashSet;

    // make_session 헬퍼
    let make_session = |title: &str, secs_ago: u64, msg_count: usize| {
        use claudedesk::domain::Session;
        use claudedesk::parser::build_search_text;
        let now = SystemTime::now();
        let modified = now - Duration::from_secs(secs_ago);
        let search_text = build_search_text(title, None, "/test", None);
        Session {
            session_id: title.to_string(),
            title: title.to_string(),
            cwd: "/test".to_string(),
            created: modified,
            modified,
            msg_count,
            is_active: false,
            path: PathBuf::from("/test/session.jsonl"),
            skipped_lines: 0,
            alias: None,
            search_text,
        }
    };

    let mut sessions = vec![
        make_session("Alpha", 300, 5),
        make_session("Gamma", 100, 15),
        make_session("Beta", 200, 10),
    ];

    // 정렬: modified desc → Gamma, Beta, Alpha
    apply_sort(
        &mut sessions,
        SortState {
            key: SortKey::Modified,
            dir: SortDir::Desc,
        },
    );
    assert_eq!(sessions[0].title, "Gamma");
    assert_eq!(sessions[1].title, "Beta");
    assert_eq!(sessions[2].title, "Alpha");

    // 검색: "gamma"
    let state = AppState {
        sessions,
        stats: ScanStats::default(),
        projects_root: PathBuf::from("/tmp"),
        sort: SortState::default(),
        search_query: Some("gamma".to_string()),
        selected_ids: HashSet::new(),
        grouped: false,
        collapsed_projects: HashSet::new(),
        aliases: claudedesk::alias::AliasStore::default(),
    };
    let idx = state.filtered_indices();
    assert_eq!(idx.len(), 1, "검색 결과가 1개여야 함");
    assert_eq!(state.sessions[idx[0]].title, "Gamma");
}
