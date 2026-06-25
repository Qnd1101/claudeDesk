/// FR-04 소프트 삭제 + FR-11 휴지통/복구/영구삭제
///
/// 안전 원칙:
/// - 원본 JSONL 파일을 이동(rename)만 한다 — 내용 수정·쓰기 없음
/// - 활성 세션(mtime 근접) 삭제 차단
/// - 영구삭제(purge)는 명시적 confirm 플래그 없으면 실행 안 함
/// - 메타(index) 쓰기는 temp+rename 원자적 방식
use anyhow::{Context, Result};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ── 경로 헬퍼 ─────────────────────────────────────────────────────────────────

/// ~/.claude/claudedesk/trash/ 경로
pub fn trash_dir() -> Result<PathBuf> {
    let base = BaseDirs::new().context("홈 디렉토리를 찾을 수 없습니다")?;
    Ok(base
        .home_dir()
        .join(".claude")
        .join("claudedesk")
        .join("trash"))
}

/// ~/.claude/claudedesk/trash/index.json 경로
pub fn trash_index_path() -> Result<PathBuf> {
    Ok(trash_dir()?.join("index.json"))
}

// ── 메타 모델 ─────────────────────────────────────────────────────────────────

/// 휴지통 항목 (index.json에 저장)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashEntry {
    /// 세션 ID (UUID, 파일명)
    pub session_id: String,
    /// 휴지통 내 파일 경로 (절대)
    pub trash_path: PathBuf,
    /// 원본 절대 경로 (복구 대상)
    pub original_path: PathBuf,
    /// 세션 제목 (표시용, 원본 불변이므로 스냅샷)
    pub title: String,
    /// 원본 cwd (표시용)
    pub cwd: String,
    /// 삭제 시각 (Unix 초)
    pub deleted_at_secs: u64,
}

impl TrashEntry {
    /// 삭제 시각을 SystemTime으로 변환
    pub fn deleted_at(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(self.deleted_at_secs)
    }
}

/// 휴지통 인덱스 (session_id → TrashEntry)
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TrashIndex {
    pub entries: HashMap<String, TrashEntry>,
}

impl TrashIndex {
    /// index.json 로드 (없으면 빈 인덱스 반환, 손상 시 graceful)
    pub fn load() -> Self {
        let path = match trash_index_path() {
            Ok(p) => p,
            Err(_) => return TrashIndex::default(),
        };
        if !path.exists() {
            return TrashIndex::default();
        }
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => TrashIndex::default(),
        }
    }

    /// index.json 원자적 저장 (temp+rename)
    /// nit §4-3: tmp를 `index.json.tmp` (with_file_name) 사용 — with_extension은 `.tmp`가 됨
    pub fn save(&self) -> Result<()> {
        let index_path = trash_index_path()?;
        // 부모 디렉토리 보장
        if let Some(parent) = index_path.parent() {
            std::fs::create_dir_all(parent).context("trash 디렉토리 생성 실패")?;
        }

        let json = serde_json::to_string_pretty(self).context("index 직렬화 실패")?;

        // temp 파일에 쓴 뒤 rename (원자적)
        let tmp_path = index_path
            .parent()
            .unwrap_or(Path::new("."))
            .join("index.json.tmp");
        std::fs::write(&tmp_path, &json).context("index temp 파일 쓰기 실패")?;
        std::fs::rename(&tmp_path, &index_path).context("index rename 실패")?;

        Ok(())
    }

    /// 정렬된 항목 목록 (삭제 시각 내림차순)
    pub fn sorted_entries(&self) -> Vec<&TrashEntry> {
        let mut entries: Vec<&TrashEntry> = self.entries.values().collect();
        entries.sort_by_key(|e| std::cmp::Reverse(e.deleted_at_secs));
        entries
    }
}

// ── 소프트 삭제 ───────────────────────────────────────────────────────────────

/// 소프트 삭제 결과
#[derive(Debug)]
pub struct SoftDeleteResult {
    /// 성공적으로 이동된 세션 ID 목록
    pub moved: Vec<String>,
    /// 활성 세션이라 스킵된 (session_id, 이유)
    pub skipped_active: Vec<(String, String)>,
    /// 기타 오류 (session_id, 오류 메시지)
    pub errors: Vec<(String, String)>,
}

/// 다중 세션 소프트 삭제 (공개 API — 실제 trash_dir() 사용)
///
/// - 각 세션 파일을 trash_dir()로 원자적으로 이동 (rename)
/// - 이름 충돌 방지: `<session_id>_<timestamp>.jsonl`
/// - 활성 세션(is_active=true)은 차단
/// - 인덱스에 원자적으로 기록
pub fn soft_delete_sessions(
    sessions: &[(&str, &Path, &str, &str, bool)], // (session_id, path, title, cwd, is_active)
) -> Result<SoftDeleteResult> {
    let trash = trash_dir()?;
    soft_delete_sessions_to(sessions, &trash)
}

/// 소프트 삭제 내부 구현 — trash 경로를 파라미터로 받아 테스트 주입 가능
pub fn soft_delete_sessions_to(
    sessions: &[(&str, &Path, &str, &str, bool)],
    trash_dir_path: &Path,
) -> Result<SoftDeleteResult> {
    std::fs::create_dir_all(trash_dir_path).context("trash 디렉토리 생성 실패")?;

    let index_path = trash_dir_path.join("index.json");
    let mut index = load_index_from(&index_path);
    let mut result = SoftDeleteResult {
        moved: vec![],
        skipped_active: vec![],
        errors: vec![],
    };

    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    for (session_id, path, title, cwd, is_active) in sessions {
        // 활성 세션 차단
        if *is_active {
            result.skipped_active.push((
                session_id.to_string(),
                format!("활성 세션(최근 수정됨): {}", title),
            ));
            continue;
        }

        // 원본 파일 존재 확인
        if !path.exists() {
            result.errors.push((
                session_id.to_string(),
                format!("파일 없음: {}", path.display()),
            ));
            continue;
        }

        // 충돌 방지 파일명: <session_id>_<now_secs>.jsonl
        let trash_filename = format!("{}_{}.jsonl", session_id, now_secs);
        let trash_path = trash_dir_path.join(&trash_filename);

        // 파일 이동 (원자적 rename; 크로스 디바이스 시 copy+remove 폴백)
        match atomic_move(path, &trash_path) {
            Ok(()) => {
                let entry = TrashEntry {
                    session_id: session_id.to_string(),
                    trash_path: trash_path.clone(),
                    original_path: path.to_path_buf(),
                    title: title.to_string(),
                    cwd: cwd.to_string(),
                    deleted_at_secs: now_secs,
                };
                index.entries.insert(session_id.to_string(), entry);
                result.moved.push(session_id.to_string());
            }
            Err(e) => {
                result
                    .errors
                    .push((session_id.to_string(), format!("이동 실패: {}", e)));
            }
        }
    }

    // 변경 사항이 있으면 인덱스 저장
    if !result.moved.is_empty() {
        save_index_to(&index, &index_path)?;
    }

    Ok(result)
}

/// §4-1: 파일 이동 — rename 시도 → 실패 시 copy+remove 폴백 (크로스 디바이스)
/// copy 성공 후 remove_file 실패 시 dst 정리(롤백) 후 Err 반환
fn atomic_move(src: &Path, dst: &Path) -> Result<()> {
    match std::fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(_) => {
            // 크로스 디바이스(Windows 드라이브 간 등) 폴백
            std::fs::copy(src, dst).context("파일 복사 실패")?;
            if let Err(e) = std::fs::remove_file(src) {
                // 원본 제거 실패 → 고아 복사본 정리 후 Err
                let _ = std::fs::remove_file(dst); // 롤백 (best-effort)
                return Err(e).context("원본 제거 실패 (복사본 롤백 시도)");
            }
            Ok(())
        }
    }
}

// ── 인덱스 I/O 헬퍼 (경로 주입 가능) ────────────────────────────────────────

/// 지정 경로에서 TrashIndex 로드 (없으면 빈 인덱스, 손상 시 graceful)
pub fn load_index_from(path: &Path) -> TrashIndex {
    if !path.exists() {
        return TrashIndex::default();
    }
    match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => TrashIndex::default(),
    }
}

/// 지정 경로에 TrashIndex 원자적 저장 (temp+rename)
pub fn save_index_to(index: &TrashIndex, index_path: &Path) -> Result<()> {
    if let Some(parent) = index_path.parent() {
        std::fs::create_dir_all(parent).context("인덱스 디렉토리 생성 실패")?;
    }
    let json = serde_json::to_string_pretty(index).context("index 직렬화 실패")?;
    let tmp_path = index_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("index.json.tmp");
    std::fs::write(&tmp_path, &json).context("index temp 파일 쓰기 실패")?;
    std::fs::rename(&tmp_path, index_path).context("index rename 실패")?;
    Ok(())
}

// ── 복구 ──────────────────────────────────────────────────────────────────────

/// 복구 결과
#[derive(Debug)]
pub struct RestoreResult {
    pub restored: Vec<String>,
    pub errors: Vec<(String, String)>,
}

/// 선택된 세션들을 원본 경로로 복구
///
/// 원본 경로에 이미 파일이 있으면 충돌 처리:
/// 원본 경로에 `_restored_<timestamp>` 접미사 추가
/// §4-2: 고아 항목 제거 포함 인덱스 변경이 있으면 반드시 save
pub fn restore_sessions(session_ids: &[&str]) -> Result<RestoreResult> {
    let mut index = TrashIndex::load();
    let mut result = RestoreResult {
        restored: vec![],
        errors: vec![],
    };
    let mut index_dirty = false; // 고아 제거 포함 변경 추적

    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    for &sid in session_ids {
        let entry = match index.entries.get(sid) {
            Some(e) => e.clone(),
            None => {
                result
                    .errors
                    .push((sid.to_string(), "휴지통에서 찾을 수 없음".to_string()));
                continue;
            }
        };

        // trash 파일 존재 확인
        if !entry.trash_path.exists() {
            result.errors.push((
                sid.to_string(),
                format!("휴지통 파일 없음: {}", entry.trash_path.display()),
            ));
            index.entries.remove(sid); // 고아 항목 정리
            index_dirty = true; // §4-2: 고아 제거도 변경
            continue;
        }

        // 원본 경로 결정 (충돌 처리)
        let target = if entry.original_path.exists() {
            // 충돌: 접미사 추가
            let stem = entry
                .original_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(sid);
            let new_name = format!("{}_restored_{}.jsonl", stem, now_secs);
            entry
                .original_path
                .parent()
                .unwrap_or(Path::new("."))
                .join(new_name)
        } else {
            // 원본 경로 부모 디렉토리 생성 보장
            if let Some(parent) = entry.original_path.parent() {
                if !parent.exists() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        result
                            .errors
                            .push((sid.to_string(), format!("원본 디렉토리 생성 실패: {}", e)));
                        continue;
                    }
                }
            }
            entry.original_path.clone()
        };

        match atomic_move(&entry.trash_path, &target) {
            Ok(()) => {
                index.entries.remove(sid);
                index_dirty = true;
                result.restored.push(sid.to_string());
            }
            Err(e) => {
                result
                    .errors
                    .push((sid.to_string(), format!("복구 이동 실패: {}", e)));
            }
        }
    }

    // §4-2: 복구 + 고아 제거 포함 변경이 있으면 저장
    if index_dirty {
        index.save()?;
    }

    Ok(result)
}

// ── 영구삭제 ──────────────────────────────────────────────────────────────────

/// 영구삭제 결과
#[derive(Debug)]
pub struct PurgeResult {
    pub purged: Vec<String>,
    pub errors: Vec<(String, String)>,
}

/// 영구삭제 — 반드시 `confirmed: true` 로 호출해야 실행
///
/// confirmed=false이면 Err 반환 (안전 게이트)
/// 자동 정리·보관기간 만료 자동 purge는 이 함수로 처리하지 않음
pub fn purge_sessions(session_ids: &[&str], confirmed: bool) -> Result<PurgeResult> {
    if !confirmed {
        anyhow::bail!("purge는 명시적 확인(confirmed=true) 없이 실행할 수 없습니다 (안전 게이트)");
    }

    let mut index = TrashIndex::load();
    let mut result = PurgeResult {
        purged: vec![],
        errors: vec![],
    };

    for &sid in session_ids {
        let entry = match index.entries.get(sid) {
            Some(e) => e.clone(),
            None => {
                result
                    .errors
                    .push((sid.to_string(), "휴지통에서 찾을 수 없음".to_string()));
                continue;
            }
        };

        // 파일이 없어도 인덱스에서 제거 (고아 항목)
        if entry.trash_path.exists() {
            if let Err(e) = std::fs::remove_file(&entry.trash_path) {
                result
                    .errors
                    .push((sid.to_string(), format!("파일 삭제 실패: {}", e)));
                continue;
            }
        }

        index.entries.remove(sid);
        result.purged.push(sid.to_string());
    }

    if !result.purged.is_empty() {
        index.save()?;
    }

    Ok(result)
}

// ── 테스트 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// 임시 디렉토리에 합성 JSONL 파일 생성, SHA-256 반환
    pub fn make_fixture(dir: &Path, filename: &str, content: &str) -> (PathBuf, String) {
        use sha2::{Digest, Sha256};
        let path = dir.join(filename);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        let hash = hex::encode(Sha256::digest(content.as_bytes()));
        (path, hash)
    }

    /// 파일 SHA-256 계산
    pub fn sha256_of_file(path: &Path) -> String {
        use sha2::{Digest, Sha256};
        let data = std::fs::read(path).unwrap();
        hex::encode(Sha256::digest(&data))
    }

    // ── 인덱스 원자성 ──────────────────────────────────────────────────────

    #[test]
    fn test_index_atomic_write_and_read() {
        let tmp = TempDir::new().unwrap();
        let index_path = tmp.path().join("index.json");

        let mut idx = TrashIndex::default();
        idx.entries.insert(
            "test-id".to_string(),
            TrashEntry {
                session_id: "test-id".to_string(),
                trash_path: PathBuf::from("/tmp/test.jsonl"),
                original_path: PathBuf::from("/orig/test.jsonl"),
                title: "테스트 세션".to_string(),
                cwd: "/orig".to_string(),
                deleted_at_secs: 1000,
            },
        );

        save_index_to(&idx, &index_path).unwrap();
        let loaded = load_index_from(&index_path);

        assert!(loaded.entries.contains_key("test-id"));
        assert_eq!(loaded.entries["test-id"].title, "테스트 세션");
    }

    // ── 소프트 삭제: 파일 이동 + 원본 없음 ───────────────────────────────

    #[test]
    fn test_soft_delete_moves_file_and_original_gone() {
        let src_dir = TempDir::new().unwrap();
        let trash_target = TempDir::new().unwrap();

        let content =
            r#"{"type":"user","message":{"content":"테스트"},"sessionId":"aaa","cwd":"/proj"}"#;
        let (src_path, hash_before) = make_fixture(src_dir.path(), "aaa.jsonl", content);

        let trash_path = trash_target.path().join("aaa_1000.jsonl");
        atomic_move(&src_path, &trash_path).unwrap();

        assert!(!src_path.exists(), "원본이 아직 남아있음 — 이동 실패");

        let hash_after = sha256_of_file(&trash_path);
        assert_eq!(hash_before, hash_after, "이동 후 내용(SHA) 변경됨");
    }

    // ── §6-1·6-2: 활성 세션 차단 — 실제 soft_delete_sessions_to 호출 ─────
    // 활성+비활성 혼합 슬라이스를 넘겨 ① 활성은 skipped_active ② 활성 원본 파일 그대로
    // ③ 비활성만 trash로 이동되는지 단언

    #[test]
    fn test_active_session_blocked_real_service_call() {
        let src_dir = TempDir::new().unwrap();
        let trash_dir = TempDir::new().unwrap();

        // 비활성 세션 파일
        let inactive_content = r#"{"type":"user","message":{"content":"비활성"}}"#;
        let (inactive_path, inactive_hash) =
            make_fixture(src_dir.path(), "inactive.jsonl", inactive_content);

        // 활성 세션 파일
        let active_content = r#"{"type":"user","message":{"content":"활성"}}"#;
        let (active_path, _) = make_fixture(src_dir.path(), "active.jsonl", active_content);

        // 혼합 슬라이스: is_active=true(활성) + is_active=false(비활성)
        let sessions = [
            (
                "active-id",
                active_path.as_path(),
                "활성 세션",
                "/proj",
                true,
            ),
            (
                "inactive-id",
                inactive_path.as_path(),
                "비활성 세션",
                "/proj",
                false,
            ),
        ];

        let result = soft_delete_sessions_to(&sessions, trash_dir.path()).unwrap();

        // ① 활성은 skipped_active에 들어가야 함
        assert_eq!(
            result.skipped_active.len(),
            1,
            "활성 세션이 skipped_active에 없음"
        );
        assert_eq!(result.skipped_active[0].0, "active-id");

        // ② 활성 원본 파일은 그대로 존재해야 함 (이동 안 됨)
        assert!(
            active_path.exists(),
            "활성 세션 원본 파일이 삭제됨 — 차단 실패!"
        );

        // ③ 비활성만 trash로 이동됨
        assert_eq!(result.moved.len(), 1, "비활성 세션이 moved에 없음");
        assert_eq!(result.moved[0], "inactive-id");
        assert!(
            !inactive_path.exists(),
            "비활성 세션 원본이 남아있음 — 이동 실패"
        );

        // 이동된 파일 SHA 동일 (내용 불변)
        let index = load_index_from(&trash_dir.path().join("index.json"));
        let entry = index
            .entries
            .get("inactive-id")
            .expect("인덱스에 항목 없음");
        let moved_hash = sha256_of_file(&entry.trash_path);
        assert_eq!(inactive_hash, moved_hash, "이동 후 내용(SHA) 변경됨");
    }

    // ── purge 확인 게이트 ─────────────────────────────────────────────────

    #[test]
    fn test_purge_requires_confirmed_true() {
        let result = purge_sessions(&["nonexistent-id"], false);
        assert!(
            result.is_err(),
            "confirmed=false 시 purge가 Ok를 반환함 — 안전 게이트 실패"
        );
    }

    // ── 복구: trash → 원본 경로 ──────────────────────────────────────────

    #[test]
    fn test_restore_moves_file_back() {
        let orig_dir = TempDir::new().unwrap();
        let trash_target = TempDir::new().unwrap();

        let content = r#"{"type":"user","message":{"content":"복구 테스트"}}"#;
        let (trash_path, hash_before) =
            make_fixture(trash_target.path(), "bbb_1000.jsonl", content);

        let orig_path = orig_dir.path().join("bbb.jsonl");

        atomic_move(&trash_path, &orig_path).unwrap();

        assert!(orig_path.exists(), "복구 후 원본 경로에 파일 없음");
        assert!(!trash_path.exists(), "복구 후 휴지통 파일이 남아있음");

        let hash_after = sha256_of_file(&orig_path);
        assert_eq!(hash_before, hash_after, "복구 후 내용(SHA) 변경됨");
    }

    // ── 원본 내용 불변: 삭제 시뮬레이션 전후 ────────────────────────────

    #[test]
    fn test_original_content_unchanged_after_soft_delete_simulation() {
        let src_dir = TempDir::new().unwrap();
        let trash_dir_tmp = TempDir::new().unwrap();

        let content = "{\"type\":\"user\",\"message\":{\"content\":\"불변 테스트\"}}\n";
        let (src_path, hash_before) = make_fixture(src_dir.path(), "ccc.jsonl", content);

        let trash_path = trash_dir_tmp.path().join("ccc_9999.jsonl");

        atomic_move(&src_path, &trash_path).unwrap();

        let hash_after = sha256_of_file(&trash_path);
        assert_eq!(
            hash_before, hash_after,
            "소프트 삭제 후 내용(SHA) 변경 — 원본 훼손!"
        );
    }

    // ── 복구 충돌 처리: 원본 이미 존재 ──────────────────────────────────

    #[test]
    fn test_restore_conflict_creates_renamed_file() {
        let orig_dir = TempDir::new().unwrap();

        let orig_path = orig_dir.path().join("ddd.jsonl");
        std::fs::write(&orig_path, b"original file").unwrap();

        let trash_target = TempDir::new().unwrap();
        let content = r#"{"type":"user","message":{"content":"충돌 복구"}}"#;
        let (trash_path, hash_before) =
            make_fixture(trash_target.path(), "ddd_2000.jsonl", content);

        let now_secs = 99999u64;
        let stem = "ddd";
        let new_name = format!("{}_restored_{}.jsonl", stem, now_secs);
        let target = orig_dir.path().join(&new_name);

        atomic_move(&trash_path, &target).unwrap();

        assert!(target.exists(), "충돌 시 renamed 파일이 생성되지 않음");
        assert!(orig_path.exists(), "충돌 시 기존 원본이 덮어써짐");

        let hash_after = sha256_of_file(&target);
        assert_eq!(hash_before, hash_after, "복구된 파일 내용 변경됨");
    }

    // ── 인덱스 정렬: 삭제 시각 내림차순 ──────────────────────────────────

    #[test]
    fn test_sorted_entries_desc_by_deleted_at() {
        let mut idx = TrashIndex::default();
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
        assert_eq!(sorted[0].session_id, "b");
        assert_eq!(sorted[1].session_id, "c");
        assert_eq!(sorted[2].session_id, "a");
    }
}
