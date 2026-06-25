# claudeDesk

> Claude Code의 로컬 세션(채팅방)을 **초경량 RAM**으로 관리하는 크로스플랫폼 **TUI** 툴.
> 세션을 한눈에 보고, 골라서 **바로 이어하기(resume)**. 무거운 GUI 없이 단일 바이너리로.

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

- ✅ **M1 (현재):** 세션 스캔 · 목록 · 이어하기 · 에러 가시성 · 도움말
- ⬜ **M2:** 검색 · 정렬 · 안전 삭제(휴지통) · 프로젝트 그룹핑
- ⬜ **M3:** 별칭 · 미리보기 · 설정 화면

자세한 내용은 [docs/](docs/README.md) — PRD, Task 분할, UI/UX 설계, 개발 가이드.

## 개발 / 기여

브랜치 `feature/* → develop → verify → main`, 모든 변경은 PR + CI 통과. [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT (예정) — [LICENSE](LICENSE).
