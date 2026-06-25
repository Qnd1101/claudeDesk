# claudeDesk — 개발 착수 가이드 (Dev Kickoff)

* **연관:** [00_PRD.md](00_PRD.md) · [01_TASK_BREAKDOWN.md](01_TASK_BREAKDOWN.md) · [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md)
* **목적:** 문서↔마일스톤 매핑, 권장 순서, 첫 스프린트 진입점, Done 기준, Open Questions를 한 곳에.

---

## 1. 문서 ↔ 마일스톤 매핑

| 마일스톤 | 무엇을 보는가 | 핵심 문서 위치 |
| :--- | :--- | :--- |
| **M0 스파이크** | RAM/로딩 게이트, resume PoC | [01](01_TASK_BREAKDOWN.md) E0 · [00](00_PRD.md) §5.1·§8·부록C |
| **M1 MVP** | 스캔→파서→목록→resume→에러가시성→도움말 | [01](01_TASK_BREAKDOWN.md) E1~E4 · [00](00_PRD.md) FR-01·02·03·12·13 · [02](02_UIUX_DESIGN.md) §2.1·4·5 |
| **M2 관리** | 검색·정렬·삭제·휴지통·그룹 | [01](01_TASK_BREAKDOWN.md) E5~E8 · [02](02_UIUX_DESIGN.md) §2.2·2.4·2.7 |
| **M3 편의** | 별칭·미리보기·설정 | [01](01_TASK_BREAKDOWN.md) E9~E11 · [02](02_UIUX_DESIGN.md) §2.5·2.3·2.6 |

---

## 2. 권장 작업 순서 (charter: 레이어 우선, happy-path 먼저)

1. **M0 먼저, 무조건.** T0.1→T0.2→T0.3→T0.4(게이트). 게이트 통과 전 M1 코드 작성 금지(스택 폴백 가능성 때문). 병렬로 T0.5(resume PoC)로 부록 J Q2 결론.
2. **M1은 레이어 순차** (불확실성 높음): 데이터(E1) → 파서(E2) → 서비스(E3) → 렌더(E4) → 입력. happy-path(정상 세션 목록→resume) 먼저 → 예외(손상줄/빈/권한) → 검증(픽스처 TX.1).
3. **M2는 Epic 병렬** (E5 검색 / E6 정렬 / E7 삭제·휴지통 / E8 그룹은 상호 독립). 각 Epic은 **수직 슬라이스**로(서비스→UI 한 기능 끝까지).
4. **M3도 Epic 병렬** (E9/E10/E11 독립).
5. 각 1~2 Task 완료마다 **테스트 → 수정 → 다음**. 500-task 일괄 금지.

---

## 3. 첫 스프린트 진입점 (M0 + M1 초반)

> 스프린트 1 = **"실측 게이트 통과 + 정상 세션 목록 렌더"**까지.

| 순서 | Task | 산출 | 담당 |
| :--- | :--- | :--- | :--- |
| 1 | T0.1 ratatui 빈 앱 4타깃 부트스트랩 | 빌드되는 골격 | backend-dev(+devops-cicd 빌드) |
| 2 | T0.2 실데이터 라인 스트리밍 스캔 | 제목 추출 stdout | backend-dev |
| 3 | T0.3 `--bench` RSS/로딩 측정 | 측정 수치 | backend-dev |
| 4 | **T0.4 게이트 판정** | bench-result(임시) → **CEO 보고** | backend-dev → CEO |
| 5 | T0.5 resume 핸드오프 PoC(OS 1종) | resume 진입 성공 | backend-dev |
| 6 | TX.1 회귀 픽스처 세트 | 픽스처 디렉토리 | backend-dev/qa-tester |
| 7 | T1.1→T1.3 데이터 레이어 | 디스커버리·stat | backend-dev |
| 8 | T2.1→T2.3 파서·도메인 | `Session` 모델 | backend-dev |
| 9 | T3.1 목록 빌드 서비스 | 정렬된 목록 | backend-dev |
| 10 | T4.1 메인 리스트 렌더 | 화면 §2.1 | frontend-dev(TUI) |

* **게이트(4번) 미통과 시 중단·재결정** — 의존성 다이어트 후 재측정, 그래도 미달이면 CEO가 스택 폴백 판단.
* TUI 렌더(T4.x)는 [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md) 와이어프레임·키맵을 계약으로 사용.

---

## 4. Definition of Done (단계별)

### Task DoD
* AC 충족 + 픽스처/유닛 테스트 통과 + `cargo clippy` 무경고 + 원본 JSONL 불변(해당 시 체크섬).

### 마일스톤 DoD
* **M0:** 300세션 idle RSS<20MB(≤25 허용) & 로딩≤300ms 실측 통과 + resume 1방식 확정.
* **M1:** 실데이터 목록→Enter resume 성공, 손상줄 스킵수 노출(FR-12), `?` 도움말. qa-tester 통과.
* **M2:** 검색·정렬·다중 소프트삭제·휴지통 복구·그룹 토글 동작, 활성세션 삭제 차단 검증.
* **M3:** 별칭 제목 반영, 미리보기 패널, 설정 영속.

### 릴리스 게이트 (charter, 모든 커밋 전)
1. devops-cicd가 버전 파일 bump(SemVer: bugfix=patch/feature=minor/breaking=major).
2. tech-writer 릴리스 노트.
3. 그 후 커밋. (M1 산출부터 적용 — TX.2·TX.3.)

---

## 5. 역할 분담 요약

| 담당 | 범위 | 주요 Task |
| :--- | :--- | :--- |
| **backend-dev** | 데이터·파서·서비스·resume·로깅 | T0.2~0.5, E1~E3, E5/E6/E7 서비스, E9~E11 서비스 |
| **frontend-dev(TUI)** | ratatui 렌더·입력핸들러·화면 | E4, T5.3/T6.2/T7.3/T7.4/T8.2/T9.3/T10.2/T11.2 |
| **devops-cicd** | 빌드·4타깃 CI·릴리스·버전 | T0.1 빌드, TX.2·TX.3 |
| **qa-tester** | 픽스처 검증·회귀·수동(resume/렌더) | TX.1, 각 마일스톤 말 |

* **API/모듈 계약 핸드오프:** 서비스 레이어가 노출하는 `Session` 모델·목록빌드/검색/삭제/resume 함수 시그니처가 backend↔frontend 계약. M1 시작 시 CEO가 §6 계약 초안 확정 후 양측 배포.

---

## 6. 모듈 계약 초안 (backend ↔ TUI)

> 정식 API가 아닌 in-process 모듈 경계. M1 착수 전 시그니처 고정 권장.

```rust
// 도메인
struct Session {
    id: String,            // sessionId(UUID)
    title: String,         // 도출/별칭 (부록 B)
    cwd: String,
    git_branch: Option<String>,
    created: Option<DateTime>,
    modified: DateTime,    // mtime
    msg_count: Option<u32>,// lazy
    is_active: bool,       // mtime 근접 휴리스틱
    alias: Option<String>,
}

// 서비스 경계(예시 시그니처)
fn scan_sessions(root: &Path, cfg: &Config) -> ScanResult; // ScanResult{ sessions, skipped_lines, skipped_files, errors }
fn sort_sessions(&mut Vec<Session>, key: SortKey, dir: SortDir);
fn filter_sessions<'a>(&'a [Session], query: &str) -> Vec<&'a Session>;
fn preview_snippet(id: &str, cwd: &Path) -> Result<String>;   // 스트리밍
fn resume(session: &Session, mode: ResumeMode) -> ResumeOutcome; // handoff|spawn, 실패 시 명령 반환
fn soft_delete(ids: &[String]) -> DeleteResult;  // 활성 차단
fn restore(ids: &[String]) -> Result<()>;
fn purge(ids: &[String]) -> Result<()>;
fn set_alias(id: &str, alias: Option<&str>) -> Result<()>; // 사이드카 원자적
```

---

## 7. Open Questions (CEO 결정 필요 — [00_PRD.md](00_PRD.md) 부록 J 취합)

| # | 질문 | 기본안 | 차단 시점 |
| :--- | :--- | :--- | :--- |
| Q1 | 사이드카 SQLite 전환 임계 | 단일 JSON 유지, 전환 보류 | M3 |
| **Q2** | **Windows resume 핸드오프 방식**(in-place exec 불가) | M0 PoC로 결정(spawn vs 래퍼 vs 명령출력) | **M1 차단** |
| Q3 | Windows conhost 지원 범위 | Windows Terminal 권장, conhost best-effort | M1 |
| Q4 | 활성세션 판정 | mtime 근접 90s 휴리스틱(설정화) — 해소, 값 튜닝만 | — |
| Q5 | 휴지통 보관기간 | 30일(설정화) — 해소 | — |
| Q6 | 메시지 수 즉시카운트 vs lazy | 기본 lazy, M0 벤치로 재평가 | M0 입력 |

* **즉시 결정 권장:** Q2(M1 진행 차단), Q3(테스트 범위). 나머지는 기본안으로 진행 가능.

---

## 8. 첫 커밋까지의 흐름(요약)

```
[research(완료: 본 docs)] → architect(완료) → CEO 승인 →
  M0(T0.1~0.5) → 게이트 보고 → (통과) →
  M1 레이어 구현(backend → 계약 → frontend) →
  code-reviewer → qa-tester →
  [릴리스 게이트: 버전 bump → 릴리스노트] → 첫 커밋
```

---

## 9. Git 워크플로우 & 자율 운영 체계 (사람 리뷰어 부재 대응)

> **전제:** 사용자는 **아이디어만** 제시한다. 기획·구현·검증·머지는 **AI 조직(CEO + 서브에이전트)**이 전부 자율 수행한다. 사람 리뷰어가 없으므로 **품질 게이트를 기계/에이전트가 강제**해 회귀·결함·사고를 막는다.

### 9.1 브랜치 모델 (3-tier + feature)

```
feature/<task>  ──PR(게이트A)──▶  develop  ──PR(게이트A)──▶  verify  ──PR(게이트B)──▶  main
  (단위 작업)                       (개발 통합)              (검증 전용)            (릴리스/보호)
```

| 브랜치 | 역할 | 보호 | 직접 push |
| :--- | :--- | :--- | :--- |
| `main` | 릴리스. 항상 배포 가능 상태 | ✅ branch protection | ❌ (PR만) |
| `verify` | QA·벤치 검증 스테이징 | ✅ | ❌ (PR만) |
| `develop` | 개발 통합 | (선택) | feature PR 권장 |
| `feature/<task-id>-<slug>` | 단위 작업(예: `feature/T2.2-title-derive`) | — | ✅ (작업자) |

* 머지 방식: **squash merge** 고정(히스토리 단순화). 머지 후 feature 브랜치 삭제.
* 브랜치명 규칙: Task ID를 접두로(추적성). 예 `feature/T0.1-ratatui-bootstrap`.

### 9.2 자동 게이트 (PR 통과 조건)

**게이트 A — CI(기계, 100% 자동).** `feature→develop`, `develop→verify` PR의 required check:
1. `cargo fmt --check`
2. `cargo clippy -- -D warnings` (경고=실패. charter "no any/unknown" 상응)
3. `cargo test` — 유닛 + **픽스처 회귀** + **원본 JSONL SHA 불변 검사**(§아래 9.4)
4. `gitleaks` — 시크릿/세션 본문 유출 스캔
5. 빌드(최소 windows + linux, 가능 시 macOS 포함 4타깃)

> Rust 코드 생성(T0.1) 전에는 cargo 스텝을 `hashFiles('Cargo.toml')` 가드로 graceful no-op 처리, `gitleaks`는 상시 수행.

**게이트 B — 에이전트+벤치(verify→main).**
1. `qa-tester` 에이전트: 회귀·엣지케이스(코드 수정 없음)
2. `code-reviewer` 에이전트: 결함·보안·컨벤션(보안/대형 diff면 opus 승격)
3. `--bench` 회귀: RSS<20MB & 로딩≤300ms 재측정(M0 게이트 회귀 가드)
4. **릴리스 게이트**(§4): 버전 bump → 릴리스 노트 → squash merge

### 9.3 자율 운영 안전핀 (사람 승인 없는 머지의 사고 방지)

* **파괴적 동작 격리:** 세션 **영구삭제(`purge`)·휴지통 자동정리**는 자율 머지 경로에서 제외 → **사람 트리거 필요**로 표시(M2 한정). 소프트삭제(휴지통 이동)까지만 자율.
* **원본 불변 강제:** 원본 `*.jsonl`은 읽기 전용. SHA 불변 검사(9.4)를 **모든 PR 필수 체크**로. 데이터 유실은 복구 불가 1순위 사고.
* **비밀·본문 유출 가드:** 테스트 픽스처는 **합성 데이터만**(실제 cwd·세션 본문 금지). `.gitignore` + `gitleaks` CI로 이중 방어.
* **PR 봇 루프 방지:** 동일 feature 브랜치 **재PR 횟수 상한**(기본 3). CI 실패 시 **자동 재시도 금지** → 에이전트 중단 후 **CEO 보고**.
* **보호 브랜치:** `main`·`verify` 직접 push 금지(branch protection). 위험 동작 변경 포함 PR은 CEO 수동 핀.

### 9.4 원본 JSONL SHA 불변 검사 (CI 필수)

* 픽스처 디렉토리의 원본 `*.jsonl` 각 파일 SHA-256을 골든값으로 고정.
* 테스트가 모든 작업(스캔/목록/검색/삭제 시뮬레이션) 전후로 원본 SHA를 대조 → **불일치 시 CI 실패**.
* `00_PRD.md` §8·§5.11과 연동(수동 가정 → CI 강제로 승격).

### 9.5 staff 흐름 (CEO 오케스트레이션)

```
[idea] → CEO triage → architect(설계) → backend-dev/frontend-dev(구현, feature 브랜치)
  → PR→develop (게이트A) → code-reviewer → PR→verify (게이트B: qa-tester+bench)
  → PR→main (릴리스 게이트) → squash merge → main
```
* staff는 서로 호출 불가 → CEO가 핸드오프 중계. 독립 작업은 병렬.
* 모든 커밋은 PR을 거친다(charter + 사용자 지시). 초기 베이스라인만 예외적으로 파이프라인 따라 승급.
