# Changelog

본 프로젝트는 [Semantic Versioning](https://semver.org/lang/ko/)을 따른다.
형식은 [Keep a Changelog](https://keepachangelog.com/ko/) 기반.

## [Unreleased]

## [0.16.0] - 2026-06-30

### Added
- **Light 테마 팔레트:** `Palette` 구조체(`dark` / `light` / `mono`)를 도입해 `Theme::Light` 선택 시 라이트 배경 터미널에 최적화된 색상 적용. `accent`=Blue, `body`=Black, `user_msg`=Blue (Dark 대비 Cyan→Blue, White→Black 전환). Mono는 `enabled=false`로 색상 전체 무효화 유지 (`src/ui/theme.rs`).
- **`Palette` 일원화:** 전체 UI 레이어(`list`, `facet_view`, `modal`, `settings`, `help`, `preview`, `trash_view`)에서 `color_enabled: bool` 단일 플래그를 `Palette`로 교체 — `palette.fg(palette.<semantic>)` 호출로 테마별 색상 분기 단순화.

### Tests
- 185 테스트 통과, clippy `-D warnings`·fmt 그린.

## [0.15.0] - 2026-06-30

### Changed (코드 품질 A + 접근성 B)
- **Health 아이콘 NO_COLOR fallback:** `NO_COLOR`/`mono` 모드 시 이모지(`⏰`/`💀`) 대신 ASCII 기호(`~`/`!`)로 fallback. 색 없이도 Stale·Zombie 구분 가능 (`src/ui/facet_view.rs`).
- **facet_view.rs 단순화(code-simplifier):** `render()` 2-pane 중복 분기 제거(단일 `left_pct` 계산으로 통합), `render_right()` preview fallback 중복 제거(`match (preview_open, preview_content)` 단일 경로), WHAT 주석 정리. 동작 변경 없음.
- **접근성 체크리스트 `[x]` 업데이트:** `docs/02_UIUX_DESIGN.md` §8 6항목 모두 구현 완료 확인·표시.

### Tests
- 185 테스트 통과, clippy `-D warnings`·fmt 그린.

## [0.14.0] - 2026-06-30

### Added (facet 뷰 UX 소형 패치 — UIUX §2.1/§2.2 정합)
- **Sort 상태 표시:** facet 탭바 2번째 줄에 `Sort: Modified ↓  |  N세션` 정보 표시. 현재 정렬 기준·방향을 키보드 진입 없이 바로 확인 가능 (`src/ui/facet_view.rs`).
- **검색바 힌트 우측 정렬:** `/` 검색바를 3구역 레이아웃으로 분리. 우측에 `(N건 · Esc 취소)` 힌트 표시. UIUX 설계서 §2.2 형식과 일치.
- **facet 좌측 패널 상태바:** Normal/Search 모드별 키 힌트를 하단 1줄에 표시. 그룹 모드 여부에 따라 Tab 접기/펼치기 힌트 동적 포함. 임시 상태 메시지 우선 표시 지원.

### Tests
- 185 테스트 통과, clippy `-D warnings`·fmt 그린.

## [0.13.0] - 2026-06-29

### Added (검색 UX 개선 — 이슈 #41)
- **facet 검색바:** `/` 키로 검색 모드 진입 시 facet 탭 아래 검색바 렌더. 입력 중인 쿼리와 매칭 건수(`N건`)를 실시간 표시. 기존 `_search_mode` 미사용 파라미터가 실제 렌더링에 연결됨 (`src/ui/facet_view.rs`).
- **매칭 강조(UNDERLINED):** 검색 쿼리와 일치하는 제목 부분에 밑줄 스타일 적용. 대소문자 무시·원본 대소문자 보존. `NO_COLOR`/Mono 테마에서도 동작 (색 무관, §5.7).
- **`highlight_query()` 헬퍼:** 쿼리 위치를 찾아 before/match/after 3-span `Line` 반환. 매치 없으면 plain text 반환. 단위 테스트 6종 추가.

### Tests
- `highlight_query` 단위 테스트 6종 신규 추가. 총 185 테스트 통과, clippy `-D warnings`·fmt 그린.

## [0.12.1] - 2026-06-29

### Fixed (코드리뷰 부채 해소 — 이슈 #5)
- **H-1 파서 정확도:** `scan_count`가 유효 JSON 줄만 카운트하도록 수정. 파싱 실패 줄이 K줄 상한에 포함되던 문제 제거 (`src/parser.rs`).
- **H-2 cwd 역치환 회귀 테스트:** cwd가 메타 줄에만 있고 user/assistant 줄엔 없을 때 cwd 추출이 정상 동작하는 픽스처(`tests/fixtures/cwd_meta_only.jsonl`) + 회귀 테스트 추가.
- **H-3 exec_resume verbose warn:** `cwd`가 비어있을 때 현재 디렉토리에서 실행한다는 warn 로그 추가 (`src/service.rs`).
- **M-2 말줄임표 폭 동적 계산:** `safe_truncate` / `middle_truncate`에서 `…` (U+2026) 폭을 하드코딩 1 대신 `UnicodeWidthStr::width()` 동적 계산으로 교체. East Asian Ambiguous 터미널 대응 (`src/ui/list.rs`).
- **N-3 PATH vs claude 미설치 구분:** `which_claude()`에서 PATH 환경변수 부재와 claude 바이너리 미존재를 구분한 warn 메시지 출력 (`src/service.rs`).
- **N-4 "어제" 달력 날짜 비교:** `relative_time()`의 "어제" 판정을 48h 휴리스틱에서 로컬 달력 날짜 비교(`NaiveDate`)로 교체. 자정 전후 불일치 제거 (`src/ui/time.rs`).
- **M-5 확인 완료:** NO_COLOR 지원은 v0.10.0에서 이미 구현됨 (`config.color_enabled()`).

### Tests
- cwd_meta_only 회귀 테스트 신규 추가. 총 179 테스트 통과, clippy `-D warnings`·fmt 그린.

## [0.12.0] - 2026-06-29

### Added (Facet 2-pane UI 재설계)
- **Health 분류(stale_cutoff 90일):** `Health` enum(Active/Empty/Stale/Zombie) + `classify()` 순수 함수로 세션 상태를 자동 분류. 90일 이상 수정되지 않은 비활성 세션을 `Stale`로 표시, 빈 세션은 `Empty`, 현재 이용 중이면 `Active`, 복구 불가능(원본 경로 삭제됨)하면 `Zombie`.
- **Facet 뷰:**  `Facet` enum(Recent/Active/Cleanup/Project) + `facet_indices()`·`matches()`·`counts()` 유틸로 세션을 4개 탭으로 분류. 추가로 `cursor_identity` anchor 구현으로 탭 전환 후 커서 위치 유지.
- **2-pane 레이아웃:** `src/ui/facet_view.rs`로 화면 크기별 반응형 레이아웃 — single(<90) / narrow(<120) / full(≥120). 좌측 facet 탭바(1~4로 직접 점프 + Tab/Shift+Tab 순환), 우측 세션 목록 패널. 단일 탭 모드(narrow/single)에서도 탭 표시로 탐색 지원.
- **Health 아이콘:** 목록에서 각 세션 앞에 health 상태를 시각 마커(`●` Active / `⏰` Stale / `○` Empty / `💀` Zombie)로 표시(색 무관, 텍스트 기반 식별).
- **설정 통합:** `src/config.rs`에 `stale_days`(기본 90) + `default_facet` 필드 추가로 기준일과 초기 탭을 커스터마이징 가능.
- **키 바인딩:** `Tab`/`Shift+Tab`으로 facet 순환, `1`~`4` 키로 Recent/Active/Cleanup/Project 직접 점프.
- **FS 자동 감지(auto-reload):** notify 6.x watcher가 `~/.claude/projects/` 변경을 감시. 300ms 디바운스 후 세션 목록 자동 갱신 — 탭/커서/검색/정렬 상태 유지, `cursor_identity`로 커서 복원.
- **`--facet` CLI 인자:** `claudedesk --facet recent|active|cleanup|project`로 시작 탭 지정 (미지정 시 `config.toml`의 `default_facet`).
- **Service 배선:** `src/service.rs`에서 health 분류 패스 + `restore_cursor()` + `start_watcher()` 구현으로 렌더링 성능과 탐색 UX 개선.
- **Domain 확장:** `src/domain.rs`의 `Session`에 `health` 필드 추가.

### Tests
- facet 분류 유닛 테스트(4개 카테고리·경계일·cursor 보존) + health 분류 회귀(Stale/Active/Empty/Zombie) + 레이아웃 반응성 테스트 추가. 총 168 테스트 통과, clippy `-D warnings`·fmt 그린.

## [0.11.0] - 2026-06-26

### Changed (CI / 비대화 모드 / resume)
- **GitHub Actions checkout v4→v5:** Node20 deprecation 대응. 빌드 파이프라인 최신화.
- **비대화 모드(`--list`):** 명령줄에서 `claudedesk --list`로 7컬럼 TSV 형식(ID/Title/Cwd/Modified/Status/Health/Project)을 stdout으로 출력. TUI 없이 스크립트/파이프 조회 지원.
- **`resume_mode=spawn` 구현:** `config.toml`에서 `resume_mode = "spawn"` 설정 시 세션 재개를 `claude --resume` 대신 새로운 spawn 기반 메커니즘으로 실행. 기존 handoff 제어 흐름은 유지하되, 프로세스 생성 방식을 교체.

### Tests
- CLI 출력(`--list` TSV 포맷) 통합 테스트 2종 + resume_mode 선택 로직 유닛 1종. 기존 86 테스트 유지, 회귀 0.

## [0.10.0] - 2026-06-26

### Added (M3 완성 — 설정 FR-10)
- **설정 파일(`config.toml`):** `~/.claude/claudedesk/config.toml`에서 기본 동작을 읽는다. 없으면 기본값으로 **원자적 생성**(temp→rename). 손상·부분 누락 TOML은 `serde(default)`로 graceful 폴백 — 패닉 없이 기본값 복원. 기록 위치는 `projects/`와 **분리된 디렉토리**라 원본 세션 JSONL은 손도 대지 않는다.
- **인앱 설정 화면(`,` 키):** Projects root / Default sort / Time format / Resume mode / Trash retention / Theme를 ↑↓ 이동·←→·Enter 토글·`s` 저장·Esc 닫기로 편집. draft 복사본에 변경을 누적했다가 `s`에서만 영속 — 저장 성공 시에만 런타임 반영(쓰기 실패 시 파일/런타임 불일치 방지).
- **CLI 오버라이드:** `--root` / `--sort` / `--no-color` / `--config` / `--verbose`가 설정 파일 값을 덮어쓴다.
- **테마/접근성:** `color_enabled()`(Theme=Mono 또는 `NO_COLOR` 환경변수) 경유. 무색 모드에서도 목록·설정·모달 전부 텍스트 마커(●/✓/▸/⚠)로 식별 가능(§5.7 색 무관 원칙).
- **default_sort·time_format 배선:** 초기 정렬을 설정값으로 적용(저장 즉시 재정렬), 시간 표기는 Relative(한국어)/Absolute(로컬 ISO) 디스패치로 목록에 즉시 반영.

### Safety
- **원본 JSONL 불변:** 설정은 별도 `config.toml`에만 기록 — 원본 `*.jsonl`은 읽기조차 변경 0. 픽스처 SHA-256 불변 회귀 8종으로 강제.

### Deferred (문서화된 의도적 보류)
- `resume_mode=spawn` 배선(PRD Could·저신뢰 — handoff 유지), light 테마 팔레트(dark와 동일, 최소 구현).

### Tests
- config 유닛(기본생성·파싱·라운드트립·CLI 우선순위·color_enabled) + 설정화면/시간 유닛. 총 144 테스트(유닛 94 + 통합 50) 통과, clippy `-D warnings`·fmt 그린.

### Milestone
- 이로써 **M3(편의 기능 — FR-06 별칭 / FR-08 미리보기 / FR-10 설정 / FR-14 오래된 세션 정리) 전부 구현 완료.**

## [0.9.0] - 2026-06-26

### Added (하우스키핑 — 날짜 기준 오래된 세션 정리 FR-14)
- **오래된 세션 일괄 선택(FR-14):** `o`로 모달을 열어 기준일(7/30/90/180/365일)을 고르면 그 이전에 수정된 비활성 세션이 한 번에 다중선택된다. 각 기준의 **대상 개수를 모달에 미리 표시**해 고르기 전에 규모를 가늠. 세션이 쌓여도 "오래된 것만 골라 정리"를 한 동작으로.
- **새 삭제 경로 없음 = 안전핀 보존:** 선택만 채우고 실제 삭제는 기존 `d`→삭제 확인 모달→소프트 삭제(휴지통) 흐름에 그대로 위임한다. 자동 삭제·영구삭제 트리거 없음(선택 ≠ 삭제). 활성 세션은 방어적으로 선택 대상에서 제외, 현재 검색 필터 범위에서만 동작(`a` 전체선택과 동일 스코프).

### Tests
- `older_than_ids`/`select_older_than` 유닛 4(오래된 것만 선택·활성 제외·기존선택 보존·검색 필터 존중) 추가. clippy `-D warnings`·fmt 그린.

## [0.8.0] - 2026-06-26

### Added (UX — 작업 폴더 가시성 ①)
- **풀 경로 노출:** 세션이 실행된 작업 디렉토리(cwd)를 더 이상 Project(마지막 세그먼트)로만 보지 않는다. ① 미리보기 패널 상단에 **전체 경로**를 노란 볼드 헤더로 표시(긴 경로는 줄바꿈), ② 상태바 기본 모드에 커서 세션의 경로를 **중간 생략(`앞…뒤`)** 으로 항상 노출 — 미리보기를 열지 않아도 "어느 폴더에서 쓴 세션인지" 바로 식별.
- cwd는 기존 파싱 데이터(레코드 `cwd` 필드)에 이미 존재 — 신규 파싱·원본 접근 없음. 순수 표시 계층 변경(렌더만 수정, 도메인/서비스 무변).

### Tests
- `middle_truncate` 유닛 2(양끝 보존·폭 예산 준수, 한글 폭 포함) 추가. 총 유닛 61 + 통합(alias 5·parser 13·trash 15·기타) 전부 통과, clippy `-D warnings`·fmt 그린.

## [0.7.0] - 2026-06-26

### Added (M3 — 세션 별칭 FR-06)
- **세션 별칭(FR-06):** `n`으로 세션에 사용자 별칭을 지정/편집/삭제(인라인 모달, 80자 가드). 목록·미리보기 제목이 별칭으로 표시되고(`~ ` 텍스트 마커로 식별 — 색 무관 §5.7), 도출 원본 제목은 보존돼 빈칸 저장 시 자동 복원. 검색(FR-05)·Title 정렬(FR-07)도 별칭(`display_title`) 기준에 합성.
- **사이드카 저장:** 별칭은 `~/.claude/claudedesk/meta.json`(trash/index.json의 형제)에 `session_id → {alias}` 스키마로 저장. `trash.rs`의 temp+rename 원자적 write 패턴을 복제, 손상 시 graceful 빈 store, `version` 필드로 포맷 진화 대비.

### Safety
- **원본 JSONL 불변:** 별칭은 사이드카에만 기록 — 원본 `*.jsonl`은 읽기조차 변경 0. 별칭 set/save 전후 및 모듈 추가 후 픽스처 SHA-256 불변 회귀로 강제.

### Tests
- alias 유닛 6 + 통합 5(별칭 재로드·display_title·JSONL SHA 불변·소프트삭제→복구 후 별칭 보존·검색 결합) + Title 정렬 회귀 1 + search_text 결합 1. 총 168 테스트 통과, clippy `-D warnings`·fmt 그린.

## [0.6.0] - 2026-06-25

### Added (M3 — 미리보기 / 하우스키핑 강화)
- **미리보기(FR-08):** `p`로 우측 분할 패널 토글(터미널 ≥100칸). 커서를 옮기면 선택 세션의 대화 스니펫(user/assistant 멀티턴)이 **실시간 갱신** — `claude`를 띄우지 않고 내용을 확인하고 삭제 여부를 판단한다. 세션 하우스키핑의 마지막 조각.
- **스트리밍·RAM 바운드:** 미리보기는 파일 전체를 메모리에 올리지 않는다. `BufReader::take(MAX_BYTES=64KB)`로 파일 읽기량을 하드 실링하고, 단일 거대 줄(base64 이미지·대형 tool_result)도 `MAX_LINE_BYTES=256KB` 가드로 차단. 캡 도달 시 `…(미리보기 일부)` 표시. 세션당 캐시로 매 프레임 재읽기 방지.
- 도구 호출/결과 블록은 `[도구 호출]`/`[도구 결과]`로 축약, text 블록만 본문 표시. 검색 모드에선 `p`가 쿼리 문자로 처리(미리보기는 Normal 모드 토글).

### Safety
- 미리보기는 **읽기 전용** — 원본 JSONL에 쓰기 0. SHA-256 불변 테스트(5개 픽스처 × read_preview 전후) 포함. 210KB 단일 줄 RAM 바운드 회귀 테스트로 스트리밍 한계 고정.

### Tests
- 미리보기 통합 테스트 13종(멀티턴 추출·캡 경계·거대 줄 바운드·null content·이모지/다국어·손상 줄 skip·SHA 불변) + 유닛 5종. 총 96 테스트(51 유닛 + 16 parser + 13 preview + 15 trash + 1 doctest 경로) 통과.

## [0.5.0] - 2026-06-25

### Added (M2 — 프로젝트 그룹핑 / 포지셔닝)
- **프로젝트 그룹핑(FR-09):** `g` 평면/그룹 뷰 토글. `cwd`(작업 폴더) 단위로 세션을 묶어 헤더(`▾` 펼침 / `▸` 접힘) + 개수로 표시. 그룹 순서는 **최근 수정 내림차순**, 그룹 내부는 현재 정렬을 따른다.
- **접기/펼치기:** 그룹 헤더에서 `Tab` 또는 `Enter`로 해당 그룹 접기/펼치기. 접힘 상태는 평면↔그룹 토글 간 유지.
- **프로젝트 단위 청소:** 그룹 헤더에서 `Space` → 그 그룹의 **현재 보이는(필터된)** 세션을 일괄 선택/해제 → `Del`로 프로젝트 째로 휴지통 이동. "claude를 띄우지 않고 프로젝트 단위로 정리"를 직접 지원.
- 검색(FR-05)·정렬(FR-07)·삭제(FR-04)와 합성: 그룹 뷰는 원본 불변 view-layer(`display_rows`)로 구현, 헤더 개수는 검색 필터 결과와 일치하고 빈 그룹은 숨김.

### Changed (포지셔닝 재정의)
- README·PRD를 "resume 보조 도구" → **"세션 하우스키핑 전용 도구(`claude`를 실행하지 않고 관리·정리·삭제)"**로 재정의. `claude --resume` 내장 피커와 겹치는 검색/정렬/RAM을 헤드라인에서 내리고, 안전 삭제·휴지통·**프로젝트 그룹핑**을 핵심 가치로 승격. (PRD v2.2.0)

### Fixed
- 그룹 헤더 `Space` 해제 시 검색에 가려진(hidden) 세션의 선택까지 풀리던 비대칭(BUG-01) 수정 — 선택·해제 모두 **보이는 세션 기준**으로 대칭화. 토글 로직을 `AppState::toggle_group_visible`로 분리해 회귀 테스트 4종 추가.

### Tests
- 그룹 뷰(`display_rows`) 유닛 테스트 5종 + 그룹 선택 토글 4종 추가. 총 77 테스트(46 유닛 + 16 parser + 15 trash) 통과, 원본 SHA 불변 유지.

## [0.4.0] - 2026-06-25

### Added (M2 — 안전 삭제 / 휴지통)
- **소프트 삭제(FR-04):** `Space` 다중 선택 → `Del`/`d` 삭제 확인 모달 → `~/.claude/claudedesk/trash/`로 **파일 이동**(내용 불변). 활성 세션은 차단.
- **휴지통/복구(FR-11):** `T` 휴지통 화면 — `r` 복구(원본 경로 복귀), `D` 영구삭제. 복구 시 원본 경로 충돌은 rename 처리.
- **영구삭제 안전 게이트:** purge는 `"DELETE"` 타이핑 + Enter 2단계 확인에서만 실행. **자동/보관기간 만료 purge 없음**(안전핀 §9.3).
- 휴지통 인덱스(복구 메타) 원자적(temp+rename) 쓰기. 도움말/상태바 키힌트 갱신.

### Safety
- 원본 JSONL은 `fs::rename`(이동)만 — 내용 쓰기 0. SHA 불변 테스트 5종 포함, trash 통합 테스트 15종.

## [0.3.0] - 2026-06-25

### Added (M2 일부 — 검색·정렬)
- **검색(FR-05):** `/`로 검색 모드 진입, 제목·프로젝트(cwd) incremental 부분일치 필터(대소문자 무시), `Esc` 해제. 원본 불변(메모리 뷰 레이어).
- **정렬(FR-07):** `s` 정렬 키 순환(수정/생성/제목/메시지수), `S` 방향 토글. 헤더에 현재 정렬 표시. 기본 수정 내림차순.
- 도움말 오버레이·상태바 키힌트에 `/`·`s`·`S` 반영.

### Fixed / Hardened
- 엣지 픽스처+테스트 추가(§5.11): 이모지/다국어, 메타 64줄 초과(제목 탐색 포기 경계), `content:null` user 폴백 — 모두 원본 SHA 불변 세트 포함. (#5 FAIL-03 일부 해소)

### Changed
- 릴리스 워크플로우에서 macOS Intel(x86_64-apple-darwin) 타깃 제거 — 빌드 타깃: Windows / macOS(arm64) / Linux(musl).

## [0.2.0] - 2026-06-25

### Added (M1 MVP — 동작하는 첫 바이너리)
- **세션 스캔/목록(FR-01·02):** `~/.claude/projects/` 자동 스캔, 제목(첫 user 메시지 도출)·프로젝트(cwd)·수정시각·메시지 수를 TUI 목록으로 표시.
- **이어하기(FR-03):** 선택 세션을 해당 `cwd`에서 `claude --resume <id>`로 복귀. `claude` 부재 시 명령 안내.
- **에러 가시성(FR-12):** 손상 줄 graceful skip + 스킵 수 노출. 빈/권한오류/경로부재 안내(크래시 0).
- **도움말 오버레이(FR-14)** `?`. 키맵: `↑/k`·`↓/j` 이동, `Enter` resume, `q`/`Esc` 종료.
- **CD 릴리스 파이프라인:** 태그 `v*` → Windows/macOS(arm64+intel)/Linux 단일 바이너리 빌드 → GitHub Release 자동 첨부(`.github/workflows/release.yml`).
- **README**: 다운로드→설치→실행 안내. **LICENSE**(MIT).

### Note
- 원본 JSONL 불변(읽기 전용) — CI SHA 불변 검사로 강제. 검색·정렬·삭제는 M2.

## [0.1.0] - 2026-06-25

### Added (기획·문서 베이스라인)
- 제품 문서 패키지(`docs/`): 재설계 PRD v2.1.0, Task 분할(M0~M3, Epic 12 / Task 45), TUI UI/UX 설계, 개발 착수 가이드.
- Git 실무 워크플로우: 3-tier 브랜치 모델(`feature → develop → verify → main`), 게이트 A(CI) / 게이트 B(에이전트+벤치).
- 자율 운영 체계: 사람 리뷰어 부재 대응 안전핀(원본 SHA 불변 CI 필수, 픽스처 합성데이터+gitleaks, 영구삭제 사람 트리거, PR 루프 상한).
- CI 스캐폴딩(`.github/workflows/ci.yml`): gitleaks 상시 + Rust 게이트(Cargo.toml 가드).

### Note
- 코드 구현은 M0(기술 검증 스파이크)부터. 본 릴리스는 **기획·워크플로우 베이스라인**이다.

[Unreleased]: https://github.com/Qnd1101/claudeDesk/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/Qnd1101/claudeDesk/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/Qnd1101/claudeDesk/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/Qnd1101/claudeDesk/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Qnd1101/claudeDesk/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Qnd1101/claudeDesk/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Qnd1101/claudeDesk/releases/tag/v0.1.0
