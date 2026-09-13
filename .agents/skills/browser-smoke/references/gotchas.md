# browser-smoke — 함정과 근거

근거는 2026-08-21 재건(#494 · #496 · #501). 살아 있는 전례 셋:

| 스모크 | TAG | PORT/DBG |
|---|---|---|
| `scripts/survivor_audio_web_smoke.sh` | `AUDIO_CHECK` | 8090 / 9310 |
| `scripts/netplay_web_smoke.sh` | `NETPLAY_CHECK` | 8091 / 9311 |
| `scripts/wasm_failpaths_web_smoke.sh` | `FAILPATH_CHECK` | 8092 / 9312 |

셋 다 `wasm-smokes` CI 잡에서 매 PR 게이트된다. 나머지 브라우저 확인은 바이트 크기만
보므로 로컬 전용이다 — 어느 쪽인지는 `grep smoke .github/workflows/ci.yml`로 센다.

## 1. `--virtual-time-budget` 금지

AudioContext 잠금 해제, WebSocket 핸드셰이크, 어댑터 해석은 전부 **벽시계**다. 가상
시간을 빨리 감으면 멀쩡한 장치에서 침묵을 읽는다. `CLAUDE.md`가 `ENGINE_CAPTURE`에
대해 기록한 것과 같은 함정이 한 겹 밖으로 나온 것이다.

## 2. 필수 플래그와 각각의 이유

- `--headless=new` — 옛 headless shell이 아닌 진짜 Chrome.
- `--enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader` — 러너에 GPU가
  없다. 소프트웨어 렌더러로 어댑터를 얻는다.
- `--autoplay-policy=no-user-gesture-required` — 없으면 AudioContext가 suspended로
  남아 **멀쩡한 엔진이 rms 0.0을 낸다.** 사보타주로 확인함.
- `--mute-audio`는 `SKELETON_MUTE=1`일 때만. 레벨 탭이 pre-volume이라 측정값은 같다.

## 3. 포트는 스모크마다 고정한다

스크립트보다 오래 산 Chrome이 DevTools 포트를 잡고 있으면 다음 스모크가 **엉뚱한 탭의
title**을 읽는다. HTTP 포트도 같은 이유로 `lsof`로 선점을 검사하고 exit 2로 죽는다 —
낡은 `http.server`는 다른 페이지를 서빙하고, 그건 "판정 없음"으로 나타나 엔진 고장처럼
읽힌다.

⚠️ **헤더의 usage 예시와 기본값이 어긋날 수 있다.** `survivor_audio_web_smoke.sh`는
헤더가 `SMOKE_DBG=9301`인데 기본값은 `9310`이다. 위 표는 기본값 쪽이다.

## 4. title 시작값에 TAG를 넣지 않는다

폴링이 즉시 매칭해 **아직 나지 않은 판정**을 읽는다.

## 5. detail에 큰따옴표를 넣지 않는다

스크립트가 `grep -oE '<TAG>: [^"]*'`로 뽑으므로 큰따옴표에서 잘린다. `>`도 피한다 —
JSON에서 `&gt;`로 새는 것을 실제로 봤다.

## 6. 에코가 필요한 경우가 있다

"보낸 것이 도착했나"를 재려면 서버가 *그것*을 되돌려줘야 한다. `netplay_server`도
소켓이 열리기 전 메시지를 받지만 **내용을 무시**해서 관측이 안 된다 — v0.150.2 버그가
네트워크 예제 넷이 있는 트리에서 살아남은 이유가 정확히 그것이다. 전용 에코 바이너리를
둔다: `wasm_failpaths_echo_server`(`examples/wasm_failpaths/echo_server.rs`).

## 7. 이름에 `_web_`을 넣는다

`scripts/*_web_smoke.sh`. "Chrome이 필요하다"가 문서 문장이 아니라 **파일명의 구조**가
되게 한다 — 삭제 전 트리엔 같은 `*_smoke.sh` 이름의 네이티브 스모크도 넷 있었다.

## 8. 진입점 유출은 빌드 게이트가 못 본다

`#[no_mangle]`은 wasm32로 **깨끗하게 컴파일된다.** `build_wasm_examples.sh`가 green인
채로 페이지가 시작조차 못 하는 상태가 #494에서 출하됐고, #496이 브라우저 스모크를
되살리자 첫 실행에서 잡혔다. 이 스킬이 존재하는 이유의 표본이다.
