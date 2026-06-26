//! claudedesk 바이너리 진입점.
//! 모듈 선언은 lib.rs에서 하고, 바이너리는 lib 크레이트에서 임포트한다.
//! (bin과 lib이 동일 소스를 중복 컴파일하면 dead_code 경고 중복; 이 구조가 정석)
use anyhow::Result;
use std::env;
use std::path::PathBuf;

use claudedesk::config::{CliOverrides, Config};
use claudedesk::service::{AppState, SessionService};
use claudedesk::ui::App;

fn main() -> Result<()> {
    // CLI 인자 파싱 (--root, --verbose, --sort, --no-color, --config, --version, --help)
    let args: Vec<String> = env::args().collect();

    if args.contains(&"--version".to_string()) || args.contains(&"-V".to_string()) {
        println!("claudedesk {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        return Ok(());
    }

    let verbose = args.contains(&"--verbose".to_string());
    let no_color = args.contains(&"--no-color".to_string());

    // --root <path> 또는 CLAUDEDESK_ROOT 환경변수
    let custom_root = parse_arg_value(&args, "--root").or_else(|| env::var("CLAUDEDESK_ROOT").ok());

    // --sort <key_dir> (예: "title_asc")
    let sort = parse_arg_value(&args, "--sort");

    // --config <path>
    let config_path = parse_arg_value(&args, "--config").map(PathBuf::from);

    let cli = CliOverrides {
        root: custom_root,
        sort,
        no_color,
        config: config_path,
        verbose,
    };

    let config = Config::load(&cli)?;
    let service = SessionService::new(config);
    let state = AppState::build(&service)?;

    let mut app = App::new(state);
    app.run()?;

    Ok(())
}

fn parse_arg_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}

fn print_help() {
    println!(
        "claudedesk {ver}
Claude Code 세션 관리자 TUI

사용법: claudedesk [옵션]

옵션:
  --root <path>      세션 루트 경로 지정 (기본: ~/.claude/projects)
  --sort <key_dir>   정렬 기준 (예: title_asc, modified_desc, created_asc, messages_desc)
  --no-color         색상 비활성화 (Theme::Mono 강제)
  --config <path>    설정 파일 경로 지정 (기본: ~/.claude/claudedesk/config.toml)
  --verbose          상세 로그 출력
  --version          버전 정보 출력
  --help             이 도움말 출력

환경 변수:
  CLAUDEDESK_ROOT    세션 루트 경로 오버라이드
  NO_COLOR           설정 시 색상 비활성화

키 바인딩:
  ↑/k  위로 이동       ↓/j  아래로 이동
  Enter  세션 이어하기  q/Esc  종료
  n    별칭 지정/편집 (빈칸 저장=삭제)
  ?    도움말 오버레이
",
        ver = env!("CARGO_PKG_VERSION")
    );
}
