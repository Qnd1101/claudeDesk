//! FR-10 설정 레이어 (T11.1).
//! 단일 진실원본: ~/.claude/claudedesk/config.toml (TOML).
//! CLI 인자가 파일보다 우선; Config::load(&CliOverrides)로 병합.
use anyhow::{Context, Result};
use directories::BaseDirs;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::path::{Path, PathBuf};

use crate::facet::Facet;

// ── 정렬 열거형 ───────────────────────────────────────────────────────────────

/// 정렬 키 (FR-07)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortKey {
    #[default]
    Modified,
    Created,
    Title,
    Messages,
}

impl SortKey {
    /// 다음 키로 순환 (Modified→Created→Title→Messages→Modified)
    pub fn next(self) -> Self {
        match self {
            SortKey::Modified => SortKey::Created,
            SortKey::Created => SortKey::Title,
            SortKey::Title => SortKey::Messages,
            SortKey::Messages => SortKey::Modified,
        }
    }

    /// 이전 키로 순환 (역방향)
    pub fn prev(self) -> Self {
        match self {
            SortKey::Modified => SortKey::Messages,
            SortKey::Created => SortKey::Modified,
            SortKey::Title => SortKey::Created,
            SortKey::Messages => SortKey::Title,
        }
    }

    /// UI 표시 레이블
    pub fn label(self) -> &'static str {
        match self {
            SortKey::Modified => "Modified",
            SortKey::Created => "Created",
            SortKey::Title => "Title",
            SortKey::Messages => "Messages",
        }
    }
}

impl fmt::Display for SortKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// 정렬 방향 (FR-07)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDir {
    Asc,
    #[default]
    Desc,
}

impl SortDir {
    /// 다음 방향으로 전환 (2-variant 토글)
    pub fn next(self) -> Self {
        match self {
            SortDir::Asc => SortDir::Desc,
            SortDir::Desc => SortDir::Asc,
        }
    }

    /// 이전 방향 (2-variant이므로 next와 동일)
    pub fn prev(self) -> Self {
        self.next()
    }

    /// service.rs 하위호환 toggle alias
    pub fn toggle(self) -> Self {
        self.next()
    }

    /// UI 표시 레이블
    pub fn label(self) -> &'static str {
        match self {
            SortDir::Asc => "Asc",
            SortDir::Desc => "Desc",
        }
    }

    /// 정렬 방향 화살표 (기존 service.rs 호환)
    pub fn arrow(self) -> &'static str {
        match self {
            SortDir::Desc => "↓",
            SortDir::Asc => "↑",
        }
    }
}

impl fmt::Display for SortDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// 기본 정렬 (SortKey + SortDir 합성).
/// TOML에서 `"modified_desc"` 단일 문자열로 직렬화/역직렬화.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultSort {
    pub key: SortKey,
    pub dir: SortDir,
}

impl DefaultSort {
    /// `"<key>_<dir>"` 형식 문자열로 직렬화 (예: "modified_desc")
    pub fn as_str(self) -> String {
        let key = match self.key {
            SortKey::Modified => "modified",
            SortKey::Created => "created",
            SortKey::Title => "title",
            SortKey::Messages => "messages",
        };
        let dir = match self.dir {
            SortDir::Asc => "asc",
            SortDir::Desc => "desc",
        };
        format!("{key}_{dir}")
    }

    /// `"<key>_<dir>"` 형식 문자열 파싱. 형식 불일치 시 None.
    pub fn parse(s: &str) -> Option<Self> {
        // 마지막 '_' 기준으로 분리: "messages_desc" → ("messages", "desc")
        let pos = s.rfind('_')?;
        let key_str = &s[..pos];
        let dir_str = &s[pos + 1..];
        let key = match key_str {
            "modified" => SortKey::Modified,
            "created" => SortKey::Created,
            "title" => SortKey::Title,
            "messages" => SortKey::Messages,
            _ => return None,
        };
        let dir = match dir_str {
            "asc" => SortDir::Asc,
            "desc" => SortDir::Desc,
            _ => return None,
        };
        Some(DefaultSort { key, dir })
    }
}

impl Default for DefaultSort {
    fn default() -> Self {
        DefaultSort {
            key: SortKey::Modified,
            dir: SortDir::Desc,
        }
    }
}

impl Serialize for DefaultSort {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.as_str())
    }
}

impl<'de> Deserialize<'de> for DefaultSort {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        DefaultSort::parse(&s).ok_or_else(|| {
            de::Error::custom(format!(
                "invalid default_sort '{}': expected '<key>_<dir>' (e.g. modified_desc)",
                s
            ))
        })
    }
}

// ── 기타 설정 열거형 ──────────────────────────────────────────────────────────

/// 시간 표시 형식
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeFormat {
    #[default]
    Relative,
    Absolute,
}

impl TimeFormat {
    /// 다음 variant 순환
    pub fn next(self) -> Self {
        match self {
            TimeFormat::Relative => TimeFormat::Absolute,
            TimeFormat::Absolute => TimeFormat::Relative,
        }
    }

    /// 이전 variant (2-variant이므로 next와 동일)
    pub fn prev(self) -> Self {
        self.next()
    }

    /// UI 표시 레이블
    pub fn label(self) -> &'static str {
        match self {
            TimeFormat::Relative => "Relative",
            TimeFormat::Absolute => "Absolute",
        }
    }
}

impl fmt::Display for TimeFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// resume 실행 방식
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResumeMode {
    #[default]
    Handoff,
    Spawn,
}

impl ResumeMode {
    /// 다음 variant 순환
    pub fn next(self) -> Self {
        match self {
            ResumeMode::Handoff => ResumeMode::Spawn,
            ResumeMode::Spawn => ResumeMode::Handoff,
        }
    }

    /// 이전 variant (2-variant이므로 next와 동일)
    pub fn prev(self) -> Self {
        self.next()
    }

    /// UI 표시 레이블
    pub fn label(self) -> &'static str {
        match self {
            ResumeMode::Handoff => "Handoff",
            ResumeMode::Spawn => "Spawn",
        }
    }
}

impl fmt::Display for ResumeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// 색상 테마
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Auto,
    Dark,
    Light,
    Mono,
}

impl Theme {
    /// 다음 variant 순환 (Auto→Dark→Light→Mono→Auto)
    pub fn next(self) -> Self {
        match self {
            Theme::Auto => Theme::Dark,
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Mono,
            Theme::Mono => Theme::Auto,
        }
    }

    /// 이전 variant 순환 (역방향)
    pub fn prev(self) -> Self {
        match self {
            Theme::Auto => Theme::Mono,
            Theme::Dark => Theme::Auto,
            Theme::Light => Theme::Dark,
            Theme::Mono => Theme::Light,
        }
    }

    /// UI 표시 레이블
    pub fn label(self) -> &'static str {
        match self {
            Theme::Auto => "Auto",
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::Mono => "Mono",
        }
    }
}

impl fmt::Display for Theme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

// ── TOML 파일 표현 (내부 전용) ────────────────────────────────────────────────

/// config.toml 직렬화/역직렬화 전용 구조체 (pub 불필요 — Config만 공개).
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct ConfigToml {
    projects_root: String,
    default_sort: DefaultSort,
    time_format: TimeFormat,
    resume_mode: ResumeMode,
    trash_retention_days: u32,
    active_window_secs: u64,
    include_subagents: bool,
    theme: Theme,
    stale_days: u32,
    default_facet: Facet,
}

impl Default for ConfigToml {
    fn default() -> Self {
        ConfigToml {
            projects_root: "~/.claude/projects".to_string(),
            default_sort: DefaultSort::default(),
            time_format: TimeFormat::default(),
            resume_mode: ResumeMode::default(),
            trash_retention_days: 30,
            active_window_secs: 90,
            include_subagents: false,
            theme: Theme::default(),
            stale_days: 90,
            default_facet: Facet::Recent,
        }
    }
}

// ── CLI 오버라이드 ────────────────────────────────────────────────────────────

/// 커맨드라인 인자 오버라이드 (§5.10). config.toml 위에 덮어씀.
#[derive(Default)]
pub struct CliOverrides {
    /// --root <path> 또는 CLAUDEDESK_ROOT 환경변수
    pub root: Option<String>,
    /// --sort <key_dir> (예: "title_asc"). 파싱 실패 시 파일 값 유지.
    pub sort: Option<String>,
    /// --no-color. true이면 Theme::Mono 강제.
    pub no_color: bool,
    /// --config <path>. 지정 경로의 TOML을 사용.
    pub config: Option<PathBuf>,
    /// --verbose
    pub verbose: bool,
}

// ── 런타임 설정 ───────────────────────────────────────────────────────────────

/// 런타임 설정 (config.toml + CLI 오버라이드 합성). 모든 필드 typed(enum).
#[derive(Debug, Clone)]
pub struct Config {
    /// 세션 루트 디렉토리
    pub projects_root: PathBuf,
    /// 기본 정렬 (키 + 방향)
    pub default_sort: DefaultSort,
    /// 시간 표시 형식
    pub time_format: TimeFormat,
    /// resume 실행 방식
    pub resume_mode: ResumeMode,
    /// 휴지통 보존 기간(일)
    pub trash_retention_days: u32,
    /// 활성 세션 판정 임계(초)
    pub active_window_secs: u64,
    /// 서브에이전트 포함 여부
    pub include_subagents: bool,
    /// 색상 테마
    pub theme: Theme,
    /// 세션 stale 판정 기간(일, 기본 90)
    pub stale_days: u32,
    /// 기본 facet (기본 Recent)
    pub default_facet: Facet,
    /// 상세 로그
    pub verbose: bool,
    /// 설정 파일 경로 (save 대상)
    config_file_path: PathBuf,
}

impl Config {
    /// config.toml 로드 + CLI 오버라이드 병합.
    ///
    /// - 파일 미존재 시 기본값으로 파일 생성 후 반환.
    /// - 파싱 실패 시 graceful default (크래시 없음).
    /// - CLI 오버라이드가 파일 값보다 우선.
    pub fn load(cli: &CliOverrides) -> Result<Self> {
        let config_path = cli
            .config
            .clone()
            .map(Ok)
            .unwrap_or_else(default_config_path)?;

        let file_cfg = load_config_file(&config_path);

        let projects_root = if let Some(ref root) = cli.root {
            expand_tilde(root)
        } else {
            expand_tilde(&file_cfg.projects_root)
        };

        let default_sort = if let Some(ref sort_str) = cli.sort {
            DefaultSort::parse(sort_str).unwrap_or(file_cfg.default_sort)
        } else {
            file_cfg.default_sort
        };

        let theme = if cli.no_color {
            Theme::Mono
        } else {
            file_cfg.theme
        };

        Ok(Config {
            projects_root,
            default_sort,
            time_format: file_cfg.time_format,
            resume_mode: file_cfg.resume_mode,
            trash_retention_days: file_cfg.trash_retention_days,
            active_window_secs: file_cfg.active_window_secs,
            include_subagents: file_cfg.include_subagents,
            theme,
            stale_days: file_cfg.stale_days,
            default_facet: file_cfg.default_facet,
            verbose: cli.verbose,
            config_file_path: config_path,
        })
    }

    /// 현재 설정을 config.toml에 원자적으로 저장 (temp write → rename).
    /// 설정 화면 `s 저장`이 호출.
    pub fn save(&self) -> Result<()> {
        let cfg = ConfigToml {
            projects_root: self.projects_root.to_string_lossy().to_string(),
            default_sort: self.default_sort,
            time_format: self.time_format,
            resume_mode: self.resume_mode,
            trash_retention_days: self.trash_retention_days,
            active_window_secs: self.active_window_secs,
            include_subagents: self.include_subagents,
            theme: self.theme,
            stale_days: self.stale_days,
            default_facet: self.default_facet,
        };

        let toml_str = toml::to_string_pretty(&cfg).context("config 직렬화 실패")?;

        let path = &self.config_file_path;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("config 디렉토리 생성 실패")?;
        }

        let tmp = path
            .parent()
            .unwrap_or(Path::new("."))
            .join("config.toml.tmp");
        std::fs::write(&tmp, &toml_str).context("config temp 쓰기 실패")?;
        std::fs::rename(&tmp, path).context("config rename 실패")?;

        Ok(())
    }

    /// 저장 대상 경로 노출 (설정 화면 표시용).
    pub fn config_path(&self) -> &Path {
        &self.config_file_path
    }

    /// 색상 활성 여부.
    /// Theme::Mono 이거나 환경변수 `NO_COLOR`가 설정돼 있으면 false.
    pub fn color_enabled(&self) -> bool {
        if self.theme == Theme::Mono {
            return false;
        }
        std::env::var_os("NO_COLOR").is_none()
    }
}

// ── 내부 헬퍼 ─────────────────────────────────────────────────────────────────

/// 기본 config.toml 경로: ~/.claude/claudedesk/config.toml
fn default_config_path() -> Result<PathBuf> {
    let base = BaseDirs::new().context("홈 디렉토리를 찾을 수 없습니다")?;
    Ok(base
        .home_dir()
        .join(".claude")
        .join("claudedesk")
        .join("config.toml"))
}

/// config.toml 로드. 없으면 기본값으로 파일 생성 후 반환.
/// 손상·파싱 실패 시 graceful default (alias.rs::load_alias_from 선례 패턴).
fn load_config_file(path: &Path) -> ConfigToml {
    if !path.exists() {
        let default = ConfigToml::default();
        // 파일이 없으면 기본값으로 생성 (실패해도 graceful 진행)
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(toml_str) = toml::to_string_pretty(&default) {
            let tmp = path
                .parent()
                .unwrap_or(Path::new("."))
                .join("config.toml.tmp");
            if std::fs::write(&tmp, &toml_str).is_ok() {
                let _ = std::fs::rename(&tmp, path);
            }
        }
        return default;
    }

    match std::fs::read_to_string(path) {
        Ok(s) => toml::from_str::<ConfigToml>(&s).unwrap_or_default(),
        Err(_) => ConfigToml::default(),
    }
}

// ── 공개 경로 유틸 (기존 코드가 사용 중) ─────────────────────────────────────

/// `~` 를 홈 디렉토리로 확장
pub fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with('~') {
        if let Some(base) = BaseDirs::new() {
            let home = base.home_dir();
            let rest = path.trim_start_matches('~').trim_start_matches('/');
            // Windows: 역슬래시도 처리
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

/// cwd 절대경로 → 폴더명 변환 (/, \, : → -). folder_name_to_cwd의 역방향.
#[allow(dead_code)]
pub fn cwd_to_folder_name(cwd: &str) -> String {
    cwd.chars()
        .map(|c| match c {
            '/' | '\\' | ':' => '-',
            other => other,
        })
        .collect()
}

/// 폴더명 → cwd 역치환 (보조, 정확도 낮음; parser.rs가 사용)
pub fn folder_name_to_cwd(name: &str) -> String {
    // Windows 경로 패턴: D--Dev-foo → D:\Dev\foo
    let chars: Vec<char> = name.chars().collect();
    if chars.len() >= 3 && chars[0].is_ascii_alphabetic() && chars[1] == '-' && chars[2] == '-' {
        let drive = chars[0];
        let rest: String = name[3..].replace('-', "\\");
        return format!("{}:\\{}", drive, rest);
    }
    name.replace('-', "/")
}

/// path가 subagents 하위인지 확인
pub fn is_subagent_path(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == "subagents")
}

// ── 테스트 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── 기존 경로 유틸 테스트 (회귀) ──────────────────────────────────────────

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

    // ── DefaultSort 라운드트립 ─────────────────────────────────────────────────

    #[test]
    fn test_default_sort_roundtrip_all_variants() {
        let cases = [
            ("modified_desc", SortKey::Modified, SortDir::Desc),
            ("modified_asc", SortKey::Modified, SortDir::Asc),
            ("created_desc", SortKey::Created, SortDir::Desc),
            ("created_asc", SortKey::Created, SortDir::Asc),
            ("title_desc", SortKey::Title, SortDir::Desc),
            ("title_asc", SortKey::Title, SortDir::Asc),
            ("messages_desc", SortKey::Messages, SortDir::Desc),
            ("messages_asc", SortKey::Messages, SortDir::Asc),
        ];
        for (s, key, dir) in &cases {
            let parsed = DefaultSort::parse(s).unwrap_or_else(|| panic!("파싱 실패: {s}"));
            assert_eq!(parsed.key, *key, "SortKey 불일치: {s}");
            assert_eq!(parsed.dir, *dir, "SortDir 불일치: {s}");
            assert_eq!(parsed.as_str(), *s, "as_str 라운드트립 불일치: {s}");
        }
    }

    #[test]
    fn test_default_sort_parse_invalid() {
        assert!(DefaultSort::parse("").is_none());
        assert!(DefaultSort::parse("modified").is_none());
        assert!(DefaultSort::parse("modified_bad").is_none());
        assert!(DefaultSort::parse("bad_desc").is_none());
    }

    // ── Config 기본 로드 + 파일 생성 확인 ─────────────────────────────────────

    #[test]
    fn test_load_creates_file_with_defaults() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        assert!(!path.exists(), "사전 조건: 파일이 없어야 함");

        let cli = CliOverrides {
            config: Some(path.clone()),
            ..CliOverrides::default()
        };
        let config = Config::load(&cli).unwrap();

        // 기본값 검증
        assert_eq!(config.default_sort, DefaultSort::default());
        assert_eq!(config.time_format, TimeFormat::Relative);
        assert_eq!(config.resume_mode, ResumeMode::Handoff);
        assert_eq!(config.trash_retention_days, 30);
        assert_eq!(config.active_window_secs, 90);
        assert!(!config.include_subagents);
        assert_eq!(config.theme, Theme::Auto);

        // 파일이 생성됐는지 확인
        assert!(path.exists(), "기본값 로드 후 config.toml이 생성돼야 함");
    }

    // ── 유효 TOML 파싱 → enum 정확 매핑 ──────────────────────────────────────

    #[test]
    fn test_valid_toml_parses_all_enums() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
projects_root = "/custom/root"
default_sort = "title_asc"
time_format = "absolute"
resume_mode = "spawn"
trash_retention_days = 60
active_window_secs = 120
include_subagents = true
theme = "dark"
"#,
        )
        .unwrap();

        let cli = CliOverrides {
            config: Some(path),
            ..CliOverrides::default()
        };
        let config = Config::load(&cli).unwrap();

        assert_eq!(
            config.default_sort,
            DefaultSort {
                key: SortKey::Title,
                dir: SortDir::Asc
            }
        );
        assert_eq!(config.time_format, TimeFormat::Absolute);
        assert_eq!(config.resume_mode, ResumeMode::Spawn);
        assert_eq!(config.trash_retention_days, 60);
        assert_eq!(config.active_window_secs, 120);
        assert!(config.include_subagents);
        assert_eq!(config.theme, Theme::Dark);
    }

    // ── 라운드트립: save → load → 동일 ────────────────────────────────────────

    #[test]
    fn test_save_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");

        // 1. 로드 (기본값 생성)
        let cli = CliOverrides {
            config: Some(path.clone()),
            ..CliOverrides::default()
        };
        let mut config = Config::load(&cli).unwrap();

        // 2. 값 변경
        config.default_sort = DefaultSort {
            key: SortKey::Messages,
            dir: SortDir::Asc,
        };
        config.time_format = TimeFormat::Absolute;
        config.resume_mode = ResumeMode::Spawn;
        config.trash_retention_days = 90;
        config.active_window_secs = 180;
        config.include_subagents = true;
        config.theme = Theme::Light;

        // 3. 저장
        config.save().unwrap();

        // 4. 다시 로드
        let cli2 = CliOverrides {
            config: Some(path.clone()),
            ..CliOverrides::default()
        };
        let loaded = Config::load(&cli2).unwrap();

        // 5. 동일 여부 확인
        assert_eq!(
            loaded.default_sort, config.default_sort,
            "default_sort 라운드트립 실패"
        );
        assert_eq!(
            loaded.time_format, config.time_format,
            "time_format 라운드트립 실패"
        );
        assert_eq!(
            loaded.resume_mode, config.resume_mode,
            "resume_mode 라운드트립 실패"
        );
        assert_eq!(
            loaded.trash_retention_days, config.trash_retention_days,
            "trash_retention_days 라운드트립 실패"
        );
        assert_eq!(
            loaded.active_window_secs, config.active_window_secs,
            "active_window_secs 라운드트립 실패"
        );
        assert_eq!(
            loaded.include_subagents, config.include_subagents,
            "include_subagents 라운드트립 실패"
        );
        assert_eq!(loaded.theme, config.theme, "theme 라운드트립 실패");
    }

    // ── 원자적 저장: .tmp 잔류 없음 ───────────────────────────────────────────

    #[test]
    fn test_save_atomic_no_tmp_residue() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");

        let cli = CliOverrides {
            config: Some(path.clone()),
            ..CliOverrides::default()
        };
        let config = Config::load(&cli).unwrap();
        config.save().unwrap();

        let tmp_path = tmp.path().join("config.toml.tmp");
        assert!(
            !tmp_path.exists(),
            "저장 후 config.toml.tmp 잔류 — rename 실패"
        );
        assert!(path.exists(), "저장 후 config.toml이 없음");
    }

    // ── CLI 오버라이드 우선순위 ────────────────────────────────────────────────

    #[test]
    fn test_cli_sort_override_wins_over_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        // 파일엔 modified_desc
        std::fs::write(&path, "default_sort = \"modified_desc\"\n").unwrap();

        let cli = CliOverrides {
            config: Some(path),
            sort: Some("title_asc".to_string()), // CLI가 title_asc
            ..CliOverrides::default()
        };
        let config = Config::load(&cli).unwrap();

        assert_eq!(
            config.default_sort,
            DefaultSort {
                key: SortKey::Title,
                dir: SortDir::Asc
            },
            "CLI --sort가 파일의 default_sort를 오버라이드해야 함"
        );
    }

    #[test]
    fn test_cli_sort_invalid_falls_back_to_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(&path, "default_sort = \"created_desc\"\n").unwrap();

        let cli = CliOverrides {
            config: Some(path),
            sort: Some("garbage_value".to_string()), // 파싱 실패 → 파일 값 유지
            ..CliOverrides::default()
        };
        let config = Config::load(&cli).unwrap();

        assert_eq!(
            config.default_sort,
            DefaultSort {
                key: SortKey::Created,
                dir: SortDir::Desc
            },
            "CLI --sort 파싱 실패 시 파일 값으로 폴백해야 함"
        );
    }

    #[test]
    fn test_cli_no_color_forces_mono() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(&path, "theme = \"dark\"\n").unwrap();

        let cli = CliOverrides {
            config: Some(path),
            no_color: true, // --no-color → Theme::Mono
            ..CliOverrides::default()
        };
        let config = Config::load(&cli).unwrap();

        assert_eq!(config.theme, Theme::Mono, "--no-color 시 Theme::Mono 강제");
        assert!(
            !config.color_enabled(),
            "--no-color 시 color_enabled false여야"
        );
    }

    // ── color_enabled ──────────────────────────────────────────────────────────

    #[test]
    fn test_color_enabled_mono_theme() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(&path, "theme = \"mono\"\n").unwrap();

        let cli = CliOverrides {
            config: Some(path),
            ..CliOverrides::default()
        };
        let config = Config::load(&cli).unwrap();

        // NO_COLOR 환경변수와 무관하게 Mono이면 항상 false
        assert!(!config.color_enabled(), "Theme::Mono → color_enabled false");
    }

    #[test]
    fn test_color_enabled_auto_theme_without_no_color() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(&path, "theme = \"auto\"\n").unwrap();

        let cli = CliOverrides {
            config: Some(path),
            ..CliOverrides::default()
        };
        let config = Config::load(&cli).unwrap();

        // NO_COLOR가 설정돼 있지 않은 환경에서는 true여야 함
        // (CI 환경에서 NO_COLOR가 설정돼 있을 수 있으므로 설정 여부로 분기)
        let expected = std::env::var_os("NO_COLOR").is_none();
        assert_eq!(
            config.color_enabled(),
            expected,
            "Auto 테마: NO_COLOR 미설정 시 true, 설정 시 false"
        );
    }

    // ── 손상된 TOML graceful 처리 ──────────────────────────────────────────────

    #[test]
    fn test_corrupted_toml_graceful_default() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(&path, b"{{ bad toml !!").unwrap();

        let cli = CliOverrides {
            config: Some(path),
            ..CliOverrides::default()
        };
        let config = Config::load(&cli).unwrap();

        // 손상돼도 기본값으로 graceful 복귀
        assert_eq!(config.default_sort, DefaultSort::default());
        assert_eq!(config.theme, Theme::Auto);
    }

    // ── 열거형 next/prev/label 순환 검증 ──────────────────────────────────────

    #[test]
    fn test_sort_key_next_prev_cycle() {
        let variants = [
            SortKey::Modified,
            SortKey::Created,
            SortKey::Title,
            SortKey::Messages,
        ];
        for i in 0..variants.len() {
            let cur = variants[i];
            let nxt = variants[(i + 1) % variants.len()];
            let prv = variants[(i + variants.len() - 1) % variants.len()];
            assert_eq!(cur.next(), nxt, "SortKey::next 순환 불일치: {:?}", cur);
            assert_eq!(cur.prev(), prv, "SortKey::prev 순환 불일치: {:?}", cur);
        }
    }

    #[test]
    fn test_theme_next_prev_cycle() {
        let variants = [Theme::Auto, Theme::Dark, Theme::Light, Theme::Mono];
        for i in 0..variants.len() {
            let cur = variants[i];
            let nxt = variants[(i + 1) % variants.len()];
            let prv = variants[(i + variants.len() - 1) % variants.len()];
            assert_eq!(cur.next(), nxt, "Theme::next 순환 불일치: {:?}", cur);
            assert_eq!(cur.prev(), prv, "Theme::prev 순환 불일치: {:?}", cur);
        }
    }

    #[test]
    fn test_sort_key_labels() {
        assert_eq!(SortKey::Modified.label(), "Modified");
        assert_eq!(SortKey::Created.label(), "Created");
        assert_eq!(SortKey::Title.label(), "Title");
        assert_eq!(SortKey::Messages.label(), "Messages");
    }

    #[test]
    fn test_time_format_labels() {
        assert_eq!(TimeFormat::Relative.label(), "Relative");
        assert_eq!(TimeFormat::Absolute.label(), "Absolute");
    }

    #[test]
    fn test_resume_mode_labels() {
        assert_eq!(ResumeMode::Handoff.label(), "Handoff");
        assert_eq!(ResumeMode::Spawn.label(), "Spawn");
    }

    #[test]
    fn test_theme_labels() {
        assert_eq!(Theme::Auto.label(), "Auto");
        assert_eq!(Theme::Dark.label(), "Dark");
        assert_eq!(Theme::Light.label(), "Light");
        assert_eq!(Theme::Mono.label(), "Mono");
    }

    // ── config_path() 노출 ──────────────────────────────────────────────────────

    #[test]
    fn test_config_path_matches_cli_input() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");

        let cli = CliOverrides {
            config: Some(path.clone()),
            ..CliOverrides::default()
        };
        let config = Config::load(&cli).unwrap();

        assert_eq!(config.config_path(), path.as_path());
    }

    // ── stale_days & default_facet 필드 추가 테스트 (T5) ────────────────────────

    #[test]
    fn test_load_defaults_stale_days_and_facet() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        assert!(!path.exists());

        let cli = CliOverrides {
            config: Some(path),
            ..CliOverrides::default()
        };
        let config = Config::load(&cli).unwrap();

        assert_eq!(config.stale_days, 90, "기본 stale_days는 90");
        assert_eq!(
            config.default_facet,
            crate::facet::Facet::Recent,
            "기본 default_facet은 Recent"
        );
    }

    #[test]
    fn test_toml_parses_stale_days_and_facet() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
stale_days = 60
default_facet = "cleanup"
"#,
        )
        .unwrap();

        let cli = CliOverrides {
            config: Some(path),
            ..CliOverrides::default()
        };
        let config = Config::load(&cli).unwrap();

        assert_eq!(config.stale_days, 60, "TOML stale_days=60 파싱");
        assert_eq!(
            config.default_facet,
            crate::facet::Facet::Cleanup,
            "TOML default_facet=cleanup 파싱"
        );
    }

    #[test]
    fn test_save_load_roundtrip_stale_days_and_facet() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");

        let cli = CliOverrides {
            config: Some(path.clone()),
            ..CliOverrides::default()
        };
        let mut config = Config::load(&cli).unwrap();

        // 값 변경
        config.stale_days = 120;
        config.default_facet = crate::facet::Facet::Active;

        // 저장
        config.save().unwrap();

        // 다시 로드
        let cli2 = CliOverrides {
            config: Some(path),
            ..CliOverrides::default()
        };
        let loaded = Config::load(&cli2).unwrap();

        assert_eq!(loaded.stale_days, 120, "stale_days 라운드트립 실패");
        assert_eq!(
            loaded.default_facet,
            crate::facet::Facet::Active,
            "default_facet 라운드트립 실패"
        );
    }

    #[test]
    fn test_all_facets_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");

        let facets = [
            crate::facet::Facet::Recent,
            crate::facet::Facet::Active,
            crate::facet::Facet::Cleanup,
            crate::facet::Facet::Project,
        ];

        for facet in &facets {
            let cli = CliOverrides {
                config: Some(path.clone()),
                ..CliOverrides::default()
            };
            let mut config = Config::load(&cli).unwrap();
            config.default_facet = *facet;
            config.save().unwrap();

            let cli2 = CliOverrides {
                config: Some(path.clone()),
                ..CliOverrides::default()
            };
            let loaded = Config::load(&cli2).unwrap();

            assert_eq!(
                loaded.default_facet, *facet,
                "facet {:?} 라운드트립 실패",
                facet
            );
        }
    }
}
