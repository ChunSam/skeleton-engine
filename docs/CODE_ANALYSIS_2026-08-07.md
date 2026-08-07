# skeleton-engine 전체 코드 분석 — 2026-08-07

> 대상: `main` @ `4867ab9`, 패키지 v0.145.0 (crate `engine`), `src/` 273 파일 / 83,110 LOC,
> 예제 146개, `#[test]` 1,421개.
> 방법: 15개 병렬 finder(서브시스템 11 + 교차 렌즈 4) → **후보마다 적대적 검증 에이전트**
> (인용 코드를 직접 열람, "반증하라"가 기본값) → HIGH는 2차 회의론자 재검. 총 107 에이전트.
> **원시 후보 89건 → 확정 55 · 반증 3 · 미검증 31**(세션 토큰 한도로 검증 에이전트 33개 사망).
> 미검증분 중 HIGH 5건은 보고서 작성자가 직접 코드를 열어 재검증했다(§2).

---

## 0. 총평 — 이미 여러 번 훑은 코드베이스다

먼저 **깨끗한 것부터**. 이건 정보다:

- `cargo clippy --all-targets` **exit 0, 경고 0**. `fmt`, `cargo doc -D warnings`도 CI에서 강제.
- `src/` 전체에 `TODO`/`FIXME`/`HACK`/`todo!()`/`unimplemented!()` **0건**.
- **ECS 코어는 튼튼하다.** 엔티티 세대(generation) 처리가 정확하고 회귀 테스트로 직접 고정돼 있다.
  아카타입 이동, `query*_mut`의 `get_disjoint_mut` 앨리어싱 — 검증 결과 문제 없음.
- **CLAUDE.md의 검증 가능한 주장은 전부 사실이었다** (별도 측정):
  178줄 ≤ 200 ✓ · `HEADLESS_SHOT` 41/109 = 37% ≈ "about 40%" ✓ · 셀프테스트 정확히 10/21 ✓ ·
  브라우저 스모크 디스크 11개 중 `wasm-smokes` 잡에서 정확히 5개 게이트 ✓ · 버전 문자열은
  `Cargo.toml`에만 ✓. 문서가 이렇게 안 썩은 저장소는 드물다.
- 과거 분석(2026-06-05 30건, 06-16 80건, 06-23, 06-26)에서 지적된 항목들은 **재발하지 않았다.**
  autotile 비트순서, 게임패드 축 `just_pressed`, 텍스처 캐시 키 동일성 — 전부 오늘 코드에서 정상이고
  일부는 테스트로 고정돼 있다(`src/asset/tests.rs:257`).

그래서 이번에 나온 건 대부분 **더 깊은 층**이다. 확정 55건의 심각도 분포는 **HIGH 4(실질 3) ·
MEDIUM 32 · LOW 19**이고, 가장 두꺼운 군집은 여전히 이 저장소의 지배적 결함 패턴인
**조용한 실패(fail-quiet)** 와 **fork 사용자 함정**이다.

### 가장 먼저 할 일

**`src/app/render/frame.rs` 하나만 고치면 확정된 HIGH 4건 중 4건이 전부 닫힌다.** GPU 파티클
링 커서 1줄(`:458`), 포맷 파이프라인 가드 위치(`:452`), 도킹 에디터 포스트체인(`:529`/`:162`) —
전부 같은 파일, 전부 `small`~`medium`이고 세 개는 사실상 한 줄짜리다. 이게 압도적으로 비용
대비 효과가 크다. 그다음이 **macOS 게임패드 입력 유실**(§2, 직접 재검증한 HIGH)인데, 이건 CI가
구조적으로 볼 수 없는 영역이라 아무도 눈치채지 못한 채 남아 있었다. 그 뒤로는 §3의 fail-quiet
군집을 순서대로 — 특히 `register_event`/`.after()` 계열의 **"등록을 잊으면 조용히 무시된다"**
3건은 fork 사용자가 정확히 밟는 지뢰다.

---

## 1. 즉시 수정 — `app/render/frame.rs` 클러스터 (확정 HIGH)

네 건 모두 적대적 검증을 통과했고, 보고서 작성자가 코드를 직접 열어 한 번 더 확인했다.

| 항목 | 위치 | 문제 | 수정 | 심각도 | 노력 |
|---|---|---|---|---|---|
| **GPU 파티클 링 커서가 매 프레임 0으로 리셋** | `src/app/render/frame.rs:458` | `let mut frame_cursor = 0u32;`가 렌더 함수 **로컬**이다. `collect_new_particles`가 이 커서로 capacity 크기 버퍼의 슬롯을 배정하는데, 매 프레임 0부터 다시 배정하므로 **직전 프레임에 살아 있던 파티클을 그대로 덮어쓴다** | 커서를 프레임 간 유지 — `GpuParticleRenderer`에 `frame_cursor: u32` 필드를 두고 `&mut`로 넘긴다. 프레임 내 에미터 간 슬롯 분리는 그대로 유지됨 | **HIGH** | small |
| **포맷 매칭 파이프라인이 `if has_emitters` 안에서만 빌드됨** | `src/app/render/frame.rs:452` | `ensure_render_pipeline(device, scene_format)`은 `if has_emitters` 블록 안인데, 바로 아래 `if let Some(gpr) = …`의 `dispatch_compute` + `render`는 **렌더러가 존재하기만 하면 무조건 실행**된다. 이 저장소가 문서로 못박은 *"파이프라인 캐시는 타깃 포맷으로 키잉"* 불변식이 깨지는 지점 | `ensure_render_pipeline` 호출을 `has_emitters` 가드 밖, 실제 패스를 기록하는 `if let Some(gpr)` 블록 안(`as_mut()`)으로 옮긴다 | **HIGH** | small |
| **도킹 에디터에서 포스트/라이팅 패스가 씬이 그린 적 없는 중간 텍스처로 게임 뷰포트를 덮어씀** | `src/app/render/frame.rs:529` | 도킹 모드에서 씬은 포스트 중간 텍스처를 우회하는데, 3.5/4/4.5 단계가 그대로 돌아 뷰포트를 그 중간 텍스처로 덮는다 | `docked_render_view.is_some()`일 때 포스트/라이팅 체인을 건너뛰고 1회 warn, 또는 도킹 타깃으로 제대로 라우팅 | **HIGH** | medium |
| **같은 원인의 앞단 — 도킹 모드에서 포스트/블룸 셋업 자체가 실행됨** | `src/app/render/frame.rs:162` | 게임 뷰포트가 검게 blank 되거나, HDR이 켜져 있으면 **wgpu 포맷 검증 에러** | `let use_post = pp_config.map(|c| c.enabled).unwrap_or(false) && docked_render_view.is_none();`를 셋업 앞에서 계산 | MEDIUM | small |

> 검증 메모: `frame.rs:458`은 서로 독립적인 두 finder(`app-render`, `world-content`)가 각각
> 발견했다. 네 건 모두 1차 적대적 검증은 통과했으나 **2차 회의론자는 세션 한도로 사망**했다 —
> 그래서 작성자가 직접 코드를 열어 커서가 로컬인 것과 가드 위치를 눈으로 확인했다(둘 다 사실).

---

## 2. 검증이 유실된 HIGH — 작성자가 직접 재검증한 결과

교차 렌즈 5개의 검증 에이전트가 **전멸**해서(§10), 그 렌즈들이 올린 HIGH 5건을 작성자가 직접
코드를 열어 판정했다.

| 후보 | 판정 | 근거 |
|---|---|---|
| **macOS 게임패드 백엔드가 `just_pressed`/`just_released`를 누적하지 않고 덮어씀** (`src/input/gamepad.rs:209`) | ✅ **확정 · HIGH** | gilrs 경로는 `slot.just_pressed.insert(gb)`로 **누적**하는데(`:151`), macOS 경로는 `slot.just_pressed = just_pressed`로 **대입**한다. `flush()`는 프레임당 1회(`app/schedule.rs:555`)인데 `apply_macos_snapshot`을 부르는 `about_to_wait`는 **이벤트 루프 iteration마다** 실행된다(`app/window.rs:555`, `WaitUntil` + "입력 이벤트는 즉시 깨움"). 즉 마우스가 움직이는 동안 한 프레임 사이에 스냅샷이 여러 번 찍히고, 두 번째 스냅샷에서 `slot.pressed`에 이미 버튼이 있으므로 diff가 비어 **눌림 엣지가 프레임에 도달하기 전에 지워진다**. `gamepad.rs:190-193` 주석이 "`pressed`는 flush를 넘어 유지되고 flush는 `just_*` 엣지만 지운다"고 스스로 명시 — 대입은 그 계약 위반 |
| **씬 `Replace`가 `AssetServer`/`ScriptRegistry`를 통째로 날림** (`src/app/core_resources.rs:37`) | ✅ **확정 · MEDIUM** | `reload_scene`(`app/scenes.rs:26+`)은 `persistent_resources` + `DebugUi`만 살린다. `insert_core_resources`는 `AssetServer::new()`와 `ScriptRegistry::default()`를 **새로** 꽂고, 둘 다 persistent 목록(`app.rs:296-339`)에 없다. 결과: 시작 시 로드한 아틀라스·스크립트·파일 워치가 씬 전환마다 **조용히** 사라진다. GPU 텍스처 캐시는 `App.render`에 있어 살아남으므로 평범한 `Sprite`는 멀쩡한데 `AtlasSprite`/`ScriptRunner`만 죽는 것이 특히 진단을 어렵게 한다. **CLAUDE.md 자신의 규칙과 충돌** — *"config, device handles, **caches** → must persist"*. `AssetServer`는 명백히 캐시다 |
| **`HierarchySystem`이 엔티티당 힙 블록 2개를 매 프레임 할당** (`src/hierarchy.rs:259`) | ✅ **확정(중복) · MEDIUM** | 독립 확정 건 `src/hierarchy.rs:234`와 동일 결함. §6 표에 이미 포함 |
| **wasm에서 `App::load_image`가 GPU 초기화 시 가짜 asset failure를 기록** (`src/renderer/texture.rs:101`) | ⚠️ **부분 확인 · 후속 필요** | `try_from_path_with_format`에 **cfg 게이트가 없고** `std::fs::read`를 무조건 호출한다(`texture.rs:101`) — wasm에서는 절대 성공할 수 없다. 다만 wasm의 `load_image`가 실제로 이 경로로 흘러드는지(async fetch 경로로 빠지는지)는 확인하지 못했다 |
| **`AssetServer→GPU` 업로드가 canonical 키만 등록 → `Sprite::textured(path)`가 흰색** (`src/renderer/sprite/textures.rs:89`) | ❌ **반증** | `image_assets_for_gpu`(`asset/image_loading.rs:157`)는 `path_to_id`의 키를 **그대로**(`path.to_string()`) 돌려주고, 업로드 seam(`app/assets.rs:265`)이 그 키를 쓴다. 게다가 `src/asset/tests.rs:257`이 2026-05-29 흰색-스프라이트 버그를 겨냥해 **이 불변식을 명시적으로 고정**하고 있다. 후보가 지목한 메커니즘은 오늘 코드에 존재하지 않음 |

---

## 3. 조용한 실패 (fail-quiet) — 이 저장소의 지배적 결함 패턴

*"잘못된 입력·누락된 등록을 만나도 panic도 log도 없이 조용히 틀린 동작을 한다."*
fork 친화 스켈레톤이라는 정체성에 정면으로 반하는 계열이며, 이번에도 가장 두껍다.

| 항목 | 위치 | 문제 | 수정 | 심각도 | 노력 |
|---|---|---|---|---|---|
| **`.after(X::LABEL)`가 X를 `add`/`add_system`으로 등록했으면 조용히 폐기됨** | `src/ecs/schedule.rs:77` | 문서에 실린 Scene 순서 지정 레시피를 그대로 따라도 **제약이 0개** 생성된다 | 매칭 안 되는 label에 `log::warn!`; 대상 시스템을 label 있는 등록으로 강제 | MEDIUM | medium |
| ↳ 같은 뿌리: **존재하지 않는 label을 쓴 `after()`/`before()`가 무경고 폐기** | `src/ecs/schedule.rs:77` | dangling 순서 참조는 항상 버그지 의도가 아니다 | `compute_order` 후 미매칭 label 수집 → warn | LOW | small |
| **`Events<UiEvent>`가 미등록 시 완전 무음으로 버려짐** | `src/ui/system/state.rs:181` | 엔진 이벤트 버스 중 **유일하게** warn-once가 없다. 모든 버튼 클릭·슬라이더·드롭다운이 사라지는데 엔진은 멀쩡해 보임 | `TriggerZoneSystem::warned_no_bus` 패턴 복제 | MEDIUM | small |
| **`Hidden`/`RenderLayer`/`SpriteFlip`/`YSort`가 serde 미등록 → 씬 저장에서 조용히 소실** | `src/app/core_resources.rs:103` | 에디터로 추가 가능하고 clone 등록도 돼 있는데 serde만 빠져, 저장→로드하면 사라진다 | 기존 `if let Some(registry)` 블록에 4줄 추가 | MEDIUM | small |
| **`spawn_entity_def`가 `EntityDef.parent`를 무시** | `src/prefab.rs:294` | 에디터의 삭제-되돌리기 / 붙여넣기 / 프리팹 스폰이 **계층을 조용히 평탄화** | `def.parent` 존중 → `hierarchy::attach`, 없으면 warn | MEDIUM | small |
| **웹에서 positional 채널이 analyser에 연결되지 않음** | `src/audio_wasm.rs:926` | `Audio::levels()`가 웹에서 **영원히 무음**을 보고 — native는 정상 계측 | `play_at_on_channel`에서 채널명을 미터로 전달 | MEDIUM | small |
| **`set_current_state`(에디터 "이 상태로 점프")가 `AnimationPlayer`를 재지정하지 않음** | `src/animation/state_machine.rs:301` | SM 상태와 화면에 보이는 클립이 어긋나고, `AnimationEnd`로만 빠져나가는 상태는 **영원히 탈출 불가** | `StateMachineSystem::run`에 멱등 재동기화 추가 | MEDIUM | small |
| **음수 `velocity_spread` / `EmitShape::Box` 반폭이 `gen_range` 빈 범위로 panic** | `src/particle/mod.rs:360` | 샘플링 지점에서 `.abs()` (4곳) | 〃 | MEDIUM | small |
| **저장이 비원자적** — `fs::write` 중 크래시 시 잘린 파일 | `src/save.rs:137` | "손상되었거나 변조됨"으로 보고되고 폴백이 없다 = **세이브 데이터 손실** | temp 파일 write → `fs::rename` (POSIX/Windows 모두 원자적) | MEDIUM | small |
| **`Replace` 프레임이 `ViewportSize` 1280×720 기본값 + `Letterbox` 없이 렌더** | `src/app/schedule.rs:567` | `compute_viewport`는 update의 1단계, `SceneCmd` 소비는 10단계, `render()`는 그 뒤 → 씬 전환마다 한 프레임 튄다. `PATTERNS.md`의 *"다음 틱에 self-heal"* 근거가 `render`에는 거짓 | `apply_scene_cmd` 뒤 `compute_viewport()` 재실행 | LOW | small |
| **`DataTable::load`는 `resolve` 경유, `save`는 raw 경로** | `src/data_table.rs:191` | 에디터가 **성공했다고 보고하면서 다른 파일에 쓴다** | 로드 시 resolved 경로를 함께 보관 | LOW | small |
| **`sync_static_from_tilemap`이 셀 *존재*만 diff** | `src/physics/world/tile_collider.rs:164` | 타일맵을 옮기거나 스케일하면 콜라이더가 **옛 위치에 그대로** | 인덱스에 `origin`/`tile_size`/`ppu` 저장 후 변하면 전체 재빌드 | LOW | small |

---

## 4. fork 사용자 함정 — "올바르게 보이는 호출이 틀린다"

| 항목 | 위치 | 문제 | 수정 | 심각도 | 노력 |
|---|---|---|---|---|---|
| **엔티티 despawn 시 rapier 강체/콜라이더가 살아남음** | `src/physics/body.rs:5` | 보이지 않는 단단한 유령이 계속 충돌하고, 무한히 샌다 | `despawn_with_body(world, entity)` 헬퍼 + `PhysicsBody` 문서에 경고(`TilemapColliders` 문구 미러) | MEDIUM | medium |
| **`FontData`가 fontdb에 로드되지만 family에 바인딩되지 않음** | `src/renderer/text/renderer.rs:533` | 셰이퍼가 항상 `Family::SansSerif`를 요청 → native에서 **게임 폰트가 폴백으로만** 쓰임 | 첫 face의 family 이름을 잡아 `db.set_sans_serif_family(name)` | MEDIUM | medium |
| **셀프 전이(A→A)가 발동하지만 완전한 no-op** | `src/animation/player.rs:74` | 클립이 재시작하지 않고 `finished`도 안 지워짐 | `AnimationPlayer::restart()` 추가 후 SM이 호출 | MEDIUM | small |
| **음수 `SkeletalAnimator::speed` + 비루프 클립 → `time`이 무한히 음수** | `src/skeletal.rs:222` | `is_finished()`가 영원히 false | 비루프 분기를 `clamp(0.0, duration)`; 역재생의 `is_finished` 의미를 정하고 문서화 | MEDIUM | small |
| **`max_per_frame`이 스폰 *루프*가 아니라 루프 *뒤* 개수를 캡** | `src/particle/mod.rs:344` | `spawn_rate: f32::INFINITY`가 **프레임을 영구 정지**시킴 | 닫힌 형태로 교체(`(timer/interval).floor()`), `interval` 유한성 가드 | MEDIUM | small |
| **`Pool::release`가 이미 반납된 엔티티를 수락** | `src/pool.rs:63` | `acquire`가 **같은 `Entity`를 서로 다른 두 호출자에게** 건넴 | `Pooled` 보유/생존 검사 후 조기 반환 + warn | MEDIUM | small |
| **`Selector`가 `Running`에 래치** | `src/behavior.rs:268` | 우선순위 높은 자식이 재평가되지 않아, 교과서적 우선순위 셀렉터 AI가 **반응을 멈춘다** | 래치를 문서화 + `ReactiveSelector` 추가, 또는 옵션 플래그 | LOW | medium |
| **`step_headless`가 `FrameConfig::max_dt` 클램프를 건너뛰고 `ShouldQuit`을 무시** | `src/app/schedule.rs:176` | "run이 도는 프레임 스텝에서 그리기만 뺀 것"이라는 문서와 불일치 → 셀프테스트가 실제 프레임과 다른 것을 검증 | `step_headless` 안에서 `FrameConfig.cap(dt)` 적용 | LOW | small |
| **`TextQueue`/`UiQueue`/`UiImageQueue`/`DebugDraw`가 `render()` 안에서만 비워짐** | `src/renderer/text/queue.rs:178` | `App::step_headless`가 이들을 **무한히 키운다** | GPU 없을 때 `update` 끝에서 비우기 | LOW | small |
| **`SystemPanicPolicy::AbortAfterLog`가 `exec_order`를 `mem::take`한 채 unwind** | `src/app/schedule.rs:486` | 스케줄이 **영구히 빈 상태**로 남음 | unwind 전 `self.exec_order = order;` 복구 | LOW | small |
| **`RenderPlugin`만 레터박스 clip scale을 못 받음** | `src/renderer/render_plugin.rs:53` | 문서대로 만든 플러그인이 `DesignResolution`에서 어긋남 | 트레이트에 투영 공식 명시(비파괴 1단계) | MEDIUM | small |

### 에디터 (실질적으로 fork 사용자가 가장 먼저 만지는 표면)

| 항목 | 위치 | 문제 | 수정 | 심각도 | 노력 |
|---|---|---|---|---|---|
| **월드 리셋/씬 로드 시 undo 히스토리가 안 지워짐** | `src/app/scenes.rs:62` | 낡은 `Entity` 핸들이 새 World의 **살아 있는 다른 엔티티를 가리킴** | `EditorHistory::clear()` 추가 후 `reset_world`/씬 로드 두 곳에서 호출 | MEDIUM | small |
| **Ctrl+D 복제가 부모 링크를 버림** | `src/app/editor/ui/shortcuts.rs:174` | 복제된 자식이 루트가 되며 **다른 월드 좌표로 점프** | 원본 `Parent`를 읽어 재수립 | MEDIUM | small |
| **부모 삭제 시 자식이 dangling `Parent`로 고아가 됨** | `src/app/editor/ui/mod.rs:271` | Scene 트리에서 **도달 불가** | despawn 전 `hierarchy::detach` | MEDIUM | small |
| **다중 선택 액션이 엔티티당 `EditorCmd` 1개씩 push** | `src/app/editor/ui/shortcuts.rs:148` | "3개 삭제됨"을 되돌리려면 Ctrl+Z 3번, 중간 상태가 반쯤 적용됨 | `EditorCmd::Group(Vec<_>)` (`PaintTiles`가 이미 스트로크를 배치 처리) | LOW | medium |

---

## 5. 정확성 — 나머지 확정 건

| 항목 | 위치 | 문제 | 수정 | 심각도 | 노력 |
|---|---|---|---|---|---|
| **`parse_color`가 바이트 길이 검사 후 `&str`를 바이트 슬라이싱** | `src/renderer/text/rich_text.rs:103` | 6/8바이트 **비-ASCII `[color=…]` 값이 panic** | `strip_prefix('#')` 뒤 `if !hex.is_ascii() { return None; }` | MEDIUM | small |
| **라이팅 `aspect_ratio`가 윈도우가 아닌 *디자인* 뷰포트 사용** | `src/renderer/lighting.rs:428` | `DesignResolution` 레터박스에서 **점광이 타원**이 됨 | clip_scale로 윈도우 공간에 재기준화 + 비항등 clip_scale 단위 테스트 | MEDIUM | small |
| **`SpatialGrid`가 `GlobalTransform`을 무시하고 로컬 좌표로 인덱싱** | `src/collision/grid.rs:101` | 부모에 붙은 히트박스가 **스프라이트와 다른 위치에서** 충돌 | `GlobalTransform` 우선(borrow split 주의: 먼저 collect) | MEDIUM | small |
| **`PhysicsSystem`이 회전잠금/키네마틱 바디의 `Transform.rotation`을 0으로 덮음** | `src/physics/system.rs:279` | 문서는 no-op이라 주장하고, 가드 테스트가 이걸 못 봄 | `is_rotation_locked()` 검사 후 스킵, 또는 `sync_rotation: bool` | MEDIUM | small |
| **`TextInput::text`만 대입하고 `cursor`를 리셋 안 하면 매 프레임 UI 패스에서 panic** | `src/ui/text_input.rs:227` | 4개 읽기 지점에서 `min(cursor, len)` 후 char boundary로 스냅 | 〃 | MEDIUM | small |
| **크로스페이드 임시 채널이 bus + base volume을 잃음** | `src/audio/playback.rs:586` | 페이드 아웃되는 트랙이 페이드 내내 **최대 음량으로 점프** | 임시 채널명에 원 채널의 믹서 상태 복사 | MEDIUM | small |
| **satisfied dead-edge 전이가 스캔 전체를 중단** | `src/animation/state_machine.rs:414` | 그 상태의 **모든 하위 우선순위 전이가 차단**됨. 자기 문서/테스트 주석과 모순 | `?`를 `continue`로 | LOW | small |
| **`PointerCapture`가 `focus_pass` 전에 재구성됨** | `src/ui/system.rs:117` | `focus_pass`가 `Dropdown::open`을 토글할 수 있어 **그 프레임 내내 capture가 낡음** | 드롭다운 토글을 `dropdown_pass`로 이동 | LOW | small |
| **Tab/Enter 포커스가 `PointerCapture`를 무시** | `src/ui/system/focus_pass.rs:126` | 모달 패널에 가려진 버튼이 **키보드로는 여전히 활성화** 가능 | `collect_focusables`에 occlusion 테스트(툴팁 패스가 이미 `occludes` 사용) | LOW | medium |

---

## 6. 프레임당 할당 / 성능

저장소 자신의 규칙: *"매 프레임 도는 시스템은 임시 버퍼를 scratch 필드(`clear()`+refill)나
`std::mem::take`로 유지한다."* 아래는 전부 **built-in 시스템** — 즉 모든 게임이 비용을 낸다.
(에디터/일회성 경로는 규칙상 면제이므로 제외했다.)

| 항목 | 위치 | 문제 | 수정 | 심각도 | 노력 |
|---|---|---|---|---|---|
| **`HierarchySystem`이 엔티티당 컨테이너 5개 + 힙 `Box` 1개를 매 프레임** | `src/hierarchy.rs:234` | `Parent`가 하나도 없는 게임도 비용을 냄. 모든 게임이 내는 built-in | scratch 필드화(`HierarchySystem`을 unit struct → `#[derive(Default)]` 구조체로) | MEDIUM | small |
| **`DialogueSystem`이 매 프레임 `LocaleResource` 전체를 딥클론** 후 모든 박스의 줄을 재구성 | `src/dialogue/system.rs:39` | `LocalizationSystem` 패턴 미러 — 대상 수집 후 비면 조기 반환 | 〃 | MEDIUM | medium |
| **`TilemapSystem`이 "변한 것 없음" 빠른 경로 *앞에서* 타일 격자 전체를 딥클론** | `src/tilemap/system.rs:153` | 2026-06-16 수정이 클론의 절반만 덮었다 | `(generation, dims)`를 먼저 읽고 bail 후에만 클론 | LOW | medium |
| **`AnimEffectSystem`이 이벤트 유무 확인 *전에* 바인딩 테이블 전체를 딥클론** | `src/anim_effect.rs:165` | 순수 순서 바꾸기 | 이벤트 스냅샷 먼저 → 비면 return | LOW | small |
| **`ZoneEffectSystem`도 동일 패턴** | `src/zone_effect.rs:184` | 〃 | 〃 | LOW | small |
| **`LayoutSystem`이 프레임당 `Vec` + 패널당 children 클론** | `src/ui/panel.rs:160` | `UiSystem`처럼 scratch 필드 3개 | 〃 | LOW | medium |
| **`upload_asset_server_images_to_gpu`가 매 프레임 전체 재스캔** (`Vec` + 이미지당 `String`) | `src/app/assets.rs:262` | `AssetServer`에 명시적 업로드 큐(`pending_gpu_upload`) 도입 | 〃 | LOW | medium |
| **`DebugDraw` 선/원이 `thickness` 크기 점마다 UI 쿼드 인스턴스 1개씩** | `src/app/render/debug_draw.rs:34` | 세그먼트당 회전 쿼드 1개로(`ui_quad_instance`에 회전 파라미터 추가 — 이미 `Quat` 경유, `IDENTITY`로 고정돼 있음) | 〃 | LOW | medium |

---

## 7. native / wasm 계약 불일치

문서화된 규칙: *"cfg 분기 백엔드는 구현이 아니라 **정책**을 공유한다 — 파생값의 *공식*을 양쪽이
호출하는 비게이트 모듈에 둔다. 안 그러면 두 플랫폼이 조용히 갈라진다."*

| 항목 | 위치 | 문제 | 수정 | 심각도 | 노력 |
|---|---|---|---|---|---|
| **native `update_position`이 재생 중인 소리의 스테레오 팬을 전혀 움직이지 않음** | `src/audio/positional.rs:87` | 모노 클립은 팬 자체가 없다. 웹은 정상 → **위치 기반 오디오가 플랫폼별로 다른 게임** | `PannedSource`에 `Arc<[AtomicU32;2]>` 라이브 팬, 모노는 업믹스 | MEDIUM | medium |
| **채널 이펙트를 걸면 native 톤의 de-click 엔벨로프가 벗겨짐** | `src/audio/playback.rs:203` | 상수 주석이 스스로 주장하는 native↔wasm 패리티가 깨짐 | 엔벨로프를 `None` 분기 밖으로, 양쪽에서 `SamplesBuffer` 기반으로 빌드 | MEDIUM | small |
| **`bands()`의 밴드→주파수 매핑이 샘플레이트를 무시** | `src/audio/analysis.rs:214` | "밴드 N"이 native/웹에서, 그리고 에셋마다 **다른 Hz 대역**을 뜻함 | 공유 정책을 bin이 아니라 Hz 기반으로(`log_band_hz_range`) | LOW | medium |
| (§2) **웹 positional 채널이 analyser 미연결** | `src/audio_wasm.rs:926` | §3 참조 | | MEDIUM | small |

> 참고: wasm 드리프트 렌즈는 **619개 `cfg(target_arch = "wasm32")` 사이트를 73개 파일에 걸쳐
> 전수 열람**했고 6건을 올렸으나, 검증 에이전트가 전멸해 위 표에는 다른 렌즈가 확정한 것만 실었다.
> 미검증 후보는 §10.

---

## 8. 문서 드리프트

CLAUDE.md의 정량 주장은 §0에서 확인했듯 전부 정확했다. 아래는 **코드 주석/모듈 문서** 쪽 드리프트다.

| 항목 | 위치 | 실제 |
|---|---|---|
| `blob_47` 독 주석의 47-마스크 나열이 **3줄 아래 `VALID_MASKS` 테이블과 불일치** | `src/tilemap/autotile.rs:110` | 낡은 나열을 지우고 `VALID_MASKS`를 단일 출처로 지목 |
| animation 모듈 문서가 `StateMachineSystem` 라벨을 `"engine::state_machine"`이라 표기 | `src/animation/mod.rs:17` | 실제 상수는 `"engine::animation_state_machine"` — 그리고 이 문서를 그대로 따라 쓴 `after()`는 §3대로 **무경고로 폐기**된다. 두 결함이 정확히 맞물리는 지점 |
| `PATTERNS.md`의 *"`ViewportSize`는 매 프레임 갱신되니 다음 틱에 self-heal"* | `docs/PATTERNS.md` (v0.139.1 감사 표) | `render`에는 거짓 (§3 마지막 항목) |

---

## 9. 반증되어 버려진 것 (3건)

검증 에이전트가 코드를 열어 기각했다. 기록해 두는 이유는 **같은 오탐이 다음 분석에서 또 나오기
때문**이다.

- `src/ui/panel.rs:214` — "Panel을 숨기면 배경만 숨겨지고 자식은 계속 렌더/클릭됨" → 기각
- `src/ui/system/capture.rs:53` — "`ProgressBar`가 `PointerCapture::rebuild`에서 누락" → 기각
- `src/input/map.rs:242` — "게임패드 축 바인딩이 `just_pressed_with_gamepad`를 영원히 레벨 트리거"
  → 기각. **이건 2026-06-16 분석에서 실제로 있었고 이미 고쳐진 버그**다. 오늘 코드는 정상

여기에 §2에서 작성자가 직접 기각한 흰색-스프라이트 후보 1건을 더한다.

---

## 10. 미검증 후보 — 세션 한도로 검증이 유실된 23건

> **이 절의 항목들은 finder가 올렸을 뿐 적대적 검증을 통과하지 않았다.** 위 §1~§8과 같은 신뢰도로
> 취급하면 안 된다. 이번 실행에서 검증 에이전트 33개가 `session limit`으로 사망했고, 그 피해가
> **교차 렌즈 5개(perf, wasm-drift, fork-api, test-gaps, docs-drift)에 집중**돼 그 렌즈들은
> 확정 0건으로 집계됐다 — 찾은 게 없어서가 아니라 검증이 못 돌아서다. HIGH 5건은 §2에서 직접
> 처리했고, 아래는 남은 MEDIUM/LOW다. 후속 세션에서 검증할 가치가 있다.

**fork-api / fail-quiet**
- `src/ron_registry.rs:11` — `RonRegistry`가 "built-in 레지스트리와 같은 canonical-path 핫리로드"를
  광고하지만 아무도 경로를 파일 워처에 등록하지 않음
- `src/serde_registry.rs:94` — 중복 컴포넌트 이름을 조용히 덮어씀. 문서는 유일해야 한다고 선언했고,
  엔진이 이미 `"Panel"` 등을 점유 중
- `src/asset/async_loading.rs:152` — 실제 wasm 이미지 fetch 실패(404/디코드 오류)가
  `asset_failures()`에도 strict 모드에도 **도달하지 않음**

**native/wasm 드리프트**
- `src/audio_wasm.rs:666` — `Audio::is_channel_playing`이 웹에서 music/positional 채널에 대해 항상 false
- `src/app/window.rs:505` — 터치 입력이 마우스 경로가 적용하는 레터박스 매핑을 우회 →
  `DesignResolution`에서 **엉뚱한 좌표**
- `src/input/gamepad.rs:87` — `GamepadState`가 wasm에서 영구 무응답인데 타입에 플랫폼 주석 없음
- `src/network/wasm_impl.rs:172` — 소켓 open 전 send가 native는 큐잉, wasm은 **드롭**;
  `max_pending_messages`가 wasm에서 무시됨
- `src/network.rs:98` — `push_event_bounded`의 wasm 복사본이 손으로 미러링됐는데
  `mod tests`가 non-wasm으로 cfg 게이트 → **오버플로 정책 커버리지 0**

**성능** (§6과 같은 계열, 미검증)
- `src/particle/mod.rs:238` — `ParticleSystem`이 매 프레임 전체 파티클 `Vec` 수집 후
  파티클당 `get_mut` 3회 (`query3_mut` 대신)
- `src/collision/grid.rs:90` — `SpatialGrid::rebuild`가 매 프레임 모든 버킷 `Vec` 재할당,
  `query_radius`/`query_aabb` 호출마다 2개 더
- `src/ui/localized.rs:81` — `LocalizationSystem`이 locale dirty 체크 없이 매 프레임 전체 재번역/재할당

**검증 사각지대**
- `src/renderer/gpu_particle.rs:432` — **GPU 파티클 렌더러를 실행하는 자동 검사가 하나도 없다.**
  유일한 테스트는 구조체 크기 assert. §1의 HIGH 2건이 여기 있었던 이유를 정확히 설명한다
- `.github/workflows/ci.yml:281` — `wasm_smoke.sh`는 이미 자기 판정 assert를 갖고 있어,
  CI에서 빼둔 문서상 근거가 사실이 아님
- `src/renderer/texture.rs:280` — 공개 `decode_image_bytes` 헬퍼의 비-에러 테스트 2개가 공허함
  (하나는 엔진 코드를 호출조차 안 함, 다른 하나는 아무 결과나 수용)

**문서 드리프트**
- `.github/workflows/ci.yml:7` — `wasm-smokes` 잡이 **required check가 아님** →
  세 문서의 "5개 브라우저 스모크가 게이트한다"는 주장이 강제되지 않음 *(주: §0에서 확인했듯
  5개가 그 잡에서 도는 것은 사실이다. 쟁점은 required 여부다)*
- `scripts/selftests.sh:74` — "9 selftests / 14 build targets"가 3곳에서 낡음 (실제 11 / 16).
  작성자 측정으로도 SELFTEST 이름은 **11개**가 맞다(게임 10 + `audio_reactive`)
- `src/network/system.rs:50` — 경고문과 등록 문서가 `app.world.register_event::<NetworkEvent>()`를
  안내하는데 **그런 메서드가 없음**
- `docs/NEXT_WORK.md:80` — "일곱 개 디렉터리 예제가 `cargo package`에서 빠진다" → 실제 여덟 개
  (`audio_reactive` 누락)
- `docs/PATTERNS.md:315` — "나머지 20개는 씬 상태" vs 자기 표는 22개 (27 − 5 = 22)
- `docs/MODULE_MAP.md:49` — 예제를 `dig_quest`/`tile_paint`로 부르지만 cargo 타깃명은
  `dig_quest_game`/`tile_paint_game`
- `.github/workflows/ci.yml:176` — 세 native 렌더 스모크의 헤더가 "CI는 렌더 못 한다"고 말한다는
  주석이 낡음 (헤더는 이미 반대로 수정됨)

---

## 11. 이 분석이 확인하지 못한 것

경계를 분명히 해 둔다.

- **빌드를 한 번도 돌리지 않았다.** 병렬 에이전트가 `target/` 락에서 교착하므로
  `cargo build/check/test`를 금지했다. 유일한 예외가 사전에 돌린
  `cargo clippy --all-targets`(exit 0). 따라서 제안된 수정은 **컴파일 검증되지 않았다.**
- **GPU 없음** — `render` 잡(lavapipe), `SKELETON_REQUIRE_GPU=1` 스모크 미실행.
  §1의 파티클/도킹 결함은 코드 판독으로 확정했지 화면으로 확인하지 않았다.
- **실기기 오디오 없음, wasm 런타임 없음, macOS/Windows 런타임 없음.**
  §2의 macOS 게임패드 건은 코드와 호출 빈도로 확정했으나 **실제 패드로 재현하지 않았다.**
- **`cargo build --release` 미실행** — `lto = "thin"` 링크 여부는 여전히 미확인.
- 교차 렌즈 5개의 후보는 §10대로 **적대적 검증을 받지 못했다.**

### 후속 실행 방법

이번 워크플로 스크립트와 각 에이전트의 원본 반환값이 남아 있어, 죽은 33개만 재실행할 수 있다:

```
Workflow({ scriptPath: ".../skeleton-engine-full-analysis-wf_bbc231d5-635.js",
           resumeFromRunId: "wf_bbc231d5-635" })
```

변경되지 않은 에이전트는 캐시에서 즉시 반환되고, 사망한 것만 실제로 다시 돈다.

---

## 12. 실행 순서 — 우선순위대로

정렬 기준은 **(막을 수 있는 고통 × 도달 가능성) ÷ 노력**, 그리고 **같은 파일은 같은 PR로 묶어
충돌을 피한다**. 버전은 저장소 규약(pre-1.0: 버그픽스 = PATCH, 공개 API 추가 = MINOR)을 따랐다.

### 순서 요약

| # | 작업 | 심각도 | 노력 | 버전 |
|---|---|---|---|---|
| **0** | 미검증 23건 검증 재개 *(선행·병렬)* | — | 무 | — |
| **1** | GPU 파티클 2건 + **실행되는** 렌더 테스트 | HIGH | S | PATCH |
| **2** | 도킹 에디터 포스트 체인 | HIGH | M | PATCH |
| **3** | macOS 게임패드 입력 유실 | HIGH | S | PATCH |
| **4** | 데이터 소실 3건 (세이브·씬저장·계층) | MED | S | PATCH |
| **5** | fail-quiet 진단 3건 | MED | S | PATCH |
| **6** | 평범한 입력으로 도달하는 panic/hang 5건 | MED | S | PATCH |
| **7** | 씬 리셋 계약 (설계 판단 필요) | MED | M | MINOR |
| **8** | 물리 / 충돌 정확성 3건 | MED | M | MINOR |
| **9** | 렌더 / 텍스트 정확성 3건 | MED | M | PATCH |
| **10** | 오디오 native↔wasm 패리티 4건 *(실기기 필요)* | MED | M | PATCH |
| **11** | 애니메이션 4건 | MED | S | MINOR |
| **12** | 프레임당 할당 8건 | MED~LOW | M | PATCH |
| **13** | 문서 드리프트 + 에디터 폴리시 + 나머지 LOW | LOW | S | 없음 |

---

### 0. 미검증 23건 검증 재개 — 선행, 다른 작업과 병렬

§10의 23건은 아직 적대적 검증을 못 받았다. **1~3번은 이미 검증됐으니 기다릴 필요 없이 착수하되**,
백그라운드로 이것부터 돌려 두면 4번 이후의 목록이 바뀔 수 있다(승격·기각). 비용은 사실상 0이다.

```
Workflow({ scriptPath: ".../skeleton-engine-full-analysis-wf_bbc231d5-635.js",
           resumeFromRunId: "wf_bbc231d5-635" })
```

---

### 1. GPU 파티클 2건 + 실행되는 렌더 테스트 — `fix(render):`

| 항목 | 위치 | 수정 |
|---|---|---|
| 링 커서가 매 프레임 0으로 리셋 | `src/app/render/frame.rs:458` | `GpuParticleRenderer`에 `frame_cursor: u32` 필드 → `&mut`로 전달 |
| 포맷 파이프라인이 `if has_emitters` 안에서만 빌드 | `src/app/render/frame.rs:452` | `ensure_render_pipeline`을 `if let Some(gpr)` 블록 안(`as_mut()`)으로 이동 |

**왜 1번인가.** 확정된 최고 심각도인데 diff는 각각 사실상 한 줄이다. 그리고 §10이 이유를 설명한다 —
**GPU 파티클 렌더러를 실행하는 자동 검사가 하나도 없다**(유일한 테스트가 구조체 크기 assert).
그래서 고치는 김에 검증 수단을 같이 넣는다: `tests/render.rs`에 `SKELETON_REQUIRE_GPU=1` 아래
에미터를 실제로 돌려 **2프레임 이상 파티클이 누적되는지**(커서 버그를 잡는 조건) 확인하는 테스트.
테스트 없이 고치면 다음에 똑같이 재발한다.

> ⚠️ 이 저장소의 `verify.sh`는 `render` 잡을 커버하지 않는다. GPU 있는 머신에서 직접 돌리거나
> PR의 CI `render` 잡을 게이트로 삼을 것.

### 2. 도킹 에디터 포스트 체인 — `fix(render):`

`src/app/render/frame.rs:162` (`use_post` 게이트) + `:529` (포스트/라이팅 스킵 + 1회 warn).

**왜 2번인가.** 1번과 **같은 파일**이라 붙여서 해야 충돌이 없다. HDR이 켜져 있으면 wgpu 포맷
검증 에러까지 나므로 실질 심각도는 1번과 동급. 검증은 이미 있는
`App::screenshot_editor_docked_headless`로 화면을 찍어 확인 — 새 도구가 필요 없다.

### 3. macOS 게임패드 입력 유실 — `fix(input):`

`src/input/gamepad.rs:209` — `slot.just_pressed = just_pressed` → **누적**으로.
gilrs 경로(`:151`)가 이미 `insert`로 누적하고 있으니 그 계약에 맞추면 된다.

**왜 3번인가.** 실제로 입력이 사라지는 HIGH이고 수정은 두 줄이다. 더 중요한 건
**지금은 유닛 테스트가 가능하다**는 점 — `apply_macos_snapshot`은 `GamepadState`에 대한 순수
함수라, *flush 없이 두 번 연속 호출*했을 때 눌림 엣지가 살아남는지 패드 없이 검증할 수 있다.
CI가 게임패드를 못 본다는 이유로 영원히 안 잡히던 종류를 여기서 닫는다.

### 4. 데이터 소실 3건 — `fix(save|editor):`

| 항목 | 위치 | 수정 |
|---|---|---|
| 비원자적 세이브 쓰기 | `src/save.rs:137` | temp write → `fs::rename` |
| `Hidden`/`RenderLayer`/`SpriteFlip`/`YSort` serde 미등록 | `src/app/core_resources.rs:103` | 기존 블록에 4줄 |
| `spawn_entity_def`가 `EntityDef.parent` 무시 | `src/prefab.rs:294` | `hierarchy::attach`, 없으면 warn |

**왜 4번인가.** 셋 다 **사용자의 데이터를 조용히 파괴**한다(세이브 파일 / 씬 파일 / 계층). 크래시보다
나쁜 부류다 — 잃고 나서야 안다. 셋 다 `small`이고 서로 독립이라 한 PR로 묶기 좋다.

### 5. fail-quiet 진단 3건 — `feat(ecs|ui):`

| 항목 | 위치 |
|---|---|
| 매칭 안 되는 `after()`/`before()` label에 warn | `src/ecs/schedule.rs:77` |
| `Events<UiEvent>` 미등록 warn-once | `src/ui/system/state.rs:181` |
| `"engine::state_machine"` → `"engine::animation_state_machine"` | `src/animation/mod.rs:17` |

**왜 5번인가.** fork 사용자 1인시간당 가치가 가장 높은 구간이다. 그리고 **뒤의 두 개가 정확히 맞물린다** —
문서가 틀린 라벨을 안내하고(`animation/mod.rs:17`), 그 라벨로 쓴 `.after()`는 무경고로 폐기된다.
첫 항목을 먼저 넣으면 **세 번째 같은 실수가 스스로 신고**하게 되므로 이 순서로 묶는다.

### 6. 평범한 입력으로 도달하는 panic / hang 5건 — `fix(*):`

`rich_text.rs:103`(비-ASCII 색상 panic) · `particle/mod.rs:360`(음수 spread panic) ·
`particle/mod.rs:344`(`INFINITY` → 프레임 영구 정지) · `ui/text_input.rs:227`(cursor panic) ·
`pool.rs:63`(이중 반납 → 같은 `Entity` 두 번 배포).

전부 `small`이고, 전부 *게임 개발자가 평범하게 쓰다가* 밟는다. 한 PR로 묶어도 되고 쪼개도 된다.

### 7. 씬 리셋 계약 — `feat(app):` **설계 판단 필요**

`src/app/core_resources.rs:37` (`AssetServer`/`ScriptRegistry`가 `Replace`에 날아감) +
`src/app/scenes.rs:62` (에디터 undo 히스토리 미정리 → 낡은 핸들이 새 엔티티를 가리킴).

**왜 여기인가.** 앞의 것들과 달리 **기계적이지 않다.** CLAUDE.md 자신의 규칙("캐시는 persist")은
persist를 가리키지만, 씬마다 에셋을 해제하고 싶은 설계도 정당하다. 어느 쪽을 고르든
**지금의 조용한 실패는 틀렸다** — 최소한 오래된 `Handle`이 죽었다는 진단은 나와야 한다.
결정하고 `PATTERNS.md`의 v0.139.1 감사 표를 갱신할 것.

### 8. 물리 / 충돌 정확성 3건 — `feat(physics):`

`physics/body.rs:5`(despawn 시 rapier 유령 — `despawn_with_body` 헬퍼 = 공개 API 추가 → MINOR) ·
`collision/grid.rs:101`(`GlobalTransform` 무시 → 부모 붙은 히트박스가 엉뚱한 곳에서 충돌) ·
`physics/system.rs:279`(회전잠금 바디의 rotation을 0으로 덮음).

### 9. 렌더 / 텍스트 정확성 3건 — `fix(render):`

`lighting.rs:428`(`DesignResolution`에서 점광이 타원) · `text/renderer.rs:533`(게임 폰트가
폴백으로만 쓰임) · `render_plugin.rs:53`(레터박스 투영 공식 문서화 — 비파괴 1단계).

### 10. 오디오 native↔wasm 패리티 4건 — `fix(audio):` **실기기 세션 필요**

`playback.rs:586`(크로스페이드가 최대 음량으로 점프) · `playback.rs:203`(이펙트 걸면 de-click
엔벨로프 소실) · `positional.rs:87`(native가 팬을 안 움직임) · `audio_wasm.rs:926`(웹에서
`levels()`가 영원히 무음).

**왜 여기로 미루나.** 저장소 문서대로 **native 오디오는 CI가 검증할 수 없다.** 귀로 확인할 수 있는
한 세션에 몰아서 하는 게 효율적이다. `SKELETON_MUTE=1`은 측정을 안 바꾸니 개발 중엔 켜 두고,
마지막 확인만 스피커로.

### 11. 애니메이션 4건 — `feat(animation):`

`player.rs:74`(셀프 전이 no-op → `restart()` 추가 = MINOR) · `state_machine.rs:301`(에디터
상태 점프 후 클립 desync) · `state_machine.rs:414`(`?` → `continue`, 한 글자) ·
`skeletal.rs:222`(음수 speed clamp).

### 12. 프레임당 할당 8건 — `perf(*):`

**`src/hierarchy.rs:234`를 먼저.** `Parent`가 하나도 없는 게임까지 포함해 **모든 게임이 내는**
built-in 비용이다. 그다음 `dialogue/system.rs:39` → `tilemap/system.rs:153` →
`anim_effect.rs:165` / `zone_effect.rs:184`(둘 다 순수 순서 바꾸기, 거의 공짜) →
`ui/panel.rs:160` → `app/assets.rs:262` → `app/render/debug_draw.rs:34`.

### 13. 문서 드리프트 + 에디터 폴리시 + 나머지 LOW — 버전 범프 없음

§8의 3건(`autotile.rs:110`, `animation/mod.rs:17`은 5번에서 처리됨, `PATTERNS.md`) +
§10의 문서 드리프트 6건(검증 후) + 에디터 다중선택 undo 그룹화(`shortcuts.rs:148`) +
`behavior.rs:268` `Selector` 래치 + 나머지.

**여기에 CI 공짜 한 건을 끼워 넣을 것** *(§10, 미검증)*: `wasm_smoke.sh`는 이미 자기 판정
assert를 갖고 있는데 CI에서 빠져 있다. 사실이면 한 줄로 브라우저 커버리지가 늘어난다.
`wasm-smokes` 잡이 required check인지도 같이 확인.

---

### 묶지 말아야 할 것

- **1번과 2번은 같은 파일이지만 별개 PR로.** 하나는 파티클, 하나는 에디터 합성이라 회귀 시
  이등분이 어려워진다. 순서만 붙이고 커밋은 나눈다.
- **12번(perf)을 앞 항목에 섞지 말 것.** 성능 변경은 동작 변경과 섞이면 벤치가 무의미해진다.
- **7번을 4번에 묶지 말 것.** 4번은 전부 자명한 버그, 7번은 설계 결정이다.
