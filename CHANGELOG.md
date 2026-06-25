# Changelog

본 프로젝트는 [Semantic Versioning](https://semver.org/lang/ko/)을 따른다.
형식은 [Keep a Changelog](https://keepachangelog.com/ko/) 기반.

## [Unreleased]

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

[Unreleased]: https://github.com/Qnd1101/claudeDesk/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/Qnd1101/claudeDesk/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Qnd1101/claudeDesk/releases/tag/v0.1.0
