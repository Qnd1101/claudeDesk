# claudeDesk

> Claude Code 세션을 **`claude`를 실행하지 않고** 관리·정리·삭제하는 standalone **세션 하우스키핑** 도구.
> 안 쓰는 대화를 안전하게 치우고, 프로젝트별로 묶어 보고, 필요하면 그 자리에서 이어하기까지 — 단일 바이너리 TUI.

**왜 별도 도구인가?** `claude --resume`의 내장 피커는 "이어할 세션 고르기"까지만 한다.
claudeDesk는 그 반대편 — **세션을 띄우지 않고 청소하는 일**(안전 삭제·휴지통 복구·프로젝트 그룹핑·메타 점검)을 전담한다.
수백 개로 쌓인 `~/.claude/projects/`를 `claude`를 켜지 않고 정리하는 게 핵심 가치다.

[![CI (Gate A)](https://github.com/Qnd1101/claudeDesk/actions/workflows/ci.yml/badge.svg)](https://github.com/Qnd1101/claudeDesk/actions/workflows/ci.yml)
[![Release (CD)](https://github.com/Qnd1101/claudeDesk/actions/workflows/release.yml/badge.svg)](https://github.com/Qnd1101/claudeDesk/actions/workflows/release.yml)
[![Latest Release](https://img.shields.io/github/v/release/Qnd1101/claudeDesk)](https://github.com/Qnd1101/claudeDesk/releases/latest)

```text
┌ claudeDesk ───────────────────────────────────── Local ┐
│  Sessions: 27   Skipped: 0   [?] Help                   │
├─────────────────────────────────────────────────────────┤
│   Title                     Project          Modified   │
│ ▸ [claudeDesk] PRD 재설계    D:\Dev\claudeDesk 2분 전     │
│   [BugFix] Docker socket     D:\Dev\gatelink  1시간 전    │
│   Untitled Session           C:\Users\PC      어제        │
├─────────────────────────────────────────────────────────┤
│ Enter 이어하기 · ↑↓ 이동 · ? 도움말 · q 종료              │
└─────────────────────────────────────────────────────────┘
```

## 설치 (Install)

### 1) 릴리스에서 다운로드 (권장)

[**최신 릴리스**](https://github.com/Qnd1101/claudeDesk/releases/latest)에서 OS에 맞는 파일을 받습니다.

| OS | 파일 |
| :--- | :--- |
| Windows (x64) | `claudedesk-vX.Y.Z-x86_64-pc-windows-msvc.zip` |
| macOS (Apple Silicon) | `claudedesk-vX.Y.Z-aarch64-apple-darwin.tar.gz` |
| Linux (x64, static) | `claudedesk-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz` |

**Windows**
1. `.zip`을 풀고 `claudedesk.exe`를 원하는 폴더(예: `C:\Tools\`)에 둡니다.
2. 그 폴더를 PATH에 추가하거나, 해당 폴더에서 `./claudedesk.exe` 실행.

**macOS / Linux**
```bash
tar xzf claudedesk-vX.Y.Z-<target>.tar.gz
cd claudedesk-vX.Y.Z-<target>
chmod +x claudedesk
sudo mv claudedesk /usr/local/bin/   # 또는 PATH 내 원하는 위치
claudedesk
```
> macOS에서 "확인되지 않은 개발자" 경고 시: `xattr -d com.apple.quarantine ./claudedesk` 후 실행.

### 2) 소스에서 빌드

[Rust](https://rustup.rs/) 설치 후:
```bash
git clone https://github.com/Qnd1101/claudeDesk.git
cd claudeDesk
cargo build --release
./target/release/claudedesk        # Windows: .\target\release\claudedesk.exe
```

## 핵심 기능 (Housekeeping)

`claude`를 켜지 않고 세션을 정리한다 — 이게 claudeDesk의 존재 이유다.

- 🗑️ **안전 삭제 · 휴지통(FR-04/11):** 다중 선택 → 휴지통으로 **이동만**(원본 내용 불변). 복구·영구삭제는 명시적 2단계 확인. 자동 삭제 없음.
- 🗂️ **프로젝트 그룹핑(FR-09):** 수백 개 세션을 작업 폴더(`cwd`)별로 접고 펴서 한눈에. 프로젝트 단위로 청소.
- 🔎 **검색 · 정렬(FR-05/07):** 제목·프로젝트 부분일치 필터, 수정/생성/제목/메시지수 정렬 — 정리할 대상을 빠르게 추린다.
- ↩️ **이어하기(FR-03):** 치우다 발견한 세션은 그 자리에서 `claude --resume`로 복귀(선택 기능).

> 원본 `*.jsonl`은 **절대 수정하지 않는다**(읽기 전용·이동만). CI가 SHA 불변을 강제한다.

## 사용법 (Usage)

그냥 실행하면 `~/.claude/projects/`의 세션을 자동으로 스캔해 목록을 띄웁니다.
```bash
claudedesk
```

### 단축키

| 키 | 동작 |
| :--- | :--- |
| `↑` / `k`, `↓` / `j` | 세션 이동 |
| `Enter` | 선택 세션 **이어하기**(`claude --resume`) |
| `/` | 검색 모드 (제목·프로젝트 incremental 필터) |
| `s` | 정렬 키 순환 (수정/생성/제목/메시지수) |
| `S` | 정렬 방향 토글 (↓/↑) |
| `Space` | 다중 선택 토글 (✓) |
| `Del` / `d` | 선택 세션 삭제 → 휴지통(확인) |
| `T` | 휴지통 화면 (복구 `r` / 영구삭제 `D`) |
| `?` | 도움말 오버레이 |
| `Esc` | 모달/검색/휴지통 닫기 (일반 모드에선 종료) |
| `q` | 종료 |

### 동작 방식

- 세션을 선택하고 `Enter`를 누르면 claudeDesk가 종료되며 해당 세션의 작업 폴더(`cwd`)에서 `claude --resume <id>`를 실행해 **그 대화로 곧장 복귀**합니다.
- `claude` CLI가 PATH에 없으면, 실행할 정확한 명령과 폴더를 안내하고 종료합니다.

## 요구사항 (Requirements)

- **터미널**: Windows Terminal / iTerm2·Terminal.app / 일반 Linux 터미널.
- **이어하기 기능**: [Claude Code](https://claude.com/claude-code) CLI(`claude`)가 설치되어 PATH에 있어야 합니다. (목록 보기만 할 때는 불필요.)

## 설정 (Config)

| 환경변수 | 설명 |
| :--- | :--- |
| `CLAUDEDESK_ROOT` | 세션 루트 경로 오버라이드 (기본 `~/.claude/projects`) |

> 더 많은 설정(정렬·테마·시간표기 등)은 로드맵의 M3에서 `config.toml`로 제공 예정.

## 원칙

- **Non-Destructive:** 원본 세션 파일(`*.jsonl`)은 **절대 수정하지 않음**(읽기 전용). CI가 SHA 불변을 강제.
- **Privacy First:** 외부 전송 0, 텔레메트리 없음, 100% 로컬.
- **Ultra-Low RAM:** 세션 전체를 메모리에 올리지 않고 필요한 줄만 스트리밍.

## 로드맵

- ✅ **M1:** 세션 스캔 · 목록 · 이어하기 · 에러 가시성 · 도움말
- 🚧 **M2 (현재):** 검색 · 정렬 ✅ · 안전 삭제(휴지통) ✅ · **프로젝트 그룹핑** 🚧
- ⬜ **M3:** 별칭 · 미리보기 · 설정 화면

> **포지셔닝:** claudeDesk는 `claude --resume` 내장 피커의 대체재가 아니라, 그것이 다루지 않는
> **세션 하우스키핑**(띄우지 않고 정리·삭제·그룹핑) 전담 도구다. resume는 보조 기능이다.

자세한 내용은 [docs/](docs/README.md) — PRD, Task 분할, UI/UX 설계, 개발 가이드.

## 개발 / 기여

브랜치 `feature/* → develop → verify → main`, 모든 변경은 PR + CI 통과. [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT (예정) — [LICENSE](LICENSE).
