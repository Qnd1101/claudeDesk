# claudeDesk — 개발 착수 문서 패키지

claudeDesk = Claude Code 로컬 세션(채팅방)을 **초경량 RAM(<20MB 목표)**으로 관리하는 크로스플랫폼 TUI 툴.
세션 조회·검색·정렬·이어하기(resume)·안전 삭제·별칭이 핵심. 스택: **Rust + ratatui(crossterm) + serde_json**.

## 문서 인덱스

| 문서 | 요약 |
| :--- | :--- |
| [00_PRD.md](00_PRD.md) | 재설계 PRD **v2.0.0** — 기능요구(FR-01~13), 측정가능 NFR, 마일스톤, 기술 부록 A~J, v2 변경이력. |
| [01_TASK_BREAKDOWN.md](01_TASK_BREAKDOWN.md) | 마일스톤(M0~M3)·Epic(12)·Task(45) 분할 — 의존성·완료조건·규모·관련 FR. |
| [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md) | TUI 설계 — 7개 화면 와이어프레임, 전이도, 키맵(24), 반응형, 접근성, ratatui 위젯 매핑. |
| [03_DEV_KICKOFF.md](03_DEV_KICKOFF.md) | 착수 가이드 — 문서↔마일스톤 매핑, 작업 순서, 첫 스프린트 진입점, Done 기준, 모듈 계약, Open Questions. |

## 빠른 시작 (읽는 순서)

1. [00_PRD.md](00_PRD.md) §1 "v2.0.0 핵심 변경"과 §4 기능요구로 **무엇을 만드는지** 파악.
2. [03_DEV_KICKOFF.md](03_DEV_KICKOFF.md) §3 첫 스프린트로 **어디서 시작하는지** 확인 (**M0 게이트가 1번**).
3. [01_TASK_BREAKDOWN.md](01_TASK_BREAKDOWN.md)에서 Task별 AC·의존성 확인.
4. TUI 구현 시 [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md) 와이어프레임·키맵을 계약으로 사용.

## 핵심 원칙 (요약)

* **Non-Destructive:** 원본 JSONL 읽기 전용. 부가정보는 사이드카(`~/.claude/claudedesk/meta.json`).
* **M0 게이트 우선:** RAM<20MB·로딩≤300ms를 **실측 검증**한 뒤에야 본 구현 착수.
* **Privacy First:** 외부 전송 0, 텔레메트리 없음.
* **1인 운영:** 과잉설계 금지, 단순·낮은 운영부담 우선.

## CEO 결정 대기 (상세는 [03_DEV_KICKOFF.md](03_DEV_KICKOFF.md) §7)

* **Q2** Windows resume 핸드오프 방식 — **M1 차단요소**.
* **Q3** Windows conhost 지원 범위.
