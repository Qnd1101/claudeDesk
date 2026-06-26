mod alias;
mod config;
mod data;
mod domain;
mod parser;
mod preview;
mod service;
mod trash;
mod ui;

use anyhow::Result;
use std::env;

use config::Config;
use service::{AppState, SessionService};
use ui::App;

fn main() -> Result<()> {
    // CLI 인자 파싱 (최소: --root, --verbose, --version, --help)
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

    // 커스텀 루트 경로: --root <path> 또는 CLAUDEDESK_ROOT 환경변수
    let custom_root = parse_arg_value(&args, "--root").or_else(|| env::var("CLAUDEDESK_ROOT").ok());

    let config = Config::load(custom_root, verbose)?;
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
  --root <path>    세션 루트 경로 지정 (기본: ~/.claude/projects)
  --verbose        상세 로그 출력
  --version        버전 정보 출력
  --help           이 도움말 출력

환경 변수:
  CLAUDEDESK_ROOT  세션 루트 경로 오버라이드

키 바인딩:
  ↑/k  위로 이동       ↓/j  아래로 이동
  Enter  세션 이어하기  q/Esc  종료
  ?    도움말 오버레이
",
        ver = env!("CARGO_PKG_VERSION")
    );
}
