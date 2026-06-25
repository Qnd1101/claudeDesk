# claudeDesk — Task 분할서 (Task Breakdown)

* **연관:** [00_PRD.md](00_PRD.md) (FR ID 출처) · [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md) (화면/키맵) · [03_DEV_KICKOFF.md](03_DEV_KICKOFF.md) (작업 순서)
* **분할 철학(charter 준수):**
  * 복잡/불확실 → **레이어 순서**: 데이터/스토리지 → 파서/도메인 → 서비스 → TUI/렌더 → 입력핸들러.
  * 단순 CRUD → **수직 슬라이스**(한 기능 happy-path 끝까지).
  * **happy path → 예외 → 검증** 순.
* **규모:** S(≤반나절) / M(1~2일) / L(3일+, 추가 분해 후보).
* **마일스톤:** M0 스파이크 → M1 MVP → M2 관리 → M3 편의. **M0이 1번.**

---

## 마일스톤 요약

| 마일스톤 | 목표 | Epic 수 | Task 수 | 관련 FR |
| :--- | :--- | :--- | :--- | :--- |
| **M0** 스파이크 | RAM<20MB·로딩 실측 게이트 통과 | 1 | 5 | (게이트) §5.1 |
| **M1** MVP | 평면 리스트 → resume(핸드오프) | 4 | 16 | FR-01·02·03·12·13 |
| **M2** 관리 | 검색·정렬·소프트삭제·휴지통·그룹핑 | 4 | 15 | FR-04·05·07·09·11 |
| **M3** 편의 | 별칭·미리보기·설정 | 3 | 9 | FR-06·08·10 |
| **합계** | | **12** | **45** | FR-01~13 |

> 의존성 표기: `→ Txx` = 선행 Task. layer 표기로 레이어 위치 명시.

---

## M0 — 기술 검증 스파이크 (게이트)

### Epic E0: RAM/성능 실측 PoC
> **이 Epic 통과 = 스택 확정.** 미달 시 의존성 다이어트 → 재측정 → 폴백 의사결정.

| ID | 설명 | layer | 의존 | 완료조건(AC) | 규모 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **T0.1** | Cargo 프로젝트 + ratatui 빈 앱(빈 화면, q 종료) 부트스트랩, 4타깃 빌드 확인 | infra | — | `cargo run`으로 빈 TUI 진입/종료. Win/Linux 최소 2타깃 빌드 성공 | S |
| **T0.2** | 실제 `~/.claude/projects/`를 `BufReader` 라인 스트리밍으로 스캔, 파일당 첫 user 줄까지 K줄 스캔(부록 B) | parser | T0.1 | 300+ 세션 스캔 완료, 제목 후보 추출 stdout 출력. 크래시 0 | M |
| **T0.3** | `--bench` 모드: 콜드/웜 로딩시간 + 측정 시점 RSS를 출력 | service | T0.2 | 로딩 ms·RSS(MB) 수치 출력. OS별 RSS 취득(WorkingSet/VmRSS/ps rss) | M |
| **T0.4** | **게이트 실측·기록:** 300 세션 idle RSS, 로딩시간 측정 → 결과를 `bench-result.md`(임시)에 기록 | service | T0.3 | RSS<20MB(≤25 허용) & 로딩≤300ms 판정. 미달 시 원인·다이어트 안 기록 | S |
| **T0.5** | resume 핸드오프 스파이크: 선택 sessionId/cwd로 `claude --resume` exec/spawn **Windows 1종 필수 PoC**(가장 불확실한 플랫폼 선검증) | service | T0.2 | **Windows**에서 실제 세션 진입 성공 + 방식 결론 1줄(spawn / 래퍼스크립트 / 명령출력-후-종료 택1). 부록J Q2 해소 | M |

**M0 Done:** T0.4 게이트 통과 + T0.5로 resume 방식 1개 확정. (실패 시 CEO 보고 → 스택/방식 재결정.)

---

## M1 — MVP (FR-01·02·03·12·13)

### Epic E1: 데이터/스토리지 레이어
| ID | 설명 | layer | 의존 | AC | 규모 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **T1.1** | 경로 해석: `projects_root` 결정(`directories` 크레이트, `~` 확장), 존재/권한 점검 | data | T0.* | 기본 경로 해석 + 부재 시 에러 객체 반환 | S |
| **T1.2** | 세션 파일 디스커버리: 프로젝트 폴더 순회, `*.jsonl` 수집, `subagents/` 제외 | data | T1.1 | 세션 경로 리스트 반환. 빈 폴더/권한오류 graceful | S |
| **T1.3** | `stat` 메타 취득: mtime/ctime/size 일괄(파싱 없이) | data | T1.2 | 파일당 메타 구조체. 누락 시 폴백값 | S |

### Epic E2: 파서/도메인 레이어
| ID | 설명 | layer | 의존 | AC | 규모 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **T2.1** | JSONL 라인 스트리밍 파서(serde_json), 모르는 필드 무시·깨진 줄 skip+카운트 | parser | T1.3 | 픽스처(정상/손상/빈)에서 skip 카운트 정확, 크래시 0 | M |
| **T2.2** | 제목 도출(부록 B): 첫 user 줄 K줄 스캔, `content` 문자열/블록배열 처리, Untitled 폴백 | parser | T2.1 | 메타선행 픽스처에서 7번째 줄 user 제목 추출. 블록배열 text 추출 | M |
| **T2.3** | `Session` 도메인 모델 조립(title, cwd, created, modified, msgCount-lazy, active flag) | parser | T2.2,T1.3 | 1세션→1 Session 구조체. unicode-width 폭 안전 | S |
| **T2.4** | 활성 세션 판정: mtime 근접(active_window_secs) 휴리스틱 | parser | T2.3 | 임계 내 세션 active=true. 설정값 반영 | S |

### Epic E3: 서비스 레이어
| ID | 설명 | layer | 의존 | AC | 규모 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **T3.1** | 세션 목록 빌드 서비스: 디스커버리→파서→정렬(기본 modified_desc) 파이프라인 | service | T2.* | 정렬된 `Vec<Session>` 반환, 300세션 ≤300ms | M |
| **T3.2** | resume 서비스: sessionId/cwd로 `claude --resume` 핸드오프(M0 확정 방식), `claude` 부재 폴백 | service | T0.5,T3.1 | 선택 세션 실제 진입. 부재 시 명령 출력 폴백 | M |
| **T3.3** | 로깅 인프라(§5.4): 파일 로그 + 레벨 + 스킵카운트 집계 | service | T2.1 | 로그 파일 생성·회전, 스킵 수 집계 노출 | S |
| **T3.4** | 에러/스킵 집계 모델(FR-12)을 서비스에서 상태로 노출 | service | T3.1,T3.3 | 스킵 파일/줄 수, 권한오류가 상태 객체에 | S |

### Epic E4: TUI/렌더 + 입력 (MVP 화면)
| ID | 설명 | layer | 의존 | AC | 규모 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **T4.1** | 메인 리스트 화면: ratatui `Table`+`Block`, 컬럼(마커/제목/프로젝트/수정시각/메시지수), 상태바 | render | T3.1 | 평면 리스트 렌더, 컬럼 정렬 표시. [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md) §2.1 | M |
| **T4.2** | 상태바·헤더: RAM·모드·스킵카운트(FR-12)·키힌트 표시. 컬러무관 마커(§5.7) | render | T4.1,T3.4 | 상태바에 스킵수·키힌트, mono 테마 식별 가능 | S |
| **T4.3** | 입력핸들러: ↑↓/jk 이동, Enter resume, q 종료, ? 도움말. crossterm 이벤트 루프 | input | T4.1,T3.2 | 키 동작 매핑. resume Enter 동작 | M |
| **T4.4** | 도움말 오버레이(FR-13, `?`): `Paragraph`+`Block` 모달, 키맵 표시 | render | T4.3 | `?`로 오버레이 토글, Esc 닫기 | S |
| **T4.5** | 빈 상태/에러/로딩 표시(빈 목록·경로없음·권한오류) | render | T4.1,T3.4 | 각 상태 전용 메시지. [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md) §4 | S |
| **T4.6** | 좁은 터미널 반응형: 컬럼 우선순위 축약(§5.7, [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md) §6) | render | T4.1 | 폭<80에서 프로젝트/시각 컬럼 축약, 크래시 0 | M |

**M1 Done:** 실데이터로 목록 표시 → Enter로 resume 성공 → 손상줄 스킵수 노출 → `?` 도움말. qa-tester 검증 + 원본 JSONL 불변(체크섬).

---

## M2 — 관리 기능 (FR-04·05·07·09·11)

### Epic E5: 검색/필터 (FR-05)
| ID | 설명 | layer | 의존 | AC | 규모 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **T5.1** | 검색 인덱스 필드 구성(제목·첫user텍스트·cwd) 도메인에 보강 | parser | T2.3 | Session에 검색대상 텍스트 보유 | S |
| **T5.2** | incremental 필터 서비스(부분일치, 대소문자 무시) | service | T5.1,T3.1 | 키워드로 목록 필터, 300세션 즉응 | S |
| **T5.3** | 검색 UI: `/` 진입, 입력바(`Paragraph`), 실시간 필터, Esc 취소 | render+input | T5.2,T4.3 | `/`→타이핑→리스트 좁힘. [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md) §2.2 | M |

### Epic E6: 정렬 (FR-07)
| ID | 설명 | layer | 의존 | AC | 규모 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **T6.1** | 정렬 키/방향 모델 + 서비스(modified/created/title/messages × asc/desc) | service | T3.1 | 4키 토글, msgCount 정렬 시 lazy 카운트 트리거 | S |
| **T6.2** | 정렬 UI: `s` 순환 토글 + 정렬 상태 헤더 표시 | render+input | T6.1,T4.3 | `s`로 키 순환, 현재 정렬 표시 | S |

### Epic E7: 삭제/휴지통 (FR-04·11)
| ID | 설명 | layer | 의존 | AC | 규모 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **T7.1** | 휴지통 스토리지: `~/.claude/claudedesk/trash/`로 원자적 이동 + 메타(삭제시각) 기록 | data | T1.1 | 파일 이동(복사 아님)+복구정보 저장 | S |
| **T7.2** | 삭제 서비스: 활성세션 차단(T2.4), 다중 선택 **소프트삭제**(자율). **영구삭제(purge)·보관기간 자동정리는 사람 트리거**(자율 머지 경로 제외, 03_DEV_KICKOFF §9.3 안전핀) | service | T7.1,T2.4 | 활성 차단, 소프트삭제 동작. purge는 명시적 사용자 확인 게이트. 원본 손상 0 | M |
| **T7.3** | 다중선택 UI(Space 토글) + 삭제 확인 모달(2단계: 소프트→영구) | render+input | T7.2,T4.3 | Space 선택, Del→확인→이동. [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md) §2.4 | M |
| **T7.4** | 휴지통 화면(FR-11): 삭제목록 조회·복구·영구삭제 | render+input | T7.2 | 휴지통 진입·복구·영구삭제 동작. [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md) §2.7 | M |

### Epic E8: 프로젝트 그룹핑 (FR-09)
| ID | 설명 | layer | 의존 | AC | 규모 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **T8.1** | cwd 단위 그룹 모델(접힘 상태, 그룹↔세션 인덱스 매핑) | service | T3.1 | 그룹 트리 데이터 구조, 평면↔그룹 토글 | M |
| **T8.2** | 그룹 렌더(섹션 헤더 + 접기/펼치기) + 토글(`g`/Tab) | render+input | T8.1,T4.1 | 그룹 표시·접힘. [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md) §2.1 변형 | M |

**M2 Done:** 검색·정렬·다중삭제(소프트)·휴지통 복구·그룹 토글 동작. 활성세션 삭제 차단 검증. qa-tester.

---

## M3 — 편의 기능 (FR-06·08·10)

### Epic E9: 별칭 (FR-06)
| ID | 설명 | layer | 의존 | AC | 규모 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **T9.1** | 사이드카(`meta.json`) read/write 원자적(temp+rename), sessionId 키 | data | T1.1 | 별칭 저장/로드, 원본 불변. 고아키 graceful | S |
| **T9.2** | 제목 도출 1순위에 별칭 반영(부록 B) | parser | T9.1,T2.2 | 별칭 있으면 제목=별칭 | S |
| **T9.3** | 별칭 편집 UI(`R`): 입력 모달, 저장/취소 | render+input | T9.1,T4.3 | `R`→입력→저장 반영. [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md) §2.5 | S |

### Epic E10: 미리보기 (FR-08)
| ID | 설명 | layer | 의존 | AC | 규모 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **T10.1** | 미리보기 서비스: 선택 세션 첫 user 텍스트 스니펫 스트리밍(전체 로드 금지) | service | T2.2 | 스니펫 반환, 대용량도 일정 메모리 | S |
| **T10.2** | 미리보기 패널 UI(분할 레이아웃, `Paragraph`) | render | T10.1,T4.1 | 선택 시 우측 패널 표시. [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md) §2.3 | M |

### Epic E11: 설정 (FR-10)
| ID | 설명 | layer | 의존 | AC | 규모 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **T11.1** | `config.toml` 로드/기본생성 + CLI 인자 오버라이드(§5.10) | service | T1.1 | 설정 파일 read, CLI 우선. 누락 시 기본값 | S |
| **T11.2** | 설정 화면 UI(경로·시간표기·resume모드·보관일·테마) read/write | render+input | T11.1 | 인앱 변경→파일 반영. [02_UIUX_DESIGN.md](02_UIUX_DESIGN.md) §2.6 | M |
| **T11.3** | 테마/컬러무관/`NO_COLOR`(§5.7) 렌더 반영 | render | T11.1,T4.2 | mono/auto/dark/light, NO_COLOR 존중 | S |

**M3 Done:** 별칭 지정 후 제목 반영, 미리보기 패널, 설정 변경 영속. qa-tester + 전체 회귀.

---

## 횡단 Task (마일스톤 무관, 지속)

| ID | 설명 | 시점 |
| :--- | :--- | :--- |
| **TX.1** | 회귀 픽스처 세트 구축(§5.11): 정상/메타선행/손상/빈/블록배열/surrogate. **합성 데이터만**(실제 cwd·세션 본문 금지) | M0~M1 |
| **TX.2** | **게이트 A CI**(03_DEV_KICKOFF §9.2): push 시 `cargo fmt`·`clippy -D`·`test`(픽스처+원본SHA불변)·`gitleaks`·빌드. 태그 시 4타깃 릴리스 빌드(devops-cicd) | M0 말(골격)~M1 |
| **TX.3** | 버전 파일 단일화 + SemVer bump 규칙(릴리스 게이트) | M1부터 |
| **TX.4** | **유출·사고 가드:** `.gitignore`(`*.jsonl`은 `tests/fixtures/**` 예외, `bench-result.md`/`research.md`/`plan.md`/`target/`) + `gitleaks` CI 단계 + 원본 JSONL **SHA 불변 테스트**(03_DEV_KICKOFF §9.4) | M0~M1, 상시 |

---

## 의존성 핵심 경로 (Critical Path)

```
T0.1→T0.2→T0.3→T0.4(게이트) ─┐
T0.2→T0.5(resume PoC) ───────┤
                             ▼
T1.1→T1.2→T1.3→T2.1→T2.2→T2.3→T3.1→T4.1→T4.3 (MVP 코어)
                                     T3.2(resume)┘
```
M1 코어는 T0 게이트 통과 후 레이어 순차. M2/M3 Epic은 대체로 병렬 가능(E5/E6/E7/E8 독립, E9/E10/E11 독립).
