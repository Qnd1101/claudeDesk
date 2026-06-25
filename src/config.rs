use anyhow::{Context, Result};
use directories::BaseDirs;
use std::path::{Path, PathBuf};

/// 런타임 설정 (config.toml 없으면 기본값, CLI 인자가 우선)
#[derive(Debug, Clone)]
pub struct Config {
    /// ~/.claude/projects 또는 CLAUDEDESK_ROOT / --root 오버라이드
    pub projects_root: PathBuf,
    /// active 세션 판정 임계(초)
    pub active_window_secs: u64,
    /// 로그 레벨 상세 여부
    pub verbose: bool,
}

impl Config {
    pub fn load(custom_root: Option<String>, verbose: bool) -> Result<Self> {
        let projects_root = if let Some(root) = custom_root {
            expand_tilde(&root)
        } else {
            default_projects_root()?
        };

        Ok(Config {
            projects_root,
            active_window_secs: 90,
            verbose,
        })
    }
}

/// `~` 를 홈 디렉토리로 확장
pub fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with('~') {
        if let Some(base) = BaseDirs::new() {
            let home = base.home_dir();
            let rest = path.trim_start_matches('~').trim_start_matches('/');
            // Windows: 슬래시도 처리
            let rest = rest.trim_start_matches('\\');
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

/// 기본 projects 루트: ~/.claude/projects
pub fn default_projects_root() -> Result<PathBuf> {
    let base = BaseDirs::new().context("홈 디렉토리를 찾을 수 없습니다")?;
    Ok(base.home_dir().join(".claude").join("projects"))
}

/// cwd 절대경로 → 폴더명 변환 (/, \, : → -). folder_name_to_cwd의 역방향, M2 경로 매칭용(현재 미사용).
#[allow(dead_code)]
pub fn cwd_to_folder_name(cwd: &str) -> String {
    cwd.chars()
        .map(|c| match c {
            '/' | '\\' | ':' => '-',
            other => other,
        })
        .collect()
}

/// 폴더명 → cwd 역치환 (보조, 정확도 낮음)
pub fn folder_name_to_cwd(name: &str) -> String {
    // Windows 경로 패턴: D--Dev-foo → D:\Dev\foo
    // 첫 두 글자가 알파벳-이면 드라이브 레터로 시도
    if name.len() >= 2 {
        let chars: Vec<char> = name.chars().collect();
        if chars[0].is_ascii_alphabetic() && chars[1] == '-' && chars[2] == '-' {
            // D--Dev-claudeDesk → D:\Dev\claudeDesk
            let drive = chars[0];
            let rest: String = name[3..].replace('-', "\\");
            return format!("{}:\\{}", drive, rest);
        }
    }
    // 기본: -를 / 로 치환
    name.replace('-', "/")
}

/// path가 subagents 하위인지 확인
pub fn is_subagent_path(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == "subagents")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cwd_to_folder_name() {
        assert_eq!(
            cwd_to_folder_name("D:\\Dev\\claudeDesk"),
            "D--Dev-claudeDesk"
        );
        assert_eq!(
            cwd_to_folder_name("/home/user/project"),
            "-home-user-project"
        );
        assert_eq!(cwd_to_folder_name("C:\\Users\\PC"), "C--Users-PC");
    }

    #[test]
    fn test_is_subagent_path() {
        let p = std::path::Path::new("/home/.claude/projects/D--Dev/abc/subagents/agent-1.jsonl");
        assert!(is_subagent_path(p));

        let p2 = std::path::Path::new("/home/.claude/projects/D--Dev/abc.jsonl");
        assert!(!is_subagent_path(p2));
    }
}
