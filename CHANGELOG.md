# Changelog

본 프로젝트는 [Semantic Versioning](https://semver.org/lang/ko/)을 따른다.
형식은 [Keep a Changelog](https://keepachangelog.com/ko/) 기반.

## [Unreleased]

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
- **도움말 오버레이(FR-13)** `?`. 키맵: `↑/k`·`↓/j` 이동, `Enter` resume, `q`/`Esc` 종료.
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

[Unreleased]: https://github.com/Qnd1101/claudeDesk/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/Qnd1101/claudeDesk/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/Qnd1101/claudeDesk/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Qnd1101/claudeDesk/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Qnd1101/claudeDesk/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Qnd1101/claudeDesk/releases/tag/v0.1.0
