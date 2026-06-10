# skeleton-engine 전체 코드베이스 분석 보고서

> 분석일: 2026-06-10 | 대상: `src/` 약 27,000 라인 | 패키지 `skeleton-engine` v4.5.0 (crate `engine`)
> 방법: 10개 모듈 그룹 분석 + 그룹별 적대적 검증(verifier) + 3개 횡단 리뷰(api-consistency / coupling / system-assembly)
> 평가 기준: "fork 가능한 MIT skeleton 2D 엔진"이라는 `docs/VISION.md`의 목표. AAA 엔진 기준이 아님.

> **처리 현황 (2026-06-10, v4.6.0 non-breaking 배치):** Top 10 중 **#1, #3, #4, #5, #6, #7, #10 완료**,
> **#9 부분 완료**(WGSL clone 제거; per-sprite String 키는 API 변경 필요 → v5).
> **#2(rapier 핸들 newtype)와 #8(`on_enter` SystemRegistrar)은 breaking이라 v5.0.0 배치로 이월** —
> deprecated `DebugDrawQueue`/`DebugRect` 제거, path shim 제거, `Sprite.texture` 타입 교체도 같은 배치.
> 아래 본문은 분석 시점 스냅샷 그대로 보존. 정확한 커밋은 git log(`fix/analysis-top10` 브랜치) 참조.
>
> **잔여 정리 스윕 (2026-06-11, 같은 브랜치):** §7 부록의 non-Top-10 항목 중 **~30건 처리** (per-frame
> 할당 정리, 중복 코드 추출, vestigial API deprecate, 문서화 항목, LABEL 상수 전 시스템 추가 + PATTERNS.md
> ordering 섹션 + platformer 예제 시연). **남긴 것**: pub 가시성 축소류·`SystemConfig`/`SystemMeta` 통합·
> `ShaderMaterial` source_hash 캐싱(pub 필드 리터럴 생성이라 breaking)·`TouchState` 접근자 → v5 배치;
> StateMachine crossfade·scripting Arrive/Wander 바인딩·AudioEffect release 구현 → 기능 작업으로 분리;
> World↔Reflect 분리(설계 변경 규모)·DefaultHasher 안정성(인메모리 캐시라 무관) → 보류/기각.
> 상세는 `docs/CHANGELOG.md` 4.6.0 항목 참조.

---

## 1. 종합 평가

skeleton-engine는 "fork 가능한 skeleton"이라는 목표에 비추어 **건강한 상태**다. 모듈 그래프는 사이클 없는 DAG이고, `ecs`와 `color`가 진짜 기반 레이어를 이루며, 가장 큰 서브시스템들(physics, audio, network)은 fork가 나머지를 건드리지 않고 통째로 떼어낼 수 있는 opt-in 슬랩으로 깔끔하게 분리돼 있다. ECS 코어(World, archetype, query, Commands, events, schedule)는 경계가 분명하고 borrow-workaround 패턴이 일관되게 적용돼 있으며, animation 파이프라인(`AnimationPlayer → AnimationSystem → UvRect/BlendUv`)과 input-camera 그룹은 캡슐화가 모범적이다(내부 변형은 `pub(crate)`, gilrs/winit/rapier 타입이 공개 표면으로 새지 않음). network 모듈은 이 그룹에서 가장 정교한 부분으로, 경계 큐와 snapshot 보간, remote-entity 생명주기를 갖춘 dual-platform WebSocket 클라이언트다. 약점은 시스템 전반의 결함이 아니라 **세 가지 반복되는 주제**에 집중돼 있다: (1) rapier2d 핸들 타입과 `Lerp`/`UvRect` 같은 타입의 잘못된 홈(home)으로 인한 레이어 누수, (2) prefab/scene 파일이 암호화 save API를 거쳐 사람이 읽을 수 없게 되는 hackability 위반, (3) editor 상태가 런타임 `App` 구조체에 무조건 섞여 들어가 fork 시 떼어내기 어려운 점. 이들은 모두 국소적이고 명확한 수정 경로가 있다.

---

## 2. Top 10 우선순위 권고

> 영향도 순. 심각도는 검증 후 조정된 값이며, downgraded 항목은 verifier 노트를 반영해 톤을 낮췄다.

### 1. Prefab/scene 파일이 AEAD 암호화되어 사람이 편집 불가 — **high**
- 근거: `src/prefab.rs:159,172-178,240,245`, `src/save.rs:90-92`
- 문제: `SceneDef::load/save`와 `Prefab::load/save`가 암호화 `save/load`로 직결돼, 디스크에 쓰인 모든 scene/prefab가 `SAVE_MAGIC + nonce + ciphertext` 바이너리다. 레벨 디자이너가 `.ron` 레벨 파일을 텍스트 에디터로 열 수 없다. 이는 skeleton 엔진의 최우선 가치인 hackability를 정면으로 위반한다. 암호화는 플레이어 세이브(스코어, 설정)엔 맞지만 설계 시점 자산엔 틀린 선택이다.
- 권고: `src/save.rs`에 암호화 없는 `write_ron`/`read_ron` 자유 함수를 추가하거나 `prefab.rs`에서 `ron::ser/de`를 직접 호출. `SceneDef`/`Prefab`의 save/load는 이 비암호 경로를 쓰고, 암호화 쌍은 실제 플레이어 세이브 전용으로 남긴다.

### 2. rapier2d 핸들 타입이 공개 API로 누수 — **high**
- 근거: `src/physics/body.rs:6-7`, `src/physics/world/body_factory.rs:16,34,...`, `src/physics/world/raycast.rs:18`, `src/physics/world/joints.rs:12-44`, `examples/crane_wrecking_ball.rs:28,230`
- 문제: 모든 팩토리 메서드가 `(RigidBodyHandle, ColliderHandle)`를 반환하고 `PhysicsBody`가 둘을 `pub` 필드로 노출하며 `cast_ray`는 `Option<(ColliderHandle, f32)>`, joint 메서드는 `RigidBodyHandle`을 받는다. 핸들을 저장하거나 joint를 호출하는 게임 코드는 모두 `rapier2d`에 직접 의존하게 된다(`crane_wrecking_ball.rs`가 실제로 그렇다). PATTERNS.md의 캡슐화 의도("rapier2d 필드에 외부에서 직접 접근하지 말 것")를 위반하며 물리 백엔드 교체를 사실상 불가능하게 만든다. `JointHandle`/`CollisionGroups`는 이미 올바른 newtype 래핑을 보여주므로 패턴은 존재하나 일관 적용되지 않았다.
- 권고: `JointHandle` 패턴을 따라 `BodyHandle(pub(crate) RigidBodyHandle)`/`ColliderHandle` newtype을 도입해 팩토리 반환·`PhysicsBody` 필드·`move_character`·`cast_ray`·joint 메서드에 전파. raw `rigid_body()`/`get_collider()` 접근자는 escape hatch로 문서화해 남긴다.

### 3. `UvRect`/`BlendUv`가 animation에 정의돼 엔진 전역에서 사용 — **high** (coupling 렌즈)
- 근거: `src/animation/player.rs:8`, `src/atlas.rs:1`, `src/tilemap.rs:5`, `src/renderer/sprite.rs:10`, `src/renderer/ui.rs:1`, `src/components.rs:309`
- 문제: `UvRect`/`BlendUv`는 의미상 animation과 무관한 순수 GPU UV 좌표 타입인데 `animation/player.rs`에 산다. 그 결과 `atlas`/`tilemap`/`renderer/sprite`/`renderer/ui`/`components`가 모두 animation 모듈에 컴파일타임 의존한다. animation을 제거·교체하려는 fork는 이 fan-in부터 풀어야 한다. (모듈별 렌더러 finding에서는 lib.rs 재노출로 인해 이동이 semver-breaking이라 downgrade됐으나, coupling 렌즈가 집계한 영향 범위는 6개 모듈로 더 넓다.)
- 권고: `src/renderer/uv.rs`(또는 `renderer/mod.rs`)로 옮기고 `renderer`에서 재노출, 6개 import 사이트 갱신. `animation::player`가 renderer에서 import하는 올바른 방향(animation이 renderer를 구동)으로 뒤집는다. semver 영향은 다음 breaking 배치에서 처리.

### 4. lighting hot path에서 프레임마다 bind group 할당 — **high**
- 근거: `src/renderer/lighting.rs:430-451`
- 문제: `LightingRenderer::run_pass`가 매 프레임 `device.create_bind_group(...)`을 호출한다. 주석은 "scene texture와 normal buffer가 매 프레임 바뀔 수 있다"고 정당화하지만, 실제로 normal_view는 `LightingRenderer` 소유이고 scene_view도 resize 때만 바뀐다. 매 프레임 GPU bind group 할당은 힙 할당 + 드라이버 작업을 submit 경로에서 유발하는 실제 비용이며, 라이팅 패스는 매 프레임 돈다.
- 권고: bind group을 `LightingRenderer`에 캐싱하고 `resize` 때만 무효화. `scene_view`를 `set_scene_view`로 받아 bind group을 재구성하고 `run_pass`는 캐시된 것을 사용.

### 5. editor 상태가 런타임 `App` 구조체에 무조건 섞임 — **medium**
- 근거: `src/app.rs:97-217,235-326` / coupling·system-assembly 렌즈에서도 독립 지적
- 문제: gizmo·clipboard·undo history·component factory map·snap 설정 등 순수 editor 관심사 필드 약 15~17개가 `#[cfg(not(wasm))]` 가드로 `App`에 직접 산다. `App`은 약 59개 필드로 커졌고 그중 ~17개가 editor용이다. editor를 떼거나 교체하려는 fork는 두 개의 별도 struct literal, 타입 정의, `editor/`의 모든 메서드에서 필드를 외과적으로 제거해야 한다. 깔끔한 추출 경계가 없어 VISION.md의 "명확한 모듈 경계" 목표와 충돌한다.
- 권고: editor 필드를 단일 `EditorState` 구조체(자체가 `#[cfg(not(wasm))]`)로 묶어 `App`에 `editor: EditorState` 한 필드로 보유. editor UI 메서드는 이미 `src/app/editor/`에 있어 동작 분리는 끝났고 구조 분리만 맞추면 된다.

### 6. 이중 debug-draw API — 병합 미완 — **medium**
- 근거: `src/resources.rs:29-145`, `src/app/render.rs:419-454`
- 문제: `DebugDrawQueue`+`DebugRect`(구식, `CollisionDebugSystem`/editor gizmo 내부 사용)와 `DebugDraw`+`DebugShape`(신식, 풍부한 API)가 두 개의 별도 리소스로 둘 다 렌더 경로를 흐른다. `render.rs`가 2.5/2.6 단계에서 순차로 비운다. `DebugRect`엔 `DebugShape`에 있는 `Line`/`Circle`/`Cross` 변형이 없다. 새 debug primitive를 추가하려는 fork는 두 경로를 모두 이해해야 하고, 둘 다 `lib.rs:111`에서 재노출돼 구현 사고가 공개 API로 새어나갔다.
- 권고: `CollisionDebugSystem`(및 editor gizmo)을 `DebugShape::Rect`를 `DebugDraw`에 push하도록 이전. 외부 호출자가 없어지면 `DebugDrawQueue`/`DebugRect`를 공개 타입에서 제거하고 render.rs의 2.5 단계 drain을 삭제.

### 7. `Lerp` 트레이트가 timeline에 정의돼 network가 import — **medium**
- 근거: `src/network.rs:6`, `src/timeline.rs:21-23`
- 문제: `Lerp`는 일반 선형보간 수학 유틸인데 컷씬 모듈인 `timeline.rs`에 정의돼 있고, `network.rs`가 `SnapshotBuffer<T: Lerp>`를 위해 import한다. timeline은 transitively `tween::Easing`/`camera::Camera`/`components`에 의존하므로, timeline을 제거·교체하려는 fork는 SnapshotBuffer가 컴파일되게 하려고 이 의존을 풀어야 한다. network는 timeline보다 보편적으로 유용하므로 의존 방향이 거꾸로다.
- 권고: `Lerp`(및 f32/Vec2/Color impl)를 `src/tween.rs`(이미 `Easing` 소유) 또는 신규 `src/math.rs`로 옮기고 `lib.rs`에서 재노출. `timeline.rs`와 `network.rs` 모두 공통 위치에서 import. `Lerp`는 이미 lib.rs에서 재노출돼 API 파괴 없는 3파일 변경.

### 8. `Scene::on_enter`가 labeled system 순서를 표현 불가 — **medium** (system-assembly 렌즈, 원 보고 high에서 조정)
- 근거: `src/scene.rs:17-19`, `src/app/scenes.rs:77-80`, `src/ecs/schedule.rs:1-10`
- 문제: `on_enter` 시그니처가 `(&mut World, &mut Vec<Box<dyn System>>)`이라 여기서 push된 시스템은 항상 `SystemMeta::default()`(라벨·순서 제약 없음)를 받는다. 새로 도입된 토폴로지 스케줄러(`before/after`, `SystemLabel`)가 정작 게임 시스템의 주 등록 경로인 scene에서 구조적으로 접근 불가능하다. `StateMachineSystem::after(AnimationSystem)`를 scene 안에서 표현할 방법이 없다. 더불어 모든 예제가 삽입 순서에만 의존하고 LABEL 상수는 한 번도 시연되지 않아, fork 작성자가 순서를 잘못 바꾸면 조용히 틀린다.
- 권고: `on_enter`의 raw `Vec` 파라미터를 `push(system)`과 `push_labeled(system, config)`를 노출하는 얇은 `SystemRegistrar` 래퍼로 교체. 단순 경로는 단순하게 유지하면서 scene에서 순서 제약을 가능케 함. 별도로 canonical 예제 하나를 `add_system_labeled`로 전환해 시연.

### 9. sprite 렌더 경로에서 프레임마다 텍스처 키 String 할당 — **medium**
- 근거: `src/renderer/sprite.rs:341-343,402,468,495-496,573`
- 문제: 매 프레임 도는 `SpriteRenderer::render`에서 스프라이트마다 `h.path().to_string()` 또는 `sprite.texture.clone()`로 힙 할당된 텍스처 키 String을 만든다. N개 스프라이트 = 프레임당 N개 String 할당. 추가로 `ShaderMaterial`마다 `mat.frag_source.clone()`로 전체 WGSL 소스를 복제하고, line 573에서 같은 데이터를 두 번째로 clone한다.
- 권고: `Sprite`/`ShaderMaterial`의 텍스처 키를 `Arc<str>`/`Cow<'static, str>`로 저장해 프레임당 참조 카운트 복사로 전환. line 573 중복은 첫 수집 패스에서 `custom_pipelines.contains_key(&hash)`를 검사해 제거.

### 10. `is_connected`가 native `NetworkClient`엔 부재, WASM에만 존재 — **high** (api-consistency 렌즈)
- 근거: `src/network.rs:480`(wasm), `src/network.rs:108,267-296`(native)
- 문제: WASM `NetworkClient`는 `is_connected() -> bool`을 노출하지만 native `NetworkClient`엔 이 메서드가 아예 없다. `client.is_connected()`를 호출하는 `cfg`-비의존 게임 시스템은 wasm32에서만 컴파일되고 native에서 깨진다. fork 작성자가 cfg-agnostic 코드를 쓰다 컴파일 에러로 부딪히는 미문서화 플랫폼 분기다.
- 권고: native `NetworkClient`에 `is_connected()` 추가. 백그라운드 스레드가 Connected/Disconnected 이벤트에서 세팅하는 `AtomicBool`을 두거나 outbound 채널 상태를 조회.

---

## 3. 아키텍처

### 3.1 레이어 누수: 타입이 잘못된 모듈에 사는 문제 (반복 근본 원인)

이 그룹의 finding들은 모두 같은 뿌리를 공유한다 — **개념적으로 기반 레이어에 속하는 타입이 상위 기능 모듈에 정의돼 있어, 그 타입을 쓰는 모든 모듈이 상위 모듈에 컴파일타임 의존하게 됨.** 세 사례:

- `UvRect`/`BlendUv`가 `animation`에 살며 6개 모듈이 import (Top 10 #3)
- `Lerp`가 `timeline`에 살며 `network`가 import (Top 10 #7)
- rapier 핸들 타입이 wrapping 없이 `physics` 공개 표면으로 누수 (Top 10 #2)

세 경우 모두 fork가 한 서브시스템(animation / timeline / physics 백엔드)을 떼거나 교체하려 할 때 무관한 모듈을 먼저 풀어야 하는 비용을 만든다. 수정은 모두 저렴하다: 타입을 올바른 기반 위치(`renderer/uv.rs`, `math.rs`, newtype 래퍼)로 옮기고 의존 방향을 뒤집는 것. `JointHandle`/`CollisionGroups`가 이미 올바른 newtype 패턴을 보여주므로, 일관 적용만 하면 된다. 렌더러가 `animation::player`에서 `UvRect`/`BlendUv`를 import하는 것(`src/renderer/sprite.rs:10`, `src/renderer/ui.rs:1`)도 동일 뿌리이며, 런타임 동작 자체는 올바르나(렌더러는 ECS 컴포넌트로 읽음) 타입의 홈이 잘못된 게 문제다.

### 3.2 design-time 자산 vs runtime 세이브의 관심사 혼동

`prefab`/`scene`이 암호화 `save` API를 재사용하는 것(Top 10 #1)은 단일 finding이지만 hackability에 가장 직접적인 위반이다. 두 개의 서로 다른 관심사(설계 시점 편집 가능 자산 / 런타임 플레이어 세이브)가 하나의 암호화 경로로 합쳐져 있다. 분리는 자유 함수 두 개 추가로 끝난다.

### 3.3 런타임 `App`에 editor 도구가 구조적으로 얽힘

editor 상태 ~17개 필드가 `App`에 무조건 사는 문제(Top 10 #5)는 coupling·system-assembly 두 렌즈가 독립적으로 같은 결론에 도달했다. VISION.md가 editor 오버레이를 비-주요 도구로 명시하므로, 깔끔한 추출 경계의 부재는 실제 fork-친화성 갭이다. `EditorState` 추출 하나로 `App` 표면을 ~59 → ~44 필드로 줄이고, 흩어진 `#[cfg]` 주석을 한 곳으로 모으며, editor를 명확히 경계 지어진 제거 가능 서브시스템으로 만든다.

### 3.4 스케줄러는 도입됐으나 주 등록 경로에서 접근 불가

새 토폴로지 스케줄러는 진짜 개선이다(공식적 순서 제약, 회귀 테스트 존재). 그러나 두 가지가 그 가치를 묻어버린다: (a) `Scene::on_enter`가 labeled 순서를 표현할 수 없고(Top 10 #8), (b) 12개 내장 시스템 중 5개만 LABEL 상수를 가지며 `PhysicsSystem`/`CollisionGridSystem`/`NetworkSystem`/`ParticleSystem`/`AudioSystem`/`TilemapSystem`엔 없다(`src/physics/system.rs:38`, `src/collision/grid.rs:186-196` 등). `CollisionGridSystem`은 `SpatialGrid`를 읽는 시스템보다 먼저 돌아야 한다는 구체 제약이 있으나 fork가 `after(...)`로 참조할 LABEL이 없다. 같은 뿌리 — **스케줄러가 일급 시민으로 통합되지 않고 add_system 옆에 부가 기능으로 붙음.** 누락 LABEL 추가 + `on_enter` 래퍼가 함께 묶이는 작업이다.

### 3.5 ECS의 상향 의존 두 건

`World`가 `reflect` 서브시스템에 직접 결합(`src/ecs/world.rs:38-59,520-607`)되고 `HierarchySystem`이 `prefab::topological_sort_entities`를 매 프레임 호출(`src/hierarchy.rs:97`)한다. 둘 다 순수해야 할 ECS/런타임 레이어가 엔진 특화·직렬화 레이어를 향하는 상향 의존이다. reflect 결합은 opt-in이라 폭발 반경이 self-contained라서 downgrade됐지만, hierarchy→prefab은 더 실질적이다 — prefab 직렬화를 제거한 fork에서 hierarchy 전파가 조용히 깨진다. `topological_sort_entities`는 `world.get::<Parent>()`만 쓰므로 `hierarchy.rs`로 옮기면 된다.

### 3.6 작은 경계 누수들 (개별 low, 패턴은 일관)

- lighting 중간 텍스처가 `App`에 raw 튜플로 관리되며 텍스처 생성 로직 중복(`src/app.rs:128-143`, `src/app/render.rs:85-123`) — `IntermediateTexture` 헬퍼 또는 `LightingRenderer` 소유로 이전 권고.
- `GpuLightData`/`LightingUniforms`/`PostProcessRenderer::target_view`/`normal_view`가 불필요하게 `pub`(재노출 안 됨) — `pub(crate)`/`pub(super)`로 좁힐 것.
- `TouchState`가 event-buffer Vec을 `pub` 필드로 노출(`src/input/touch.rs`), input 4개 서브모듈이 모두 `pub`(후자는 실제 다운스트림 트래픽이 lib.rs 재노출로 흘러 downgrade).
- `components.rs:309-312`의 backward-compat 재노출 블록이 animation·resources 모듈에 결합 — migration 잔재.

---

## 4. 코드 품질 / 단순화

### 4.1 프레임당 할당: 반복되는 단일 패턴

품질 finding의 가장 큰 묶음은 **매 프레임 도는 시스템에서의 불필요한 힙 할당**이며, 거의 모두 같은 처방을 공유한다(소유권 take/swap, 버퍼 재사용, 또는 reference-count 타입).

- 텍스트 큐 전체 Vec clone(`src/renderer/text.rs:270-271`) — `std::mem::take`로 교체(같은 패턴이 `app/render.rs:459` 등에서 이미 사용 중인 불일치).
- sprite 텍스처 키 String 할당(Top 10 #9) 및 `ShaderMaterial::frag_source` 프레임당 clone+hash(`src/renderer/sprite.rs:466-468`) — `Arc<str>` / 캐시된 `source_hash`.
- `PhysicsSystem::run`의 프레임당 `col_map`/`current`/`current_intersections` HashSet+HashMap+Vec 할당(`src/physics/system.rs:76-161`) — 인접한 `CollisionGridSystem`은 clear-and-refill로 이미 올바르게 재사용하므로 일관성 불일치.
- `ParticleSystem`의 emitter 이중 스캔 + 2개 Vec 할당(`src/particle.rs:161-176,234-248`) — 단일 패스로 병합.
- `SteeringSystem`의 5회 entity-collect 패스(`src/steering.rs:113,141,172,209,240`), `LayoutSystem`의 Panel 이중 query(`src/ui/panel.rs:74-118`), `text_input_pass`의 글자마다 `get_mut`(`src/ui/system/text_input_pass.rs:92-117`).
- `App::update`의 프레임당 `exec_order` Vec clone(`src/app/schedule.rs:201`) 및 `update_editor_ui`의 무조건 HashMap/Vec 할당(`src/app/editor/ui/mod.rs:19-51`).

이들 대부분 단독으로는 low지만, **borrow-workaround 패턴이 "collect-then-mutate"를 강제할 때 재사용 가능한 scratch 버퍼 대신 매번 새로 할당하는 습관**이라는 단일 뿌리를 공유한다. skeleton의 entity 수가 적어 현재 비용은 작지만, 일관된 take/swap·필드 재사용 규약을 PATTERNS.md에 명시하면 fork가 헤비한 씬을 만들 때 함정을 피한다.

### 4.2 중복 코드 블록

- editor UI에 Tag name-editor 블록(`src/app/editor/ui/mod.rs:239-271, 612-643`)과 Ctrl+click multi-select 로직(`mod.rs:219-236, 402-429`)이 각각 두 번 — private 헬퍼로 추출.
- `PhysicsSystem::run`의 collision/trigger event-diff 블록(`src/physics/system.rs:75-155`)이 구조적으로 동일 — 제네릭 `diff_pairs` 헬퍼.
- `AssetServer::new()`가 같은 struct literal을 세 번 초기화하고 Err arm이 불필요한 두 번째 채널 쌍 할당(`src/asset.rs:200-243`).
- `App::new()`의 native/WASM 이중 struct literal(`src/app.rs:235-326`) — 비관용적 `#[cfg]` return 패턴 포함.
- `AnimationSystem`의 frame_dur 계산 중복(`src/animation/system.rs:41-83`), input bind 메서드 3개의 `or_insert_with` 중복(`src/input/map.rs:118-170`), fullscreen-quad 정점 셰이더가 `fade.rs`/`lighting.rs`에 중복.

### 4.3 vestigial / dead API (footgun)

- `register_reflect`는 빈 type_name을 저장 — 모든 production 호출이 `register_reflect_named`를 씀(`src/ecs/world.rs:526-535`). 호출 시 Inspector 표시가 조용히 깨지는 footgun.
- `SystemConfig`/`SystemMeta`가 구조적으로 동일 — 하나가 변환 후 잉여(`src/ecs/schedule.rs:6-62`). 둘 다 lib.rs:75 노출이라 통합은 semver-break.
- `AudioEffect::release_secs`가 public + 문서화됐으나 엔진이 절대 읽지 않음(`src/audio/types.rs:16`); `play_streaming`가 `#[allow(dead_code)]`로 연명(`src/audio/playback.rs:297-330`).
- `NetworkEvent::JsonParseError`가 엔진 어디서도 생성 안 됨(`src/network.rs:24`) — transport enum에 protocol-layer 관심사를 인코딩한 leaky abstraction.
- `ScriptCommands::spawned_ids`가 쓰이기만 하고 읽히지 않음(`src/scripting/context.rs:11`); `BbEntry`가 `bb_snap`에서 절대 안 읽는 중복 key String 보유(`src/scripting/context.rs:15-19`).
- `character_movement.rs`의 dead `shape_type` 바인딩(`:30-40`), `CharacterController::max_slope_angle` mirror 필드가 `inner` 상태 중복(desync 위험, `src/physics/character.rs:23`).

### 4.4 API 일관성 (api-consistency 렌즈)

공개 API는 이 단계 엔진치고 대체로 일관적이나, fork 작성자가 알아챌 마찰점들이 있다:

- **`Scene::on_enter`의 `Box::new` boilerplate**(medium): `App::add_system`은 unboxed 시스템을 받지만 scene 코드는 `systems.push(Box::new(NetworkSystem))`를 써야 함(coin_race/salvage_run/mp_client 예제에서 가시). VISION.md의 "예제에서 어색하면 API를 고쳐라" 규칙에 직접 걸린다 — Top 10 #8의 래퍼가 이것도 해결.
- **생성자 명명 불일치**: `ParticleEmitter::for_burst()`만 유일한 `for_*` 패턴(다른 모든 곳은 noun-adjective: `TileCollider::solid/one_way`, `Sprite::colored`) — `burst()`로 개명 권고.
- **중첩 생성자**: `load_texture` vs `load_image`(전자는 vestigial), `Sprite::colored`에 대응하는 `DrawImage::colored` 부재, query 변형 명명 불규칙(`query_opt2`는 있고 `query_opt3`는 없음).
- **tuple-struct 누수**: `ShouldQuit(pub bool)`을 모든 예제가 `q.0 = true`로 접근 — `quit()`/`is_quitting()` setter 추가로 구현 디테일 누수 제거.

이들은 대부분 low지만 한데 모이면 fork 작성자가 마주하는 표면 일관성을 갉아먹는다.

---

## 5. 모듈별 건강 요약

| 모듈 그룹 | 한 줄 평가 | 살아남은 finding (H/M/L) |
|---|---|---|
| renderer | 구조 견고; sprite 파이프라인 잘 분리, 핫패스 할당 2건이 주 이슈 | H1 / M3 / L5 |
| core-app | split 깔끔하나 editor 상태가 `App`에 무조건 섞이고 debug-draw 이중화 | H0 / M4 / L5 |
| ecs | 코어 깨끗·저결합; reflect/hierarchy 상향 의존이 주 관심사 | H0 / M3 / L5 |
| ui / locale | per-widget 분해 우수; LayoutSystem이 UiOutput 우회가 핵심 갭 | H0 / M1 / L4 |
| physics-collision | rapier 래퍼 잘 짜였으나 핸들 타입 누수가 가장 큰 문제 | H1 / M2 / L4 |
| input-camera | 엔진에서 가장 깨끗한 축; 캡슐화 모범, 소수 표면 이슈 | H0 / M2 / L5 |
| animation 외 | 엔진 최강 영역; `Lerp` 홈 오배치와 crossfade 갭이 주 이슈 | H0 / M3 / L4 |
| assets-audio | 의도적·잘 격리됨; 국소 중복 artifact와 확장성 갭 | H0 / M2 / L4 |
| net-save-prefab | network 정교; prefab 암호화가 hackability 직접 위반 | H1 / M1 / L4 |
| ai-scripting | 서브모듈 진짜 독립; thread-local 버퍼 재사용 영리, 소수 vestigial | H0 / M1 / L4 |

> 횡단 렌즈 topIssue 중 모듈 finding과 별개로 추가 집계된 것: api-consistency(`is_connected` 플랫폼 분기 high, `on_enter` boilerplate medium 등), system-assembly(`on_enter` 순서 표현 불가 high, LABEL 미시연/미보유 medium 2건).

---

## 6. 기각된 주장들 (검증이 실제로 일했다는 증거)

1. **ecs: "exec_order가 시스템 변형을 위해 매 프레임 불필요하게 clone된다"** → 기각. clone은 *필요*하다: 루프 본문의 `catch_unwind(AssertUnwindSafe(|| self.systems[i].run(&mut self.world, dt)))`가 `self`를 가변 캡처하므로 같은 스코프에서 `self.exec_order`를 공유 인덱스 읽기조차 충돌. 제안된 `for idx in 0..len()` 수정은 같은 이유로 컴파일 안 됨.
2. **ui: "LocalizationSystem이 위젯 타입마다 무조건 텍스트를 clone한다"** → 기각. `localized.rs:71,74`의 clone은 `if let Some` 블록 *안*이라 엔티티가 실제로 해당 컴포넌트를 가질 때만 실행. Label만 가진 엔티티는 정확히 한 번 clone — "검사 전 clone"이라는 특성화가 부정확.
3. **assets-audio: "TilemapAtlas가 TextureAtlas의 uv 로직을 asset/atlas 레이어 밖에서 중복한다"** → 기각. 두 타입 모두 이미 공유 `UvRect::from_grid`에 위임하므로 수학 중복 없음. 남은 차이(String path vs Handle, index wrapping)는 tilemap 모듈을 asset-server 결합에서 자유롭게 유지하려는 의도적 구조 선택.
4. **input-camera: "axis binding이 `just_pressed_with_gamepad`에서 조용히 누락 (high)"** → downgrade. 누락은 실재하나 `map.rs:228-232` doc-comment가 한계를 명시하고 우회법까지 기술 — 문서화된 설계 결정이지 silent trap이 아님. high 부적합.
5. **ai-scripting: "`Reflect::fields()`가 `is_enabled()` gate 전에 호출돼 오버레이 닫혀도 프레임당 할당 (medium)"** → downgrade. `comp_fields`는 gate가 닫힌 뒤 `mod.rs:746-757`에서 inspector edit write-back에 무조건 소비되므로 early-return 불가. `entity_list`/`tag_map`만 gate 안 전용 — 주장이 부분적으로만 옳고 핵심 인용 예시가 틀림.

> 추가로 검증이 다음을 downgrade했다: physics 핸들 누수 외의 reflect-결합·`query_added` 미사용 할당·`AnimationPlayer` pub 필드·`AssetServer` 확장성("fork가 곧 의도된 확장 경로")·pathfinding Tilemap 결합("`new`+`set_walkable` escape hatch 존재") — 모두 실재하나 skeleton 철학상 심각도가 과대평가됐다는 판정.

---

## 7. 부록: 전체 surviving finding 목록 (모듈별)

> 형식: 제목 — 심각도 | files | 권고 압축. (D)=downgraded.

### renderer
- Per-frame bind group allocation in lighting hot path — **high** | `lighting.rs:430-451` | bind group 캐싱, resize 때만 무효화.
- Renderer imports animation module types (D) — medium | `sprite.rs:10`, `ui.rs:1` | `UvRect`/`BlendUv`를 renderer/types로 이전(semver 배치).
- Intermediate lighting texture in App as raw tuple — medium | `app.rs:128-143`, `app/render.rs:85-123` | `IntermediateTexture` 헬퍼 추출 또는 LightingRenderer 소유.
- Per-frame clone of text queue items Vec — medium | `text.rs:270-271` | `std::mem::take`.
- Per-frame String allocation for sprite texture key — medium | `sprite.rs:341-573` | `Arc<str>`/`Cow`, line 573 중복 제거.
- GpuLightData/LightingUniforms unnecessarily public — low | `lighting.rs:14,32` | `pub(crate)`.
- PostProcessRenderer.target_view leaked as pub — low | `post_process.rs:86` | `pub(crate)`/`pub(super)`.
- DefaultHasher for shader cache key not stable — low | `sprite.rs:465-469` | FxHasher 또는 String-keyed map.
- Fullscreen vertex shader duplicated fade/lighting — low | `fade.rs:36-47`, `lighting.rs:72-86` | 공유 `fullscreen.wgsl` 추출.

### core-app
- Editor state mixed unconditionally into App — medium | `app.rs:97-217` | `EditorState` 구조체 추출.
- Dual debug-draw APIs incomplete merge — medium | `resources.rs:29-145`, `render.rs:419-454` | `DebugDraw`로 통합, 구식 제거.
- Duplicate App::new() struct literal — medium | `app.rs:235-326` | 공유 필드 헬퍼/Default.
- Per-frame exec_order Vec clone (D) — low | `schedule.rs:201` | take/swap (저비용, low로 조정).
- FadeTransition WASM no-op undocumented — low | `app.rs:124-125`, `resources.rs:461-530` | doc 주석 추가 또는 native-only 게이트.
- Duplicated Tag name-editor block — low | `editor/ui/mod.rs:239-271,612-643` | 헬퍼 추출.
- Duplicated Ctrl+click multi-select — low | `editor/ui/mod.rs:219-236,402-429` | 헬퍼 추출.
- unsafe transmute missing SAFETY doc — low | `egui_pass.rs:29-33` | SAFETY 주석 + dangling 참조 수정.
- Per-frame HashMap/Vec in update_editor_ui — low | `editor/ui/mod.rs:19-51` | `is_enabled` gate 안으로 이동.

### ecs
- World couples to Reflect subsystem (D) — medium | `world.rs:38-59,520-607` | ReflectRegistry 리소스로 분리(opt-in이라 조정).
- HierarchySystem depends on prefab::topological_sort_entities — medium | `hierarchy.rs:97`, `prefab.rs:339` | 함수를 hierarchy.rs로 이전.
- query_added/query_changed allocate, no production callers (D) — medium | `world.rs:638-663` | 클로저 인터페이스 또는 trade-off 문서화.
- SceneChange asymmetric visibility (D) — low | `scene.rs:53` | `take()` 접근자 또는 문서화.
- register_reflect dead API — low | `world.rs:526-535` | named 변형으로 통합/문서.
- despawn linear scan — low | `world.rs:179-181` | `entity_location` 활용, entities Vec 제거.
- move_entity clones two TypeId Vecs — low | `world.rs:773,795` | `split_at_mut` 또는 문서화.
- SystemConfig/SystemMeta identical — low | `schedule.rs:6-62` | 단일 타입 통합(semver).

### ui / locale
- LayoutSystem renders Panel background, bypassing UiOutput — medium | `panel.rs:116-140`, `system/state.rs:56-88` | 전용 panel_pass로 이전, doc 수정.
- LocalizationSystem split from LocaleResource (D) — low | `locale.rs`, `ui/localized.rs` | bridging doc 주석(문서 이슈로 조정).
- ViewportSize Copy-deref elided (D) — low | `system/state.rs:62-66`, `panel.rs:65-69` | `.copied()` (style nit).
- LayoutSystem queries Panel set twice — low | `panel.rs:74-118` | 단일 패스 병합.
- text_input_pass get_mut per character — low | `system/text_input_pass.rs:92-117` | 단일 get_mut hoist.

### physics-collision
- Rapier handle types bleed through public API — **high** | `body.rs:6-7`, `world/body_factory.rs`, `world/joints.rs` 외 | `BodyHandle`/`ColliderHandle` newtype.
- CollisionGroups vs CollisionLayer naming confusion — medium | `world.rs:31-84`, `grid.rs:39-49` | 모듈 doc 또는 `GridLayer` 개명.
- Duplicated event-diff block in PhysicsSystem::run — medium | `system.rs:75-155` | 제네릭 `diff_pairs`.
- CollisionDebugSystem ordering undocumented (D) — low | `collision/debug.rs:43-82` | 순서 경고 doc(문서 갭으로 조정).
- Dead shape_type binding — low | `character_movement.rs:30-40` | 제거.
- max_slope_angle mirror field — low | `character.rs:23,50,64-68` | computed getter.
- Per-frame HashMap+HashSet in event diffing — low | `system.rs:76-161` | 필드로 승격, clear+swap.

### input-camera
- Reflect::fields() allocates Vec per call (D) — medium | `components.rs:135-188`, `reflect.rs:49` | caller-buffer 시그니처(editor-only라 조정, semver).
- ShaderMaterial::frag_source cloned+hashed per frame — medium | `sprite.rs:466-468` | 캐시된 `source_hash`.
- Axis bindings absent from just_pressed_with_gamepad (D) — high→문서화된 한계 | `map.rs:226-247` | 문서화된 설계 결정(조정).
- Camera inline ecs::Entity path (D) — low | `camera.rs:48` | `use` import.
- TouchState raw event-buffer pub fields — low | `touch.rs:34-50` | 접근자 메서드.
- components.rs re-exports animation/resources — low | `components.rs:309-312` | 감사 후 제거 또는 facade 문서.
- input submodules all pub (D) — low | `input/mod.rs:1-4` | `pub(crate) mod`(실 트래픽 없어 조정).
- bind/or_insert_with three-way duplication — low | `map.rs:118-170` | `bindings_for` 헬퍼.

### animation 외 (timeline/particle/skeletal 포함)
- Lerp trait homed in timeline, imported by network — medium | `timeline.rs:21-23`, `network.rs:6` | `math.rs`/`tween.rs`로 이전.
- StateMachineSystem transitions cannot crossfade — medium | `state_machine.rs:38-43,269` | `crossfade_duration` 필드 추가.
- ParticleSystem double-scans emitters per frame — medium | `particle.rs:161-248` | 단일 패스 병합.
- GpuParticleEmitter driven from render.rs not System (D) — low | `gpu_particle.rs:75`, `render.rs:500` | CLAUDE.md 문서화(의도적이라 조정).
- Main-clip frame advance 'if' vs crossfade 'while' — low | `system.rs:47-92` | `while` 루프로 통일.
- AnimationPlayer exposes impl fields pub (D) — low | `player.rs:174-177` | `pub(crate)`+`seek_frame`(skeleton 철학상 조정).
- Duplicated frame_dur computation — low | `system.rs:41-83` | private 헬퍼.

### assets-audio
- AssetServer closed to user asset types (D) — medium | `asset.rs:149-166` | "two HashMaps" 패턴 문서화(fork가 의도된 경로라 조정).
- AssetServer::new() triple struct init — medium | `asset.rs:200-243` | 헬퍼/Default, Err arm 채널 재사용.
- AudioEffect::release_secs unread stub — low | `audio/types.rs:16,25` | 구현 또는 제거/`#[doc(hidden)]`.
- play_streaming dead code — low | `audio/playback.rs:297-330` | 제거.
- Effect-application tree duplicated (D) — low | `playback.rs:102-246` | 스타일 정렬(i16/f32 차이로 조정).

### net-save-prefab
- Prefab/scene files AEAD-encrypted — **high** | `prefab.rs:159-245`, `save.rs:90-92` | 비암호 `write_ron`/`read_ron`.
- SnapshotBuffer imports Lerp from timeline — medium | `network.rs:6`, `timeline.rs:21-23` | `math.rs`로 이전(3.1과 동일 뿌리).
- NetworkConfig not re-exported at root — low | `lib.rs:85`, `network.rs:29-45` | lib.rs 재노출 추가.
- JsonParseError emitted nowhere — low | `network.rs:24` | enum에서 제거, `Error(..)` 사용.
- SceneDef::save clones entire entity list (D) — low | `prefab.rs:172-178` | 직접 직렬화/field write(design-time이라 조정).
- Native send_text clones String — low | `network.rs:276-284` | `len` 선캡처(WASM도 동일).

### ai-scripting
- Reflect::fields() before is_enabled() gate (D) — medium | `editor/ui/mod.rs:25-80` | `entity_list`/`tag_map`만 lazy(comp_fields 불가, 부분 조정).
- pathfinding coupled to Tilemap (D) — low | `pathfinding.rs:6,72-84` | `from_tile_grid` 편의 생성자(escape hatch 존재로 조정).
- scripting hard-codes Seek/Flee only — low | `scripting.rs:87-91`, `scripting/context.rs:22-35` | Arrive/Wander 바인딩 또는 문서.
- BbEntry redundant key String — low | `scripting/context.rs:15-19` | `BbValue` 분리.
- SteeringSystem five collect passes (D) — low | `steering.rs:113-240` | scratch 버퍼(pub unit struct라 조정).
- spawned_ids written never consumed — low | `scripting/context.rs:11` | 제거 또는 `#[allow]`+주석.

### 횡단 렌즈 추가 (api-consistency / system-assembly)
- is_connected absent from native NetworkClient — **high** | `network.rs:480,267-296` | native에 추가.
- Scene::on_enter cannot express labeled ordering — high→medium 조정 | `scene.rs:17-19`, `app/scenes.rs:77-80` | `SystemRegistrar` 래퍼.
- Scene::on_enter Box::new boilerplate — medium | `scene.rs:19`, 예제들 | `push_system`/래퍼(위와 동일 수정).
- Examples use insertion-order only, LABEL never demonstrated — medium | platformer/blend_locomotion 예제 | canonical 예제 전환 + PATTERNS.md.
- Most engine systems lack LABEL constants — medium | `physics/system.rs:38`, `collision/grid.rs:186-196` 외 | LABEL 추가 + 순서 doc.
- ParticleEmitter::for_burst() naming — low | `particle.rs:79` | `burst()` 개명.
- load_texture/load_image overlapping — low | `app/assets.rs:22-33` | deprecate/개명.
- DrawImage lacks colored() — low | `renderer/ui.rs:6-31` | `colored()` 생성자.
- ShouldQuit .0 access leak — low | `resources.rs:238`, 예제들 | `quit()`/`is_quitting()`.
