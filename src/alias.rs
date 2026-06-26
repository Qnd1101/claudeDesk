//! FR-06 별칭 사이드카. 원본 JSONL 불변(§5.3), temp+rename 원자적 write(부록 D).
use anyhow::{Context, Result};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ── 경로 헬퍼 ─────────────────────────────────────────────────────────────────

/// ~/.claude/claudedesk/meta.json 경로 (trash/index.json의 형제)
pub fn alias_meta_path() -> Result<PathBuf> {
    let base = BaseDirs::new().context("홈 디렉토리를 찾을 수 없습니다")?;
    Ok(base
        .home_dir()
        .join(".claude")
        .join("claudedesk")
        .join("meta.json"))
}

// ── 메타 모델 ─────────────────────────────────────────────────────────────────

/// 세션별 부가 메타 (M3=별칭만; pinned/tags는 향후 확장 슬롯)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AliasEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    // 향후: pinned/tags — 지금은 미직렬화. 알 수 없는 필드는 serde가 무시(방어적).
}

/// session_id → AliasEntry
#[derive(Debug, Serialize, Deserialize)]
pub struct AliasStore {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub entries: HashMap<String, AliasEntry>,
}

impl Default for AliasStore {
    fn default() -> Self {
        AliasStore {
            version: 1,
            entries: HashMap::new(),
        }
    }
}

fn default_version() -> u32 {
    1
}

impl AliasStore {
    /// 로드 (없으면 빈 store, 손상 시 graceful default — trash.rs::load 선례)
    pub fn load() -> Self {
        match alias_meta_path() {
            Ok(p) => load_alias_from(&p),
            Err(_) => AliasStore::default(),
        }
    }

    /// 원자적 저장 (temp+rename)
    #[allow(dead_code)]
    pub fn save(&self) -> Result<()> {
        let path = alias_meta_path()?;
        save_alias_to(self, &path)
    }

    /// 별칭 조회 (빈 문자열은 None 취급)
    pub fn get(&self, session_id: &str) -> Option<&str> {
        self.entries
            .get(session_id)
            .and_then(|e| e.alias.as_deref())
            .filter(|s| !s.is_empty())
    }

    /// 별칭 설정/삭제. 트림 후 빈 문자열이면 키 제거 (빈 값 잔류 방지).
    #[allow(dead_code)]
    pub fn set(&mut self, session_id: &str, alias: &str) {
        let trimmed = alias.trim();
        if trimmed.is_empty() {
            self.entries.remove(session_id);
        } else {
            self.entries.insert(
                session_id.to_string(),
                AliasEntry {
                    alias: Some(trimmed.to_string()),
                },
            );
        }
    }
}

// ── I/O 헬퍼 (경로 주입 가능 — 테스트 격리) ──────────────────────────────────

/// 지정 경로에서 AliasStore 로드 (없으면 빈 store, 손상 시 graceful — trash::load_index_from 대응)
pub fn load_alias_from(path: &Path) -> AliasStore {
    if !path.exists() {
        return AliasStore::default();
    }
    match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => AliasStore::default(),
    }
}

/// 지정 경로에 AliasStore 원자적 저장 (temp+rename — trash::save_index_to 대응)
#[allow(dead_code)]
pub fn save_alias_to(store: &AliasStore, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("meta 디렉토리 생성 실패")?;
    }
    let json = serde_json::to_string_pretty(store).context("meta 직렬화 실패")?;
    let tmp = path
        .parent()
        .unwrap_or(Path::new("."))
        .join("meta.json.tmp");
    std::fs::write(&tmp, &json).context("meta temp 쓰기 실패")?;
    std::fs::rename(&tmp, path).context("meta rename 실패")?;
    Ok(())
}

// ── 테스트 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// §5.1 roundtrip: set → save_alias_to → load_alias_from, 값 일치
    #[test]
    fn test_alias_store_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("meta.json");

        let mut store = AliasStore::default();
        store.set("session-abc", "결제 모듈 리팩터");

        save_alias_to(&store, &path).unwrap();
        let loaded = load_alias_from(&path);

        assert_eq!(
            loaded.get("session-abc"),
            Some("결제 모듈 리팩터"),
            "roundtrip 후 별칭 값 불일치"
        );
        assert_eq!(loaded.version, 1, "버전 필드 불일치");
    }

    /// §5.1 set_empty_removes_key: 빈/공백 별칭 set → entries에서 키 제거
    #[test]
    fn test_alias_set_empty_removes_key() {
        let mut store = AliasStore::default();
        store.set("session-abc", "some alias");
        assert!(store.entries.contains_key("session-abc"));

        store.set("session-abc", "");
        assert!(
            !store.entries.contains_key("session-abc"),
            "빈 별칭 set 후 키가 남아있음"
        );

        let mut store2 = AliasStore::default();
        store2.set("session-def", "whitespace alias");
        store2.set("session-def", "   ");
        assert!(
            !store2.entries.contains_key("session-def"),
            "공백 별칭 set 후 키가 남아있음"
        );
    }

    /// §5.1 get_filters_empty: 빈 문자열 alias는 get None
    #[test]
    fn test_alias_get_filters_empty() {
        let mut store = AliasStore::default();
        // 파일에서 직접 로드 시 빈 문자열이 들어올 수 있음 — get은 None 반환해야 함
        store.entries.insert(
            "session-empty".to_string(),
            AliasEntry {
                alias: Some(String::new()),
            },
        );
        assert_eq!(
            store.get("session-empty"),
            None,
            "빈 문자열 alias가 Some으로 반환됨"
        );
    }

    /// §5.1 load_missing_default: 없는 경로 → 빈 store
    #[test]
    fn test_alias_load_missing_file_is_default() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nonexistent_meta.json");
        let store = load_alias_from(&path);
        assert!(
            store.entries.is_empty(),
            "없는 파일 로드 시 빈 store가 아님"
        );
    }

    /// §5.1 load_corrupted_graceful: 깨진 JSON → 빈 store (크래시 없음)
    #[test]
    fn test_alias_load_corrupted_graceful() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("meta.json");
        std::fs::write(&path, b"{{bad json!!").unwrap();

        let store = load_alias_from(&path);
        assert!(
            store.entries.is_empty(),
            "손상된 JSON 로드 시 빈 store가 아님 — 크래시 위험"
        );
    }

    /// §5.1 save_atomic_temp_rename: 저장 후 meta.json.tmp 잔류 없음
    #[test]
    fn test_alias_save_atomic_temp_rename() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("meta.json");

        let mut store = AliasStore::default();
        store.set("session-xyz", "테스트 별칭");
        save_alias_to(&store, &path).unwrap();

        let tmp_path = tmp.path().join("meta.json.tmp");
        assert!(
            !tmp_path.exists(),
            "저장 후 meta.json.tmp 잔류 — rename 실패"
        );
        assert!(path.exists(), "저장 후 meta.json이 없음");
    }
}
