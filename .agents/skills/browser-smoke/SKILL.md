---
name: browser-smoke
description: 엔진을 실제 브라우저에서 돌려 무언가를 단언할 때. 페이지가 스스로 판정해 `document.title`에 남기고 스크립트가 그것을 읽는다. 네이티브 헤드리스 인수 테스트는 `example-selftest`.
---

# browser-smoke

wasm으로 **컴파일되는** 것은 wasm에서 **도는** 것이 아니다. 빌드 게이트가 원리적으로
못 보는 것만 여기서 잰다.

## 계약
- 페이지가 `<TAG>: PASS — 근거` / `FAIL — 무엇이`를 **title**에 쓴다. 캔버스도
  콘솔도 아니다 — title은 페이지가 멎어도 밖에서 읽힌다.
- 스크립트가 Chrome DevTools `/json`을 폴링해 그 title을 읽는다.
- **타임아웃은 실패다.** 타임아웃 메시지에 *그 순간 본 값*을 싣는다. 안 그러면
  모든 사보타주가 같은 메시지로 죽어 데드라인만 검증된다.
- 진입점은 `#[wasm_bindgen]`이어야 한다. `#[no_mangle]`도 컴파일되지만 JS 바인딩이
  안 생겨 페이지가 시작조차 못 한다(2026-08-21 실제 유출).

플래그·포트·에코 서버·`--virtual-time-budget` 금지 → `references/gotchas.md`.
사보타주 절차는 `sabotage-check`.
