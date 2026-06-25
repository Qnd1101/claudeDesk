use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::config::is_subagent_path;

/// 파일 스탯 메타
#[derive(Debug, Clone)]
pub struct FileMeta {
    pub path: PathBuf,
    pub mtime: SystemTime,
    pub ctime: SystemTime,
    /// 파일 크기. M2 대용량 세션 가드/진단에서 사용 예정.
    #[allow(dead_code)]
    pub size: u64,
}

/// projects_root 하위 모든 *.jsonl 파일 수집 (subagents/ 제외, 최상위만)
pub fn discover_sessions(projects_root: &Path) -> Result<Vec<FileMeta>> {
    let mut sessions = Vec::new();

    if !projects_root.exists() {
        return Ok(sessions);
    }

    // 각 프로젝트 폴더 순회
    let read_dir = match std::fs::read_dir(projects_root) {
        Ok(rd) => rd,
        Err(e) => {
            eprintln!("경로 읽기 실패 {}: {}", projects_root.display(), e);
            return Ok(sessions);
        }
    };

    for entry in read_dir.flatten() {
        let project_path = entry.path();
        if !project_path.is_dir() {
            continue;
        }

        // 프로젝트 폴더 최상위 *.jsonl만 (subagents/ 제외)
        let inner = match std::fs::read_dir(&project_path) {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        for file_entry in inner.flatten() {
            let file_path = file_entry.path();

            // subagents/ 하위 제외
            if is_subagent_path(&file_path) {
                continue;
            }

            // 디렉토리면 스킵 (sessionId 디렉토리는 subagent 컨테이너)
            if file_path.is_dir() {
                continue;
            }

            // .jsonl 확장자만
            if file_path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            if let Ok(meta) = stat_file(&file_path) {
                sessions.push(meta);
            }
        }
    }

    Ok(sessions)
}

/// 파일 stat (mtime, ctime, size)
pub fn stat_file(path: &Path) -> Result<FileMeta> {
    let meta = std::fs::metadata(path)?;
    let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    // ctime: Unix에서는 created() 미지원, Windows는 created() 사용
    let ctime = meta.created().unwrap_or(mtime);
    Ok(FileMeta {
        path: path.to_path_buf(),
        mtime,
        ctime,
        size: meta.len(),
    })
}
