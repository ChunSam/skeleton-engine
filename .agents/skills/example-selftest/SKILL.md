---
name: example-selftest
description: 예제에 `NAME_SELFTEST=1` 헤드리스 인수 테스트를 추가하거나 고칠 때. CI가 만들 수 없는 것(실기기 오디오, 실시간 신호, 창 있는 재생)을 예제 스스로 증명하게 만든다. 스크린샷이나 `ENGINE_CAPTURE`로는 검증되지 않는 기능에 사용.
---

# example-selftest

`<GAME>_SELFTEST=1`로 예제를 창 없이 돌려 CI가 못 만드는 것을 증명한다.
전례는 세어서 본다: `grep -rhoE '"[A-Z_]+_SELFTEST"' examples/ | sort -u`

## 형태
- `main` 첫 줄에서 env 검사 → `std::process::exit(self_test())`,
  `#[cfg(not(target_arch = "wasm32"))]`로 감싼다.
- 셋업은 게임과 **같은 함수**를 부른다. 하니스가 복제하면 그 셋업의 버그는
  영원히 안 잡힌다.
- 게임의 진짜 시스템을 `main` 순서대로 tick 한다. 로직을 재구현하지 않는다.
- 검사마다 고유 exit code. `SKIP` + exit 0은 **오디오 장치 없음**만 — 나머지 부재는 실패다.

시간·`ENGINE_CAPTURE`·배경 스포너 함정 → `references/gotchas.md`.
네트워크 예제(서버 스폰·포트·SKIP 규칙) → `references/networked.md`.
새 검사를 사보타주로 검증하는 절차는 `sabotage-check` 스킬.
