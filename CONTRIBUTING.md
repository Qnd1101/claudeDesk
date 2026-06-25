# Contributing & 운영 규약 — claudeDesk

> **운영 모델:** 사용자는 **아이디어만** 제시하고, 기획·구현·검증·머지는 **AI 조직(CEO + 서브에이전트)**이 자율 수행한다.
> 사람 리뷰어가 없으므로 **품질 게이트를 기계/에이전트가 강제**한다. 상세 설계는 [docs/03_DEV_KICKOFF.md](docs/03_DEV_KICKOFF.md) §9.

## 브랜치 모델 (3-tier + feature)

```
feature/<task>  ──PR(게이트A)──▶  develop  ──PR(게이트A)──▶  verify  ──PR(게이트B)──▶  main
  (단위 작업)                       (개발 통합)              (검증 전용)            (릴리스/보호)
```

| 브랜치 | 역할 | 직접 push |
| :--- | :--- | :--- |
| `main` | 릴리스. 항상 배포 가능. **보호** | ❌ PR만 |
| `verify` | QA·벤치 검증 스테이징. **보호** | ❌ PR만 |
| `develop` | 개발 통합 | feature PR 권장 |
| `feature/<task-id>-<slug>` | 단위 작업 (예: `feature/T0.1-ratatui-bootstrap`) | ✅ |

* 머지: **squash merge** 고정, 머지 후 feature 삭제.
* 브랜치명은 Task ID 접두(추적성). Task ID는 [docs/01_TASK_BREAKDOWN.md](docs/01_TASK_BREAKDOWN.md).
* **모든 변경은 PR을 거친다.** 직접 커밋 금지(초기 베이스라인만 파이프라인 따라 승급).

## 게이트 (PR 통과 조건)

### 게이트 A — CI(기계, 100% 자동) · `feature→develop`, `develop→verify`
1. `cargo fmt --check`
2. `cargo clippy -- -D warnings` (경고=실패)
3. `cargo test` — 유닛 + 픽스처 회귀 + **원본 JSONL SHA 불변**
4. `gitleaks` — 시크릿/세션 본문 유출 스캔
5. 빌드 (windows + linux, 가능 시 macOS)

> Rust 코드(`Cargo.toml`) 생성 전에는 cargo 스텝 graceful no-op, `gitleaks`는 상시.

### 게이트 B — 에이전트+벤치 · `verify→main`
1. `qa-tester` 에이전트 (회귀·엣지)
2. `code-reviewer` 에이전트 (보안/대형 diff면 opus)
3. `--bench` 회귀 (RSS<20MB · 로딩≤300ms)
4. 릴리스 게이트: 버전 bump → 릴리스 노트 → squash merge

## 자율 운영 안전핀 (사람 승인 없는 머지 사고 방지)

* **파괴적 동작 격리:** 세션 **영구삭제(`purge`)·휴지통 자동정리는 사람 트리거**(자율 머지 경로 제외). 소프트삭제까지만 자율.
* **원본 불변 강제:** 원본 `*.jsonl` 읽기 전용. SHA 불변 검사를 모든 PR 필수 체크로.
* **유출 가드:** 테스트 픽스처는 **합성 데이터만**. 실제 cwd·세션 본문 커밋 금지. `.gitignore` + `gitleaks` 이중 방어.
* **PR 봇 루프 방지:** 동일 feature 재PR **상한 3회**. CI 실패 시 자동 재시도 금지 → CEO 보고.
* **보호 브랜치:** `main`·`verify` 직접 push 금지.

## 커밋 메시지

* Conventional Commits 권장: `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`, `ci:`.
* 본문에 관련 Task ID(예: `T2.2`) 명시.

## 버전 (SemVer)

* bugfix=patch / feature=minor / breaking=major. 릴리스 노트는 [CHANGELOG.md](CHANGELOG.md).
