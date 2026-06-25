# claudeDesk

> Claude Code의 로컬 세션(채팅방)을 **초경량 RAM(<20MB 목표)**으로 관리하는 크로스플랫폼 **TUI** 툴.
> 세션 조회·검색·정렬·이어하기(resume)·안전 삭제·별칭이 핵심.
> 스택: **Rust + ratatui(crossterm) + serde_json**.

[![CI (Gate A)](https://github.com/Qnd1101/claudeDesk/actions/workflows/ci.yml/badge.svg)](https://github.com/Qnd1101/claudeDesk/actions/workflows/ci.yml)

## 왜?

Claude Code 세션이 `~/.claude/projects/`에 수백 개 누적되지만 기본 CLI로는 **어느 세션이 무엇이었는지 식별·재진입이 번거롭다.** 기존 AI 관리 GUI는 Electron 기반이라 무겁다. claudeDesk는 **단일 정적 바이너리 + 초경량 TUI**로 이를 해결한다.

## 핵심 원칙

- **Non-Destructive:** 원본 JSONL은 **읽기 전용**. 부가정보(별칭 등)는 사이드카(`~/.claude/claudedesk/`)에 분리.
- **Privacy First:** 외부 전송 0, 텔레메트리 없음, 100% 로컬.
- **Ultra-Low RAM:** 전체 파싱 금지("첫 user 줄까지 스캔 + `mtime` stat"). RAM<20MB는 M0에서 **실측 검증**.

## 문서 (`docs/`)

| 문서 | 내용 |
| :--- | :--- |
| [docs/README.md](docs/README.md) | 패키지 인덱스(여기서 시작) |
| [docs/00_PRD.md](docs/00_PRD.md) | 제품 요구사항 v2.1.0 — FR-01~13, 측정가능 NFR, 기술 부록 A~J |
| [docs/01_TASK_BREAKDOWN.md](docs/01_TASK_BREAKDOWN.md) | 마일스톤·Epic(12)·Task(45) 분할, critical path |
| [docs/02_UIUX_DESIGN.md](docs/02_UIUX_DESIGN.md) | TUI 설계 — 화면·키맵·반응형·접근성 |
| [docs/03_DEV_KICKOFF.md](docs/03_DEV_KICKOFF.md) | 개발 착수 가이드 + **§9 Git 워크플로우/자율 운영** |

## 개발 워크플로우

브랜치: `feature/* → develop → verify → main`. 모든 변경은 **PR**을 거치며 CI(게이트 A)·에이전트 검증(게이트 B)을 통과해야 한다. 상세는 [CONTRIBUTING.md](CONTRIBUTING.md).

## 상태

기획·워크플로우 베이스라인(v0.1.0). 코드 구현은 **M0(기술 검증 스파이크)**부터. → [docs/03_DEV_KICKOFF.md](docs/03_DEV_KICKOFF.md) §3.

## License

미정(TBD).
