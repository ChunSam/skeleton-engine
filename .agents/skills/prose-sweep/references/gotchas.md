# prose-sweep — 분류표와 함정

근거는 v0.143.15(규칙 기록) → v0.143.17(스크립트 헤더 5개 + CLAUDE.md + WASM_SMOKES +
VERIFICATION + NEXT_WORK 2곳) → v0.153.0(`examples/` 삭제, 라이브 문서 15개) →
#559~#561(2026-09-04, 문서 242개 + 고아 산출물 181MB).

## 0. 지우기 전 — 이 스킬은 여기서 시작한다

산문 스윕은 **삭제 후**의 절반이다. 2026-09-04에 실제로 값을 한 것은 그 앞 네 단계였다.

1. **삭제 집합을 재도출한다. 이전 세션이 적어둔 숫자를 믿지 않는다.** 그날 인수받은 표는
   `22 / 207 / 8`이었고 실측은 `23 / 207 / 12`였다. 더 중요한 건 개수가 아니라 **살아 있는
   `src/audio/fixtures/README.md`가 후보에 섞여 있었다**는 것 — `Cargo.toml`과
   `src/audio/playback.rs`가 인용하는 파일이다. 목록을 파일로 떨어뜨리고 그것만 지운다.
2. **인용 감사를 live 문서 *와* 코드 양쪽에 돌린다.** 문서만 보면 `src/renderer/mod.rs`의
   doc 주석 같은 것을 놓친다. 후보마다 "누가 이걸 근거로 인용하는가"를 센다.
3. **인용이 *무엇을* 나르는지 본다.** 지울 문서 셋이 이미 지운 `docs/HANDOFF.md`를 "현재
   상태는 여기"라고 가리키고 있었다 — 남겨두면 노이즈가 아니라 **자신 있게 틀린 답**이 된다.
   반대로 살아 있는 내용이 그 안에만 있으면 먼저 옮긴다.
4. **추적되지 않는 파일이면 백업부터.** gitignore된 파일에는 `git show`가 없다. 지우기 전
   저장소 밖으로 tar를 뜨고, 경로를 사용자에게 알린다.

⚠️ **`git status`가 조용하다고 다 본 것이 아니다.** ignore된 잔재는 삭제 뒤에도 남는다 —
디렉터리마다 `git ls-files | wc -l`과 `find -type f | wc -l`을 비교해서 **tracked=0인데
files>0**이면 잔재다. 그렇게 181MB를 석 달 만에 찾았다(`session-cleanup`도 같은 항목을 든다).

## 분류표 — 이 저장소 기준

| 라이브 (고친다) | 날짜 기록물 (건드리지 않는다) |
|---|---|
| `CLAUDE.md`, `README.md`, `FORKING.md` | `docs/CHANGELOG.md`의 **과거 항목** |
| `docs/MODULE_MAP.md`, `docs/PATTERNS.md` | `docs/PROGRAM_HISTORY.md` |
| `docs/VERIFICATION.md`, `docs/NEXT_WORK.md` | `docs/ROADMAP.md` (스스로 *historical roadmap*이라 적음) |
| `docs/VISION.md`, `docs/AGENT_*.md` | |
| 서브시스템 설계 문서(`REMOTE_ENTITIES_DESIGN`, `SKELETAL`, `MACOS_FFI`) | |

CHANGELOG는 **한 파일 안에서 갈린다** — 새 릴리스 항목은 쓰고, 과거 항목은 안 고친다.

⚠️ **이 표는 2026-09-04에 줄었다. 예전 판이 들던 것들이 실제로 사라졌기 때문이다** —
`docs/HANDOFF.md`·`plans/handoffs/*`·`docs/CODE_ANALYSIS_*`는 #559에서, `REFERENCE.html`과
`ARCHITECTURE.html`은 2026-08-20에 삭제됐다. **추적 `*.md`는 이제 20개뿐이라 표가 곧
전수 목록에 가깝다.** 그러니 표를 외우지 말고 확인한다: `git ls-files '*.md'`.
(이 표가 넉 달간 없는 파일을 가리키고 있었다는 것 자체가 §9 — 스윕한 세션이 자기가 연
파일만 훑었다 — 의 사례다. 스킬의 참조 문서도 스윕 대상이다.)

## 1. 파일명 grep은 그 파일이 소유하던 플래그를 놓친다

v0.153.0에서 `selftests.sh` 참조는 전부 훑었는데, **그 스크립트만 읽던
`SKELETON_REQUIRE_AUDIO`** 는 남았다. 그 시점에 그것은 코드 어디에도 없는 죽은 이름이었는데
`docs/VERIFICATION.md`와 `docs/NEXT_WORK.md`는 **살아있는 도구처럼** 서술하고 있었다.

⚠️ **현재 상태는 반대다(2026-08-23 재확인).** 예제 재건이 그 플래그를 되살려
`scripts/selftests.sh:131,186`이 실제로 읽는다 — 문서 쪽이 다시 맞다. 이 항목은 **함정의
사례**이지 현 트리의 결함 목록이 아니다. §7이 처방하는 재확인을 이 파일 자신에게 돌려서
잡았다.

`docs/VERIFICATION.md`가 스스로 문서화한 *"Grep for the concept, not for the file you
were editing"* 함정에, **그 문서를 고치면서** 빠졌다.

절차: 지운 것이 독점하던 이름을 먼저 **목록으로 적고**(플래그, 환경변수, 스크립트명,
잡 이름, 고유 용어) 각각 `rg -c`로 센다. 파일명 하나로 끝내지 않는다.

## 2. 날짜 기록물을 고치고 싶은 충동이 가장 강한 순간이 위험하다

CHANGELOG 과거 항목이 "이제 틀린 말"로 보이지만 **그때는 맞았다.** v0.153.0에서
CHANGELOG·PROGRAM_HISTORY·HANDOFF·ROADMAP·CODE_ANALYSIS_*·plans/를 전부 남겼다.

## 3. `[gone]` 표시가 삭제보다 낫다

`docs/VERIFICATION.md`의 셀프테스트·스모크 섹션은 재구축이 **다시 마주칠 함정의
명세**다. 지우면 같은 값을 두 번 치른다. 섹션째 남기고 헤더에 `[gone]`을 단다.

## 4. 인용을 남길 땐 배너에 복구 명령을 함께 적는다

배너가 남기는 것은 "그건 없다 + git에서 꺼내라"다. **밀집 표는 스트립보다 배너가
낫다** — 행을 지우면 표의 구조까지 거짓이 된다.

```sh
git log --diff-filter=D --name-only -- <path>   # 지운 커밋 찾기
git show <commit>^:<path>                        # 그 시점 내용 꺼내기
```

## 5. 사실 주장은 스트립이 아니라 반전이다

"X가 CI에서 검증된다"가 거짓이 되면 문장을 **지우지 말고 뒤집는다.** v0.153.0에서
*"Web Audio is covered"* 를 지우는 대신 *"no audio claim of any kind is checked
anywhere but a unit test"* 로 바꿨다. 지우면 다음 사람이 "검증되나?"를 다시 조사하고,
뒤집으면 답을 이미 받는다.

## 6. 규칙이 미달 상태가 되면 폐기가 아니라 부채로 적는다

`CLAUDE.md`의 *"A feature is not done until a small playable example exercises it"* 은
예제가 사라진 뒤에도 **규칙으로 남겼고**, *"nothing currently satisfies it — treat that
as a standing debt, not as the rule having been relaxed"* 를 붙였다. 문장만 남기면
다음 세션이 없는 디렉터리에 예제를 만들려 하고, 지우면 규칙이 조용히 증발한다.

## 7. 고친 뒤 같은 grep을 다시 돌려 0을 확인한다

찾아낸 grep은 **고치기 전에만** 유효한 게 아니다. #498에서 낡은 주장 하나가 네 곳에
있었다. 주장 자체를 grep해 넷을 다 찾았는데 **둘만 고쳤고**, 나머지 둘(메모리 본문과
`MEMORY.md` 인덱스)은 병렬 세션이 따로 찾아냈다. 찾아낸 그 grep을 한 번만 더 돌렸으면
즉시 보였다.

⚠️ 이 항목을 쓰자마자 #501의 스크립트 rename에 적용했더니 **rename 목록이 놓친 살아
있는 참조 하나**(`Cargo.toml:72`)가 나왔다. n=2다.

⚠️ **n=3 (2026-08-25), 그리고 형태가 새롭다 — 재grep이 잡은 것은 *고치면서 내가 심은*
주장이었다.** `FORKING.md`의 낡은 문장을 뒤집어 쓰면서 "핫리로드는 자동 커버리지가 없다"를
딸려 옮겼다. 그 10분 전에 `RPG_QUEST_SELFTEST` 체크 6이 실제 파일 워처를 벽시계로 돌리는 것을
직접 읽어놓고도 그랬다 — 원문에서 문장을 옮겨 적는 순간 오염 경로가 하나 더 생긴다. 고친
주장을 다시 grep하는 단계에서 1이 떠서 잡혔다. **재grep은 놓친 곳을 찾는 절차가 아니라 새로
만든 곳도 찾는 절차다.** 같은 스윕에서 "4 of 6"은 **네 파일**에 있었다 — #498과 같은 숫자다.

같은 내용이 `docs/VERIFICATION.md` § *After fixing a claim, re-grep it and confirm zero*
에도 있다(#501). 중복이 아니라 분업이다 — 여기는 절차, 저기는 사건 기록.

## 8. 방향이 둘이다 — 돌아온 것도 산문을 남긴다 (2026-08-25)

이 스킬은 오래 **삭제·rename 한 방향만** 서술했고 description도 그랬다. 그래서 반대
방향에서는 트리거되지 않았다. 세 가지 형태가 모두 실제로 발생했다:

- **복구.** 인수 계층(`selftests.sh`, `build_wasm_examples.sh`, `wasm-smokes` 잡)이
  2026-08-19에 삭제됐다가 08-21까지 되살아났는데, `docs/VERIFICATION.md` 배너는
  08-25까지 *"전부 사라졌다, 스모크 16개 전부"* 라고 말하고 있었다. 실제로는 러너·wasm
  빌드·잡이 전부 돌아왔고 스모크도 3개 살아 있었다. **`[gone]` 마커가 붙은 섹션 하나는
  이미 `[rebuilt]` 였어야 했다** — 3번 항목의 마커는 유지보수 대상이지 영구 라벨이 아니다.
- **revert.** `scripts/selftests.sh` 헤더가 *"CI provisions a PulseAudio null sink"* 인 채로,
  그 실험을 되돌린 **바로 그 커밋에서** 살아남아 v0.143.10부터 #426까지 갔다. revert diff는
  세 파일 건너의 주석을 보여주지 않는다.
- **재건.** `plans/2026-08-19-examples-rebuild-plan.md`가 다섯 게임이 다 지어진 뒤에도
  *"Nothing here is built yet"* 으로 시작했다 — **같은 파일의 phase 표가 done 이었는데도.**

판정은 삭제 때와 같다(라이브인가 기록물인가). 다만 **grep 대상이 반대**다: 지운 이름이
아니라 *"없다 / 사라졌다 / gone / no longer"* 로 **단언하는 문장**을 찾는다.

## 9. 스윕을 돌린 다음 날 다섯 개가 더 나왔다 — 열어둔 파일만 훑었기 때문 (2026-08-25)

#503이 바로 이 §8을 근거로 스윕을 돌렸다. 제목이 *"the acceptance layer came back and **three**
documents did not notice"* 다. 하루 뒤 같은 개념 grep을 돌리니 **다섯 개가 더** 나왔다:

| 놓친 파일 | 얼어 있던 문장 |
|---|---|
| `README.md` | "다섯 중 **둘** 들어옴, 나머지 셋은 아직" |
| `docs/VISION.md` | "루프는 **절반만** 재건 — 오디오·wasm·GPU 파티클·네트워킹은 기준 미달" |
| `FORKING.md` | "실제 브라우저에서도 실제 플레이에서도 **아무것도** 확인하지 않는다" |
| `docs/MODULE_MAP.md` | "**모든** 인용이 없는 파일을 가리킨다 / 이 파일의 어떤 것도 검증되지 않았다" |
| `CLAUDE.md` | "핫리로드 — 커버리지 **전무**" (실제로는 `RPG_QUEST_SELFTEST` 체크 6이 덮는다) |

스크립트 헤더 둘(`verify.sh:36`, `build_wasm_examples.sh:9`)도 `wasm-smokes`를 **아직 오지 않은
phase 5** 로 서술하고 있었다.

**메커니즘은 §1의 일반형이다: 그 세션이 이미 열어둔 파일만 훑었다.** #503은 `VERIFICATION.md`와
재건 계획서를 고치는 중이었고, 스윕이 정확히 거기서 끝났다. 개념 grep은 열려 있는 버퍼가 아니라
**저장소 전체**에 돌려야 한다 — 라이브 문서 목록(§ 분류표)을 명시적으로 인자에 적는다.

⚠️ **분포가 개수보다 중요하다.** 되돌아온 변경 하나는 문서 한 곳이 아니라 *그것을 서술한 모든
문서*를 동시에 낡게 만들고, 각자 **다른 날짜에** 얼어붙는다. 그래서 서로가 서로를 뒷받침한다 —
MODULE_MAP과 VISION이 둘 다 "없다"고 말하면 교차 확인처럼 읽힌다. 한 곳을 고쳐 안심하는 것이
이 함정의 본체다.

⚠️ **`<NAME>_SELFTEST` 이름이 *다른 파일에* 되살아나는 경우가 최악이다.** `SURVIVOR_SELFTEST`,
`RPG_QUEST_SELFTEST`, `NETPLAY_SELFTEST` 셋 다 재건된 트리에 존재하지만 **다른 게임의 다른
체크**다. 죽은 이름은 grep이 0을 내서 잡히는데, **되살아난 이름은 1을 내며 현재형으로 읽힌다.**
MODULE_MAP 배너에 이 셋을 이름으로 박아둔 이유다.

⚠️ **정정 — 단위는 '파일'이 아니라 '주장'이다 (2026-08-26).** 위 문단은 처방을 *"열려 있는 버퍼가
아니라 저장소 전체에 돌려라"* 로 적었는데, 그것으로도 부족하다는 게 다음 날 드러났다. #508 스윕은
`docs/NEXT_WORK.md`를 **여섯 군데나 편집하면서** 같은 파일 421번째 줄의 *"the decision it exposed is
still open"* 을 지나쳤다 — 그 결정은 2026-08-08에 닫혔고, **같은 파일의 `Open — process` 섹션이
줄곧 "closed"라고 말하고 있었다.** 파일은 열려 있었다. 열려 있지 않았던 것은 *그 주장*이다.

그러니 §7의 재grep은 **고친 주장마다** 돌려야 하고, §9의 전수 grep은 **주장의 목록**으로 돌려야 한다.
"이 파일은 봤다"는 아무것도 보장하지 않는다 — 한 파일 안에서 두 섹션이 서로 반대되는 말을 하고
있을 수 있고, 실제로 그랬다.

⚠️ **자기가 방금 만든 드리프트가 가장 안 보인다.** 같은 세션에서 `APPROACH`를 40 → 70으로 되돌리고,
그 값을 인용하던 닫힌 행("70 → 40 roughly doubles it")은 고치지 않았다. 코드를 바꾼 지 몇 시간 만이고,
그 행을 그날 아침에 직접 읽었는데도 그랬다. **커밋 직전에 diff가 건드린 상수·플래그·경로를 목록으로
뽑아 각각 grep하라** — 남이 남긴 산문보다 자기가 방금 낡게 만든 산문이 더 위험하다.
