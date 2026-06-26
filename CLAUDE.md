# claudeDesk — Project Guide

Claude Code 세션 하우스키핑 TUI. `claude`를 띄우지 않고 세션을 목록·검색·그룹핑·미리보기·삭제(휴지통)한다.

> 작업 방식·브랜치·게이트 등 공통 운영 규칙은 글로벌 charter를 따른다. 이 파일엔 **claudeDesk 고유 사항만** 둔다.

## 스택
Rust 2021 / ratatui 0.29 + crossterm 0.28 / serde·serde_json / chrono · directories · anyhow.
lib(`claudedesk`) + bin(`claudedesk`) 이중 크레이트. 릴리스 프로파일은 크기·RAM 최적화(`opt-level="z"`, lto, strip).

## 빌드 · 테스트 · 린트
> `cargo`가 PATH에 없으면 `~/.cargo/bin`(rustup 기본)을 추가한 뒤 실행한다.

| 목적 | 명령 (CI 게이트 A와 동일) |
|---|---|
| 빌드 | `cargo build` / `cargo build --release` |
| 테스트 | `cargo test --all` (유닛 + `tests/` 통합 3종 + 원본 SHA 불변 회귀) |
| 린트 | `cargo clippy --all-targets --all-features -- -D warnings` |
| 포맷 | `cargo fmt --all -- --check` |

## 모듈 레이어 (`src/`)
기반 `config`(경로·`CLAUDEDESK_ROOT`)·`domain`(타입) → `data`(FS 스캔) → `parser`(JSONL) → `service`(오케스트레이션·resume·외부 `claude` 실행).
`trash`(소프트삭제/휴지통)·`preview`(스트리밍 미리보기)는 독립 모듈이며, `ui/`(ratatui 렌더+키 핸들)가 이들과 `service`를 묶는다.
기능엔 `FR-XX` 번호가 붙는다 — 정의는 `docs/00_PRD.md`.

## 불변 원칙 (어기지 말 것)
- 🔒 **원본 `~/.claude/projects/**/*.jsonl`은 읽기 전용 — 절대 수정 금지.** CI가 SHA 불변을 회귀로 강제한다.
- 테스트 픽스처는 **합성 데이터만**(`tests/fixtures/`). 실제 세션 본문·실제 cwd 금지.
- **RAM 바운드:** 세션 전체를 메모리에 올리지 않는다. 미리보기는 바이트 실링까지만 스트리밍.

## repo / CI (이 repo는 GitHub — 글로벌 charter의 GitLab+Jenkins 예외)
`github.com/Qnd1101/claudeDesk` · 브랜치 `feature/<task> → develop → verify → main`(squash) · CD는 `v*` 태그.
상세 게이트·인증은 메모리 `claudedesk-git-workflow` 참조.

## 문서
`docs/` — PRD(`00`) · Task 분해(`01`) · UIUX(`02`) · DevKickoff(`03`).
