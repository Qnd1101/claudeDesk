# 제품 요구사항 정의서 (PRD): claudeDesk

## 1. 문서 메타데이터 (Document Metadata)

* **프로젝트명:** claudeDesk (클로드데스크)
* **문서 버전:** **v2.1.0**
* **작성자:** 개발자(본인) + architect 재설계 + 레드팀 보강
* **문서 상태:** Redesigned (개발 착수용 재설계 완료 + 자율 운영 게이트 반영)
* **최종 업데이트 일자:** 2026-06-25
* **관련 문서:** [01_TASK_BREAKDOWN.md](01_TASK_BREAKDOWN.md) · [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md) · [03_DEV_KICKOFF.md](03_DEV_KICKOFF.md)

### 변경 이력 (Change Log)

| 버전 | 일자 | 변경 내용 |
| :--- | :--- | :--- |
| v1.0.0 | 2026-06-25 | 초안 작성 (개요·기능·NFR·마일스톤) |
| v1.1.0 | 2026-06-25 | 실측 데이터 모델 반영, 스택 확정(Rust+ratatui), FR-07~09 추가, 기술 부록 A~J |
| **v2.0.0** | **2026-06-25** | **재설계. 아래 "v2.0.0 핵심 변경" 참조.** |
| **v2.1.0** | **2026-06-25** | 레드팀 보강: 원본 JSONL SHA 불변을 **CI 필수 게이트**로 승격(§5.11·§8), 경량 인덱스 캐시 **조건부 도입** 명기(§6), 픽스처 합성데이터·gitleaks 명문화. (워크플로우·브랜치·게이트는 [03_DEV_KICKOFF.md](03_DEV_KICKOFF.md) §9) |

### v2.0.0 핵심 변경 (Why)

1. **NFR 측정 가능화.** "200ms 이내"·"RAM<20MB"에 **측정 방법·하드웨어 기준선·세션 수 조건**을 명시. 모순되던 RAM 밴드(15/30MB vs <20MB 하드타깃)를 **단일 정의(상주 RSS, 측정 시나리오 고정)**로 통일. (§5)
2. **Resume 기본 전략 전환: 새 터미널 스폰 → "셸 핸드오프(exec/hand-back)" 기본.** 새 창 스폰은 OS·터미널별로 신뢰도가 낮고 cwd·환경 승계가 깨지기 쉬움. claudeDesk가 종료하며 선택 세션의 `cwd`에서 `claude --resume <id>`로 **프로세스를 교체(또는 부모 셸에 명령 위임)**하는 방식을 1순위로 확정. 새 창 스폰은 옵션(Could)로 강등. (§4 FR-03, 부록 C)
3. **제목/메타 도출 로직 실측 정정.** 실측 결과 **첫 줄이 user 메시지가 아니다**(예: `agent-setting`, `mode`, `attachment` 등 메타 줄이 7줄 선행). "첫 줄만 읽기" 성능 가정을 **"첫 user `text` 블록까지 앞에서 K줄 스트리밍 스캔(상한 가드)"**으로 정정. `content`가 블록 배열일 때 `text` 블록 추출 규칙 명문화. (부록 B)
4. **누락 비기능 항목 신설.** 로깅(§5.4), 설정 스키마(§5.5), 국제화·시간 표기(§5.6), TUI 접근성·컬러무관(§5.7), 패키징·배포(§5.8), 업데이트(§5.9), CLI 인자(§5.10), 테스트 전략(§5.11)을 정식 NFR로 편입.
5. **MVP 범위 축소(우선순위 재조정).** **FR-09(프로젝트 그룹핑)을 M1 Must → M2 Should**로 이동. 그룹/트리는 평면 리스트보다 렌더·상태관리 복잡도가 커 MVP 리스크. MVP는 **평면 리스트(프로젝트 컬럼 표시)**로 먼저 가치 전달.
6. **"활성 세션" 판정 휴리스틱 구체화.** 파일 락 감지는 OS별 신뢰 불가 → **mtime 근접도(기본 ≤ 90초) + 진행성 변화 휴리스틱**을 1차 기준으로 확정하고, 락은 보조로만. 오탐 시에도 "삭제 차단"은 보수적으로 적용. (부록 E, 부록 J Q4 해소)
7. **에러 가시성 추가(FR-12).** graceful skip된 손상 줄/파일 수를 **상태바·로그로 사용자에게 노출**. 조용한 실패 방지.
8. **삭제 안전장치 명문화.** 소프트 삭제(휴지통 디렉토리 이동) + 보관 기간(기본 30일) + 복구(FR-11) + 영구삭제 별도 확인 2단계. (FR-04 분해)
9. **성공지표를 검증 가능한 Exit Criteria로 재작성.** "락 현상 0%" 같은 비측정 문구 제거, M0 게이트(실측 RAM)와 마일스톤별 Done 기준으로 대체. (§8, [03_DEV_KICKOFF.md](03_DEV_KICKOFF.md))
10. **용어·FR ID 정합.** 본 문서의 FR ID(FR-01~FR-13)를 Task/UIUX 문서와 1:1 일치시킴.

---

## 2. 프로젝트 개요 및 배경 (Executive Summary & Background)

* **제품 요약 (Elevator Pitch):** Claude Code의 로컬 세션(채팅방)을 **초경량 RAM(<20MB 목표)**으로 빠르고 안전하게 조회·검색·정렬·이어하기·삭제·별칭 관리하는 크로스플랫폼 TUI 툴.
* **개발 배경 (Problem Statement):**
  * Claude Code 세션이 `~/.claude/projects/`에 수백 개 누적되지만 기본 CLI로는 **어느 세션이 무엇이었는지 식별·재진입이 번거로움**.
  * 기존 AI 관리 GUI는 Electron 기반이 많아 RAM 소모가 큼. **Docker·IDE·빌드툴을 동시 구동하는 리소스 타이트 환경**에서도 가볍게 도는 전용 관리자가 필요.
* **핵심 가치 (Core Values):**
  1. **Ultra-Low RAM** — 메모리 풋프린트 최소화 (Target RSS < 20MB, §5.1에서 측정 정의).
  2. **Privacy First** — 외부 서버 전송 0. 100% 로컬. 텔레메트리 없음.
  3. **Developer Experience (DX)** — Claude Code 흐름을 끊지 않는 빠른 접근·직관적 단축키.
  4. **Non-Destructive** — 원본 JSONL은 **읽기 전용**. 부가 정보(별칭 등)는 사이드카 분리. (부록 D)

---

## 3. 타겟 유저 및 페르소나 (Target User & Persona)

* **주요 사용자층:** Claude Code를 메인으로 쓰는 SW/DevOps 엔지니어, 터미널 선호 개발자.
* **사용자 환경:** RAM 확보에 민감(다수 컨테이너·IDE 동시 구동), 다양한 터미널(Windows Terminal/conhost, iTerm2/Terminal.app, 각종 리눅스 에뮬레이터).
* **페르소나:** "수십 개 프로젝트 폴더를 오가며 세션을 띄우는 민준. `~/.claude/projects/`에 300+ 세션이 쌓였지만 어제 그 디버깅 세션을 못 찾는다. 무거운 GUI 없이 단축키 몇 번으로 원하는 세션에 곧장 이어붙고 싶다."
* **1인 운영 전제:** 개발·유지보수 모두 1인. **운영 부담이 낮은 단순 설계**를 과잉설계보다 우선한다(예: 사이드카는 단일 JSON으로 시작).

---

## 4. 핵심 기능 요구사항 (Functional Requirements)

> **MoSCoW:** Must / Should / Could / Won't. **마일스톤:** M0(스파이크) / M1(MVP) / M2(관리) / M3(편의).

| 기능 ID | 대분류 | 상세 요구사항 | 우선순위 | 마일스톤 | 비고 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **FR-01** | 세션 스캔 | `~/.claude/projects/` 하위 모든 프로젝트 폴더의 `*.jsonl` 세션을 자동 감지·목록화. 서브에이전트(`<id>/subagents/*.jsonl`)는 기본 제외. | Must | M1 | 커스텀 루트 경로 설정 지원 (§5.5) |
| **FR-02** | 목록 조회 | 각 세션의 제목·프로젝트(cwd)·생성시각·최종수정시각·메시지 수를 평면 리스트로 표시. | Must | M1 | 제목 필드 없음 → 도출(부록 B). 메시지 수는 lazy 허용 |
| **FR-03** | 세션 이어하기 | 선택 세션을 해당 `cwd`에서 `claude --resume <sessionId>`로 이어하기. **기본=셸 핸드오프(claudeDesk 종료 후 명령 실행/exec)**, 옵션=새 창 스폰. | Must | M1 | 부록 C |
| **FR-04** | 삭제/정리 | 세션을 **소프트 삭제**(휴지통 디렉토리 이동). 다중 선택 지원. 활성 세션 삭제 차단. 영구삭제는 별도 2단계 확인. | Should | M2 | 부록 E·F, FR-11과 연동 |
| **FR-05** | 검색/필터 | 제목·첫 프롬프트·cwd 키워드로 incremental 검색. 전문(full-text)은 범위 외(부록 H). | Should | M2 | 부록 B·H |
| **FR-06** | 별칭(이름변경) | 세션에 사용자 별칭 지정. 사이드카 저장, 원본 불변. | Could | M3 | 부록 D |
| **FR-07** | 정렬 | 최종수정/생성/제목/메시지수 기준 토글. 기본=최종수정 내림차순. | Should | M2 | — |
| **FR-08** | 미리보기 | 선택 세션의 첫 user 메시지 스니펫을 패널에 표시. 전체 로드 없이 스트리밍. | Could | M3 | 부록 B |
| **FR-09** | 프로젝트 그룹핑 | cwd 단위로 섹션/트리 그룹 표시(접기/펼치기). | Should | **M2** | MVP에서 제외(v2 강등). 부록 B |
| **FR-10** | 설정 | 커스텀 루트 경로·시간 표기·삭제 보관일 등 설정. TOML 파일 + 인앱 설정 화면. | Could | M3 | §5.5 |
| **FR-11** | 휴지통/복구 | 소프트 삭제된 세션 목록 조회·복구·영구삭제. 보관기간 경과분 자동 정리. | Should | M2 | FR-04와 한 쌍 |
| **FR-12** | 에러 가시성 | 파싱 스킵된 줄/파일 수, 권한 오류 등을 상태바·로그로 노출. | Should | M1 | §5.4, 부록 E |
| **FR-13** | 도움말 | 단축키 도움말 오버레이(`?`). | Should | M1 | [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md) |

### 4.1 우선순위 결정 근거 (압축)

* **M1 Must = FR-01·02·03·12·13** → "찾아서 이어붙는다"의 최소 가치 + 조용한 실패 방지 + 사용성. 그룹핑·검색·삭제 없이도 단독으로 유용.
* **그룹핑(FR-09) 강등 사유:** 트리 상태(접힘/선택 인덱스 매핑)는 평면 리스트보다 렌더·키핸들 복잡. MVP 리스크 대비 가치 낮음 → M2.
* **별칭/미리보기/설정(FR-06·08·10) = Could:** 편의 기능, 코어 검증 후.

---

## 5. 비기능적 요구사항 (Non-Functional Requirements)

### 5.1 성능·리소스 (측정 정의 포함)

* **RAM(하드 타깃):** **상주 RSS < 20MB**.
  * **측정 정의:** 300개 세션 환경에서 앱 기동 → 목록 렌더 완료 → 60초 idle 후 OS 도구로 측정한 **RSS(Resident Set Size)**. (Win: 작업관리자/`Get-Process`의 WorkingSet64, Linux: `/proc/<pid>/status` VmRSS, macOS: `ps -o rss`.)
  * **밴드 통일(모순 제거):** idle 상주 RSS ≤ 20MB(목표) / ≤ 25MB(허용 상한). 미리보기·검색 등 일시 피크 시에도 ≤ 35MB. v1.1.0의 "15MB/30MB" 별도 밴드는 폐기.
  * **게이트:** §8 M0 스파이크에서 **실측으로 통과 여부 판정**(가정 아님).
* **응답 속도(측정 정의):**
  * **목록 초기 로딩 ≤ 300ms** @ 300 세션, 기준선 = NVMe SSD, 일반 개발 노트북. (v1.1.0 "200ms"는 하드웨어 미명시로 측정 불가 → 조건 명시 + 보수적 300ms로 조정. 스파이크 실측 후 재확정.)
  * **키 입력 응답(렌더 프레임) ≤ 50ms** 체감.
  * 측정: 내장 `--bench` 모드(개발용)로 콜드/웜 캐시 각각 N회 중앙값.
* **확장성:** 1,000 세션까지 선형. 메시지 수·전체 파싱은 lazy/온디맨드.

### 5.2 신뢰성·호환성

* **비공식 포맷 방어:** JSONL 스키마는 비공식 → **모르는 필드 무시 + 필수 필드 누락 시 graceful skip**. `version` 필드 감지 로깅. 호환성 픽스처 보관(§5.11).
* **OS:** Windows 10+, macOS 12+, 주요 Linux. 터미널: Windows Terminal/conhost(부록 J Q3), iTerm2/Terminal.app, xterm 계열.
* **무중단 원칙:** 손상 줄 1개가 전체 목록 실패를 유발하지 않음.

### 5.3 보안·프라이버시

* 세션 내용·경로·키를 **외부 전송 0**. 네트워크 호출 없음(업데이트 체크조차 기본 off, §5.9).
* 사이드카·휴지통·로그는 모두 로컬(`~/.claude/claudedesk/`).

### 5.4 로깅 (신설)

* **위치:** `~/.claude/claudedesk/logs/claudedesk.log` (단일 파일, 회전 1MB×3).
* **레벨:** error/warn/info/debug. 기본 warn. `--verbose`로 상향, `CLAUDEDESK_LOG` 환경변수 지원.
* **내용:** 파싱 스킵 카운트(파일/줄), 권한 오류, resume 호출 결과, 사이드카 쓰기. **세션 본문은 로그에 남기지 않음**(프라이버시).
* **UI 연동:** FR-12 상태바에 스킵 수 요약, 상세는 로그.

### 5.5 설정 (신설, FR-10)

* **파일:** `~/.claude/claudedesk/config.toml` (TOML, 단순/주석 가능). 없으면 기본값 생성.
* **키(초안):**
  ```toml
  projects_root = "~/.claude/projects"   # 커스텀 루트
  default_sort  = "modified_desc"         # modified|created|title|messages + asc/desc
  time_format   = "relative"              # relative | absolute(ISO/locale)
  resume_mode   = "handoff"               # handoff | spawn
  trash_retention_days = 30
  active_window_secs   = 90               # 활성 세션 mtime 근접 임계
  include_subagents    = false
  theme = "auto"                          # auto | dark | light | mono
  ```
* 인앱 설정 화면(M3)은 위 파일을 읽고/쓰는 뷰. 우선순위: 파일이 단일 진실원본.

### 5.6 국제화·시간 표기 (신설)

* **UI 언어:** 초기 한국어 우선, 문자열 테이블 분리로 영어 확장 여지(과잉 i18n 프레임워크 금지 — 단순 키맵).
* **시간:** `relative`(예: "2분 전")는 한국어 라벨, `absolute`는 로컬타임존 ISO. `time_format` 설정으로 토글.
* **인코딩:** UTF-8 고정. 실측에서 본 surrogate/이모지·다국어 본문 깨짐 방지(렌더 시 grapheme 폭 계산, `unicode-width` 사용).

### 5.7 접근성·컬러무관 (신설, TUI)

* **컬러무관:** 상태(활성/선택/삭제대상)를 **색 + 기호(마커)** 이중 표기. 색맹/모노 터미널에서도 식별. `theme=mono` 제공.
* **스크린리더/내로우:** 단축키 전부 키보드로 도달. 핵심 정보는 텍스트 라벨로도 제공.
* **NO_COLOR** 환경변수 존중(색 비활성).

### 5.8 패키징·배포 (신설)

* **산출물:** OS·아키별 **단일 정적 바이너리**. 타깃(v0.3.0 확정): `x86_64-pc-windows-msvc`, `aarch64-apple-darwin`, `x86_64-unknown-linux-musl`(정적). **macOS Intel(`x86_64-apple-darwin`)은 제외**(사용자 결정 2026-06-25, Intel 러너 희소·수요 낮음 — Apple Silicon으로 대체).
* **배포:** GitHub Releases(바이너리 첨부) 1순위. 패키지 매니저(brew/scoop/cargo-binstall)는 후순위.
* **버전:** SemVer. 버전 파일 단일화. (운영 부담 최소 — CI는 release 태그 시 빌드.)

### 5.9 업데이트 (신설)

* **기본 off.** 자동 업데이트·온라인 버전체크 없음(프라이버시·단순성). `--check-update` 수동 옵션만 GitHub Releases API 조회(opt-in).

### 5.10 CLI 인자 (신설)

```
claudedesk [--root <path>] [--sort <key>] [--no-color] [--verbose]
           [--bench] [--check-update] [--version] [--help]
           [--config <path>]
```
* 인자는 config.toml을 **오버라이드**. 비대화 진단용 `--list`(stdout로 세션 목록 출력, TUI 미진입)도 Could.

### 5.11 테스트 전략 (신설)

* **유닛:** 파서(부록 B 도출 로직), 폴더명 역치환, 시간 포맷, 사이드카 read/write 원자성.
* **픽스처:** 실측 기반 JSONL 픽스처 세트 — 정상/메타선행/손상줄/빈파일/블록배열 content/surrogate. (비공식 포맷 회귀 방지.)
* **통합:** 임시 `projects_root`에 픽스처 배치 → 스캔→목록 결과 스냅샷.
* **수동:** resume 핸드오프(OS별 1회), TUI 렌더(좁은 터미널/모노).
* **벤치:** `--bench`로 RAM·로딩시간 게이트 자동 측정(M0 산출).
* **원본 불변(CI 필수):** 픽스처 원본 `*.jsonl`의 SHA-256을 골든값으로 고정하고, 모든 작업(스캔/목록/검색/삭제 시뮬레이션) 전후로 대조하는 테스트를 **모든 PR의 required check**로 강제(수동 가정 아님). 데이터 유실은 복구 불가 1순위 사고. (03_DEV_KICKOFF §9.4)
* **시크릿/본문 유출:** 픽스처는 **합성 데이터만**(실제 cwd·세션 본문 금지) + CI `gitleaks` 스캔.

---

## 6. 제약 사항 및 기술 스택 (확정)

* **제약:** 무거운 런타임(Node/Electron) 금지. GC 베이스라인이 큰 런타임은 <20MB 하드타깃에 불리. 단일 정적 바이너리.

### 채택 스택: **Rust + ratatui(crossterm) + serde_json**

* **언어:** Rust — GC 없음, 단일 정적 바이너리, 크로스플랫폼, 메모리 안전.
* **TUI:** `ratatui`(crossterm 백엔드).
* **파싱:** `serde_json` + `BufReader` 라인 스트리밍. **주의:** 첫 user 메시지는 줄 선두가 아닐 수 있어(실측) **첫 K줄 스캔 가드** 적용(부록 B).
* **부가 크레이트(권장):** `serde`/`serde_json`, `directories`(경로), `unicode-width`(렌더폭), `toml`(설정), `chrono` 또는 경량 시간 처리, `anyhow`/`thiserror`(에러). **무게 주의 — 최소 의존성 원칙.**
* **사이드카:** 단일 JSON `~/.claude/claudedesk/meta.json`(원자적 temp+rename). SQLite 전환은 규모 임계 도달 시(부록 J Q1).
* **경량 인덱스 캐시(조건부):** 매 기동 시 전량 스캔 비용이 문제될 때만 `mtime+size` 기반 인덱스(`~/.claude/claudedesk/index.json`) 도입 검토. **단 M0 벤치에서 로딩 ≤300ms를 만족하면 불필요한 복잡도이므로 보류**(과잉설계 금지). 즉 "M0 벤치 미달 시 조건부 도입".

### 대안 비교 (요약)

| 후보 | RAM | 배포 | TUI 생태계 | 판정 |
| :--- | :--- | :--- | :--- | :--- |
| **Rust + ratatui** | ◎ | ◎ 단일 바이너리 | ◎ | **채택** |
| Go + Bubble Tea | △ GC 15~40MB | ◎ | ○ | 타깃 마진 부족 — 기각 |
| Zig | ◎ | ○ | △ | 생태계 열위 — 기각 |
| C++ + FTXUI | ○ | △ | ○ | 안전성·DX 열위 — 기각 |
| Python(Textual) | ✗ 인터프리터 | △ | ○ | 하드타깃 위배 — 기각 |

---

## 7. 유저 저니 (요약)

실행 → `projects_root` 자동 스캔 → 평면 리스트 출력(프로젝트 컬럼) → 방향키/`/`검색/`s`정렬로 좁히기 → `Enter`로 선택 세션 resume(핸드오프). 상세 화면·키맵·와이어프레임은 [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md).

---

## 8. 출시 기준 및 마일스톤

### 성공 지표 (검증 가능)

* **M0 게이트:** 300 세션 실측 RSS < 20MB(허용 ≤25MB) **AND** 목록 로딩 ≤ 300ms. 미달 시 의존성 다이어트 → 재측정, 그래도 미달 시 스택 폴백 의사결정.
* **안정성:** 손상 줄/빈 파일 픽스처에서 크래시 0(graceful skip). 24h idle RSS 증가 < 2MB(누수 가드).
* **무결성(CI 필수 게이트):** 모든 동작 후 원본 JSONL 바이트 불변. SHA-256 대조 테스트를 **모든 PR의 required check**로 강제(수동 가정 아님 — §5.11, 03_DEV_KICKOFF §9.4).

### 마일스톤

* **M0 — 스파이크:** ratatui 빈 앱 + 대용량 JSONL 라인 스트리밍 파싱 + **RAM/로딩 실측(`--bench`)**. 게이트 통과가 곧 스택 확정.
* **M1 — MVP:** FR-01·02·03·12·13. 평면 리스트 → 선택 → resume(핸드오프). 에러 가시성·도움말 포함.
* **M2 — 관리:** FR-04·05·07·09·11. 검색·정렬·소프트삭제·휴지통·그룹핑.
* **M3 — 편의:** FR-06·08·10. 별칭·미리보기·설정 화면.

마일스톤별 Task·Done 기준은 [01_TASK_BREAKDOWN.md](01_TASK_BREAKDOWN.md), 진입점은 [03_DEV_KICKOFF.md](03_DEV_KICKOFF.md).

---
---

# 부록 (Technical Appendix)

> 부록의 기술 사실은 `~/.claude/projects/` **실측**(2026-06-25)과 `claude --help` 확인에 근거. 비공식 포맷이므로 버전 변동 가능성 전제.

## 부록 A. 세션 데이터 모델 & 스토리지

### A.1 디렉토리 구조
```text
~/.claude/projects/
├── D--Dev-claudeDesk/                      # 프로젝트(cwd) 단위 폴더
│   ├── 4bf02f8c-2370-4906-b145-2518877fe1e6.jsonl   # 세션(파일명=sessionId)
│   ├── <another-uuid>.jsonl
│   └── <session-uuid>/subagents/agent-<id>.jsonl    # 서브에이전트(기본 제외)
├── C--Users-PC/ ...
```

### A.2 폴더명 치환 규칙
* 폴더명 = cwd 절대경로의 `/`·`\`·`:`를 `-`로 치환. 예: `D:\Dev\claudeDesk` → `D--Dev-claudeDesk`.
* **표시 1순위는 각 줄의 `cwd` 필드**(원본 경로 정확). 역치환은 cwd가 없을 때 보조.

### A.3 세션 파일 = JSONL — 실측 보강
* 1 파일 = 1 세션. 파일명(확장자 제외) = `sessionId`(UUID). 줄당 JSON 1객체, 시간순 append.
* **실측(2026-06-25):** 한 세션 140줄 중 **0~6번 줄이 메타/첨부**(`agent-setting`, `mode`, `permission-mode`, `file-history-snapshot`, `attachment`)이고 **7번 줄이 첫 `type:"user"`**. 즉 **첫 줄 ≠ user**. 일부 메타 줄은 `{agentSetting, sessionId, type}`처럼 `timestamp`·`message`가 없다.

#### 줄 공통 스키마(관측 필드 — 비공식)
| 필드 | 타입 | 설명 |
| :--- | :--- | :--- |
| `type` | string | `user`/`assistant`/`attachment`/`agent-setting`/`mode`/`permission-mode`/`file-history-snapshot` 등 |
| `sessionId` | string(UUID) | 세션 식별자(=파일명) |
| `timestamp` | string(ISO8601)\|없음 | 일부 메타 줄엔 부재 → mtime 폴백 |
| `cwd` | string | 작업 디렉토리 절대경로(메타 줄엔 없을 수 있음) |
| `gitBranch` | string | 당시 브랜치 |
| `version` | string | Claude Code 버전 |
| `message` | object | `{role, content}`. **`content`는 문자열 또는 블록 배열**(`[{type:"text",text:...}, ...]`) |
| `uuid`/`parentUuid` | string | 레코드/부모 UUID(대화 트리) |
| `entrypoint`/`userType`/`isSidechain` | 기타 | 부가 메타 |

> 파서 원칙: 모르는 필드 무시, 필수 누락 시 graceful skip(부록 E·F).

## 부록 B. 세션 제목·메타 도출 로직 (실측 정정)

* **제목 도출 우선순위:**
  1. 사이드카 별칭(FR-06)이 있으면 그것.
  2. 없으면 **첫 `type:"user"` 줄의 `message.content`에서 첫 `text`를 추출** → 트림 → 앞 N자.
     * `content`가 **문자열**이면 그대로. **블록 배열**이면 첫 `{type:"text"}` 블록의 `text`.
     * 추출 텍스트가 비표시(첨부 메타/명령 스텁)면 다음 user 줄로 폴백.
  3. 최종 폴백 `Untitled Session`.
* **첫 user 줄 탐색(성능 가드 — v1.1.0 정정):** 첫 줄만 읽는 가정은 틀림(메타 선행). **앞에서부터 최대 `K`줄(기본 K=64) 또는 첫 user `text` 발견까지** 스트리밍 스캔. K 초과 시 `Untitled`로 처리하고 로그. 이래도 파일당 수십 줄·수 KB 수준으로 200~300ms 타깃 내.
* **표시 메타:**
  * 프로젝트/경로: 줄 `cwd`(1순위) · 폴더명 역치환(보조).
  * 생성시각: 첫 `timestamp` 보유 줄, 없으면 파일 ctime.
  * 최종수정: 파일 **mtime**(stat만, 파싱 불필요 — 정렬·표시 핵심).
  * 메시지 수: `type∈{user,assistant}` 줄 카운트. 대용량은 lazy(선택 시 계산) 허용.
* **검색(FR-05) 범위:** 제목·첫 user 텍스트·cwd. 전문 인덱싱은 범위 외(부록 H).

## 부록 C. 세션 이어하기(Resume) 연동 — 전략 전환

* **CLI 계약(실측 `claude --help`):**
  * `claude -r, --resume [value]` — sessionId로 이어하기(인자 없으면 선택 UI).
  * `claude -c, --continue` — 가장 최근 대화 이어하기.
  * (`--print`/`--output-format` 등은 비대화 모드용 — resume 인터랙티브엔 불필요.)
* **기본 전략 = 셸 핸드오프(handoff):** claudeDesk가 선택 세션의 `cwd`로 이동 후 **자신을 종료하며 `claude --resume <id>`로 프로세스를 교체(unix: exec, Windows: 부모 셸에 명령 위임/래퍼 스크립트)**. 새 창 미사용 → cwd·env·터미널 승계가 자연스럽고 신뢰성↑.
  * **구현 노트:** 완전한 in-place `exec`가 어려운 플랫폼(Windows)은 **claudeDesk 종료 → 셸이 출력한 명령/래퍼가 `claude` 실행** 패턴 또는 child process로 `claude`를 띄우고 claudeDesk는 TUI를 양보(raw mode 해제 후 상속). M1에서 OS별 1방식 확정.
* **옵션 전략 = 새 창 스폰(`resume_mode="spawn"`, Could):** Win `wt.exe`, macOS `open -a`, Linux `$TERMINAL`. 신뢰도 낮아 옵션으로만.
* **실패 처리:** `claude` 부재(PATH) 시 안내 + 명령 클립보드/화면 출력 폴백.

## 부록 D. 별칭·메타 사이드카

* **원칙:** 원본 JSONL 불변. 별칭/핀/태그/색상은 사이드카에만.
* **저장:** `~/.claude/claudedesk/meta.json` — `{ "<sessionId>": { "alias", "pinned", "tags":[] } }`.
* **쓰기 원자성:** temp 파일 작성 → `rename`(동시 실행·중단 안전).
* **키:** `sessionId`(불변). 원본 이동/삭제돼도 정합 관리 용이. 고아 항목은 주기적 정리(또는 표시).

## 부록 E. 엣지 케이스 & 에러 처리

| 케이스 | 처리 |
| :--- | :--- |
| 손상/부분 JSONL 줄 | 줄 단위 파싱, 깨진 줄 graceful skip + 카운트(FR-12 노출·로그) |
| **첫 줄이 메타(user 아님)** | 첫 user 줄까지 K줄 스캔(부록 B). K 초과 시 Untitled |
| `content`가 블록 배열 | 첫 `text` 블록 추출, 없으면 폴백 |
| 활성(진행 중) 세션 | **mtime 근접(≤active_window_secs, 기본 90s) 휴리스틱**으로 "활성" 표시 + 삭제/이동 차단. 락은 보조 |
| 빈 세션/메시지 없음 | `Untitled`, 삭제 후보 분류 |
| 초대용량 파일 | 전체 로드 금지(첫 user 줄+mtime). 미리보기/검색만 스트리밍 |
| 경로 부재/권한 오류 | 경고 + 빈 목록 또는 커스텀 경로 입력 유도(FR-12 로그) |
| 동시 실행(claudeDesk 2개) | 사이드카·휴지통 쓰기 원자적(temp+rename), 읽기 위주 |
| 휴지통 보관기간 경과 | FR-11에서 자동 영구삭제(기본 30일) |

## 부록 F. 리스크 레지스터

| 리스크 | 영향 | 대응 |
| :--- | :--- | :--- |
| 비공식 포맷 변경 | 파싱 실패 | 방어적 파싱, `version` 감지, 회귀 픽스처(§5.11) |
| CLI 옵션 변경 | resume 깨짐 | 기동 시 `claude --resume` 가용성 점검, 실패 안내 |
| 파괴적 삭제 사고 | 데이터 유실 | 소프트삭제+휴지통+30일+2단계 확인(FR-04·11) |
| RAM 타깃 미달 | 핵심가치 훼손 | M0 실측 게이트, 의존성 다이어트, 스택 폴백 |
| 활성세션 오탐 | 잘못된 차단/허용 | 보수적 차단(오탐 시 안전측), 휴리스틱+락 병행 |
| Windows resume 핸드오프 난이도 | 핵심기능 불안정 | M0/M1에서 OS별 방식 1개 확정·수동검증 |

## 부록 G. 가정·의존성

* `claude`가 PATH에서 실행 가능. 세션 경로 `~/.claude/projects/`(또는 설정 경로) 존재.
* `claude --resume <id>` / `-c` 계약 유지. OS = Win/macOS/Linux + 터미널 에뮬레이터.

## 부록 H. 범위 외 (Won't — this version)

* 클라우드 동기화/원격 백업. 세션 **내용 편집**(읽기 전용 위배). 다중 머신 머지.
* 웹 UI/Electron. **전문(full-text) 인덱싱**(초기 제목/첫 프롬프트/cwd 한정). 텔레메트리.
* 자동 업데이트(수동 opt-in만). 서브에이전트 로그 관리(기본 제외, 후순위).

## 부록 I. 용어 정의

| 용어 | 정의 |
| :--- | :--- |
| 세션 | 1 Claude Code 대화 단위 = 1 `.jsonl` |
| sessionId | 세션 UUID(=파일명) |
| JSONL | 줄당 JSON 1객체 |
| cwd 치환 | cwd의 `/`·`\`·`:`→`-` 폴더명 규칙 |
| 사이드카 | 원본 불변, 부가메타 별도 파일 |
| 핸드오프(handoff) | claudeDesk 종료 후 셸/exec로 `claude` 실행하는 resume 방식 |
| 소프트삭제 | 휴지통 디렉토리로 이동(영구삭제 아님) |
| 스파이크 | 가설(RAM<20MB) 검증 PoC |
| TUI | 터미널 기반 UI |

## 부록 J. 미해결 질문 (Open Questions) — CEO 결정 필요

1. **사이드카 SQLite 전환 임계.** 단일 JSON 시작 확정. 전환 트리거(예: 세션 1,000개 또는 검색 지연)를 수치로 둘지? → 기본: 보류, 필요 시 도입. **(CEO 확인)**
2. **Resume 핸드오프 OS별 정확 방식.** Windows에서 in-place exec 불가 → child spawn vs 래퍼 스크립트 vs "명령 출력 후 종료" 중 기본값? → M0/M1 스파이크로 결정. **(M1 차단요소)**
3. **Windows conhost 지원 범위.** 구형 conhost까지 vs Windows Terminal 권장? → 기본: Windows Terminal 권장, conhost는 best-effort. **(CEO 확인)**
4. **활성 세션 판정.** mtime 근접(기본 90s) 휴리스틱으로 v2 확정. 임계값 설정화함. 락 감지는 보조. **(해소, 값 튜닝만)**
5. **휴지통 보관기간 기본값.** 30일로 확정(설정화). **(해소)**
6. **(신규) 메시지 수 표시 정책.** 모든 세션 즉시 카운트(스캔 비용) vs 선택 시 lazy? → 기본 lazy, M0 벤치로 즉시카운트 가능성 재평가. **(M0 입력)**
