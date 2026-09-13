# 정리 대상 판정 기준과 명령 시퀀스

근거는 2026-08-09 세션. 디스크 여유 213GB → 773GB (560GB 확보), 이 중 551GB가
`skeleton-engine/target` 하나였다.

## 멈출 때

**여유가 20% 이상이고 회수 가능량이 1GB 미만이면 아무것도 지우지 않고 측정 결과만 보고한다.**

정리는 공짜가 아니다. `cargo clean -p`로 지운 만큼 다음 빌드가 다시 컴파일한다. 여유가
넉넉한데 몇백 MB를 얻자고 갓 빌드한 산출물을 날리면 순손해다.

2026-08-09 첫 테스트 실행이 이 경우였다: 30분 전 정리 직후라 회수 가능량이
skeleton-engine 664MB(그나마 직전 `cargo check`가 만든 것) + wasm 377KB뿐이었고,
다른 두 프로젝트는 `Summary 0 files`였다. 여유 773GB(83%) 상태에서 정리하지 않는 것이
정답이었다. "정리했음"이 아니라 **"정리할 것이 없음"도 유효한 결론**이다.

## 경계 — 어기지 않는다

### 지운다 (재생성되는 개발 산출물)

| 대상 | 방법 | 비고 |
|---|---|---|
| 각 프로젝트 `target/` | `cargo clean -p <pkg>` | 타깃 트리플마다 한 번씩 |
| 미사용 rustup 툴체인 | `rustup toolchain uninstall <v>` | MSRV 확인 후 |
| npm 캐시 | `npm cache clean --force` | |
| Homebrew | `brew cleanup -s --prune=all` | |
| pip | `pip cache purge` | |
| `~/.cargo/registry/src` | `rm -rf` | `.crate`에서 재추출, **영구 절감 아님** |
| 참조 끊긴 `~/.cargo/git/{db,checkouts}/<name>` | `rm -rf` | 아래 확인 절차 필수 |
| `~/Library/Caches/node-gyp`, `org.swift.swiftpm`, `swift-frontend` | `rm -rf` | |
| **저장소 안에 남은 ignore된 빌드 산출물** | `rm -rf` | 트리를 지워도 산출물은 남는다. 아래 확인 절차 필수 |

### 저장소 안 잔재 — `git status`가 절대 알려주지 않는다

`target/`만 보면 놓친다. **git에서 트리를 지워도 그 트리의 ignore된 빌드 산출물은 디스크에
남고, ignore되어 있으니 `git status`는 영원히 조용하다.** 2026-09-04에 `examples/` 밑에서
**181MB**가 나왔다 — 2026-08-19에 삭제된 예제 13개의 `web/pkg/*.wasm`과, `build_wasm.sh`가
더 이상 쓰지 않는 경로에 3개월 묵은 `examples/wasm/pkg`. 소스가 없으니 **다시 만들 수도 없는**
순수 사석이었고, 석 달간 아무 명령에도 안 걸렸다.

찾는 법 — 디렉터리마다 추적 파일 수와 실제 파일 수를 비교한다:

```sh
for d in examples/*/; do echo "$(git ls-files "$d" | wc -l) $(find "$d" -type f | wc -l) $d"; done
```

**tracked=0인데 files>0이면 잔재다.** 지우기 전에 그 디렉터리에 추적 파일·소스
(`*.rs`/`*.html`/`*.ron`/`*.toml`)·`pkg/` 밖 파일이 **전부 0**인지 확인한다 — 하나라도 있으면
잔재가 아니라 커밋되지 않은 작업일 수 있다. 트리를 지운 직후에 한 번 돌리면 애초에 안 쌓인다.

### 안 지운다

- **사용자 앱 데이터** — `Google`(Chrome), `Firefox`, `net.imput.helium`,
  `com.spotify.client`, `Steam`. 크기와 무관하게 기본 제외. 지우려면 **매번 따로 승인**받고,
  해당 앱이 종료돼 있는지 `pgrep`으로 확인한다.
- **실행 중인 도구의 캐시** — `Codex`, `com.openai.codex`, `claude-cli-nodejs` 등.
  codex와 claude-cli는 보통 실행 중이다.
- **의존성** — `node_modules`는 캐시가 아니라 의존성이다. `ms-playwright`도 쓰는
  프로젝트가 있으면 남긴다 (`grep -rln playwright ~/Projects/*/package.json`).
- **`deps/`** — `cargo clean -p`가 알아서 남긴다. 이게 다음 빌드를 빠르게 하는 부분이다.
  전체 `cargo clean`은 이걸 날려서 wgpu·winit 트리를 처음부터 다시 컴파일하게 만든다.
- **`CoreSimulator`** — `xcrun simctl list devices | grep -c unavailable`이 0이면 전부 유효.
- 확신이 안 서면 지우지 말고 보고에 후보로 올린다.

## 순서

```sh
# 1. 측정
df -h ~ | tail -1
du -sh */ .[a-z]*/ 2>/dev/null | sort -hr | head -20

# 2. Rust 프로젝트 — 규모 확인 후 타깃별로
cargo clean -p <pkg> --dry-run
cargo clean -p <pkg>
cargo clean -p <pkg> --target wasm32-unknown-unknown
```

워크스페이스면 멤버 이름을 먼저 얻는다:

```sh
cargo metadata --no-deps --format-version 1 \
  | python3 -c "import sys,json; print(' '.join(p['name'] for p in json.load(sys.stdin)['packages']))"
```

### 툴체인

핀이 없고 MSRV로 빌드 불가한 것만 지운다:

```sh
grep -h 'rust-version' ~/Projects/*/Cargo.toml    # MSRV
find ~/Projects -maxdepth 3 -name 'rust-toolchain*'   # 핀 (없어야 안전)
rustup toolchain list
```

2026-08-09에는 세 프로젝트 모두 `rust-version = "1.95"`, 핀 없음이어서 1.79.0과 1.88.0이
애초에 빌드 불가한 잔재였다 (2.4GB).

### `~/.cargo/git` 판정

git 의존성이 살아 있으면 남긴다. 재클론 비용이 절감분보다 크다.

```sh
grep -rn 'git = ' ~/Projects/*/Cargo.toml
grep -rn '<name>' ~/Projects/*/Cargo.toml ~/Projects/*/Cargo.lock   # 끊긴 항목만 삭제
```

### Library 캐시

**글로브로 훑지 말 것** (아래 함정 참고). 이름을 적어 개별 측정한다:

```sh
cd ~/Library/Caches && du -sh Google Homebrew ms-playwright pip Steam Firefox \
  com.spotify.client net.imput.helium Codex claude-cli-nodejs node-gyp \
  org.swift.swiftpm swift-frontend 2>/dev/null | sort -hr
```

## 함정

- **`cargo clean -p`는 호스트 타깃만 지운다.** 1차 실행 후에도 wasm 산출물이 통째로 남는다
  (그때 187GB). `--target wasm32-unknown-unknown`으로 한 번 더 돌려야 한다.
- **인자 조합을 셸 변수에 담아 루프 돌리지 말 것.** 이 셸은 zsh이고, zsh은 따옴표 없는
  변수를 단어 분리하지 **않는다**. `for a in "" "--target wasm32-unknown-unknown"; do cargo
  clean -p pkg $a; done`은 `--target wasm32-unknown-unknown`을 **한 단어**로 넘겨 cargo가
  `For more information, try '--help'.` 한 줄만 뱉고 끝난다 — 실패인데 실패처럼 안 보인다.
  2026-08-19에 이걸로 wasm 13GB가 그대로 남았고, `du` 전후 비교가 아니었으면 "정리 완료"로
  오보고할 뻔했다. **타깃별로 줄을 따로 쓴다.** (bash였다면 동작했을 코드라서 더 위험하다.)
- **cargo의 "Removed N files, X GiB"는 하드링크를 중복 계산**해 실제보다 크게 나온다.
  네이티브는 440.6GiB라고 했지만 `du` 기준 실제 감소는 그보다 작았다. 확보량은
  `du -sh target/` **전후 비교로만** 보고한다.
- **`du -sh ~/Library/Caches/*`는 멈춘다** — Apple 시스템 캐시(CloudKit 계열)에서 걸려
  두 번 다 완주하지 못했다. 개별 측정으로 우회한다.
- **이 머신에 `timeout`/`gtimeout`이 없다.** 쓰면 빈 출력이 나오는데, 이걸 타임아웃으로
  착각해 멀쩡한 캐시를 전부 "측정 실패"로 오판한 적이 있다. `command -v`로 먼저 확인한다.
- **백그라운드 완료 알림은 마지막 명령의 상태**다. `echo $? > /tmp/x.exit` 후 파일을 읽되,
  먼저 `rm -f`한다 — 오래된 파일이 즉시 매치된다.
- `du`를 중복으로 띄우면 서로 I/O를 잡아먹는다. 멈춘 스캔은 `pkill -f 'du -sh'`로 정리한다.

## 검증과 보고

정리 전에 `git status --short`와 기존 변경 내용을 기록한다. 정리 후에는 소스·문서 및
기존 미커밋 파일의 내용이 보존되었는지 비교한다. 상태 목록이 같다는 것만으로 내용
보존이 증명되지는 않는다. 깨끗한 작업 트리를 만들려고 기존 변경을 되돌리지 않는다.

```sh
cargo check          # exit 0 — deps가 보존됐으면 몇 초 안에 끝난다
git status --short   # 정리 전 상태와 비교한다. 기존 미커밋 변경은 유지한다
df -h ~ | tail -1
```

정리 직후 `cargo check --all-targets`가 7.15초에 끝난 것이 deps 보존의 증거였다.
전체 `cargo clean`이었다면 몇 분이 걸렸을 것이다.

보고에는 **지운 것 / 남긴 것과 그 이유 / 미승인 후보**를 각각 표로 넣는다.
건너뛴 검증이 있으면 명시한다.
