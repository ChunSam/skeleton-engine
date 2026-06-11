# skeleton-engine 전체 코드베이스 분석 보고서

> 분석일: 2026-06-12 | 대상: `src/` 전체 모듈 | 패키지 `skeleton-engine` v5.1.1 (crate `engine`)  
> 검토 커밋: `9e2c3ace6070e018d756c0ce3b056370b83fd169` (Merge PR #15, fix/v5.1.1-review-fixes)  
> 방법: 7개 각도(정확성·불변식·교차모듈·재사용·단순화·효율·고도) 병렬 하위 에이전트 → 후보 통합 → 코드 재확인(CONFIRMED/PLAUSIBLE/REFUTED) → 합성  
> 제외: `docs/CODE_ANALYSIS_2026-06-10.md`에 이미 보고됐거나 v4.6.0/v5.0.0/v5.1.0/v5.1.1에서 수정된 항목은 재보고하지 않음.

---

## 1. 종합 평가

v5.1.1 기준 skeleton-engine은 직전 분석 라운드 대비 크게 개선됐다. 2026-06-10 Top-10 전체가 v4.6.0~v5.1.1 사이에 처리됐고, 아키텍처 레이어 누수(`UvRect`, `Lerp`, rapier 핸들)가 정리됐으며, 직렬화·씬 전환·오디오 envelope 등 핵심 버그도 수정됐다.

이번 라운드에서 새로 발견된 결함은 **세 가지 뿌리**로 수렴한다:

1. **네트워크 수신 큐 오버플로 처리 결함** — `dropped` 카운터가 누적되지 않고 오버플로 감지 시 정상 이벤트까지 삭제하는 두 개의 데이터 손실 버그 (이전 분석에 없음)
2. **animation crossfade 잔여 케이스** — v5.1.1에서 SM crossfade 진동·AnimationEnd 오탐은 수정됐지만, 새 클립으로 crossfade 중단 시의 visual pop과 완료 시 `to_timer` 버림이 남음
3. **프레임당 할당 패턴 잔존** — v4.6.0 sweep에서 상당수 정리됐으나 3개 animation 시스템·sprite 렌더러·particle·audio fade 루프에 잔존 또는 신규 per-frame 할당 확인

---

## 2. Top-10 우선순위 권고

> 영향도 순. (PLAUSIBLE) 표시는 재현 조건이 있는 추론 확인 항목.

### 1. `ReceiveQueueFull.dropped`가 항상 1, 누적 안 됨 — **high** (신규)
- 근거: `src/network.rs:62–68` (native), `src/network.rs:80–86` (WASM)
- 문제: `push_event_bounded`가 큐 가득 찼을 때 첫 번째만 `ReceiveQueueFull { dropped: 1 }` 이벤트를 발행하고, 이후 추가 오버플로에는 `!matches!(events.back(), Some(NetworkEvent::ReceiveQueueFull { .. }))` 조건이 false가 돼 아무것도 발행하지 않는다. `dropped` 필드는 누적 카운터가 아니라 상수 `1`이다. 서버가 한 프레임에 1030개를 보내면 게임 코드는 `dropped: 1`을 단 한 번 받지만 실제로는 30개가 소실됐다. WASM 경로도 동일한 결함.
- 권고: `events.back_mut()`으로 기존 `ReceiveQueueFull`의 `dropped`를 `+= 1` 갱신하거나, incoming 메시지를 early-return으로 버리고 큐에 이미 들어간 항목은 유지하는 방식으로 전환.

### 2. 오버플로 시 `pop_back()`이 마지막 정상 이벤트를 삭제 — **high** (신규)
- 근거: `src/network.rs:62–64`, WASM `src/network.rs:80–83`
- 문제: 큐가 용량에 도달하면 `pop_back()`으로 직전에 삽입한 정상 이벤트를 제거한 뒤 `ReceiveQueueFull`을 삽입한다. N번째 메시지로 큐가 꽉 찰 때 N번째 메시지 자체가 오버플로 마커로 교체되므로, 소비자는 마지막 정상 이벤트를 영구히 받지 못한다.
- 권고: #1과 통합 수정 — incoming 메시지를 버리는 early-return 방식으로 전환해 이미 큐에 들어간 이벤트에는 손대지 않음.

### 3. crossfade 도중 세 번째 클립으로 `play_with_crossfade` 시 visual pop — **medium** (신규)
- 근거: `src/animation/player.rs:76–97`
- 문제: `play_with_crossfade(clip_b, d)` 진행 중(blend_weight=0.7) 새 `play_with_crossfade(clip_c, d2)` 호출 시, 새 blend의 `from = current_clip`(= 원래 clip_a)으로 시작해 화면이 clip_a 자세(weight=0)로 팝백한다. v5.1.1에서 같은 clip_b 재호출에 대한 idempotency(동일 타깃 재진입 방어)는 수정됐지만, 다른 clip_c 요청 케이스는 커버하지 않는다.
- 권고: 새 `play_with_crossfade` 요청 시 현재 `blend_weight`에서 베이크한 프레임을 새 from 상태로 사용하거나, to 클립만 교체하고 blend 진행을 이어가는 방식.

### 4. crossfade 완료 시 `to_timer` 버림 → 저fps 애니메이션 스터터 — **medium** (신규)
- 근거: `src/animation/system.rs:76`
- 문제: crossfade 완료(`cf.elapsed >= cf.duration`)에서 `current_clip = cf.to_clip`, `current_frame = cf.to_frame`으로 승격하지만 `timer = 0.0`으로 리셋한다. `cf.to_timer`(블렌드 중 to 클립의 누적 프레임 진행 시간)가 버려진다. 5fps 클립(frame_dur=200ms)에서 to_timer=180ms가 이미 쌓였어도 timer=0부터 재시작해 첫 프레임이 가시적으로 더 길어진다.
- 권고: line 76을 `self.timer = cf.to_timer`로 교체.

### 5. lag spike 시 `drop_timer` 즉시 소멸 → one-way 플랫폼 낙하 실패 — **medium** (신규)
- 근거: `src/physics/world/character_movement.rs:51`
- 문제: `drop_timer -= dt; drop_timer = drop_timer.max(0.0)`이 `move_character` 호출 전에 실행된다. `dt > DROP_DURATION(0.2 s)`인 단일 프레임(GC pause, 렌더 스파이크)에서 drop window가 같은 프레임에 열리고 닫혀, 플레이어가 one-way 플랫폼을 통과하지 못한다.
- 권고: `dt.min(DROP_DURATION)` 클램프 적용 또는 `drop_timer` 감산을 `move_character` 실행 후로 이동.

### 6. `CollisionEvent`/`TriggerEvent` 미등록 시 이벤트 무음 드롭 — **medium** (신규)
- 근거: `src/physics/system.rs:156–164`
- 문제: `PhysicsSystem::run`이 `world.resource_mut::<Events<CollisionEvent>>()`에서 `None`을 받으면 `if let Some` 가드에 걸려 조용히 버린다. 사용자가 `app.register_event::<CollisionEvent>()`를 빠뜨리면 물리 충돌이 발생해도 이벤트를 영원히 받지 못하고 런타임 에러나 경고도 없다.
- 권고: `None` 분기에서 `log::warn!` 발행 또는 `PhysicsSystem` 등록 시 두 이벤트 타입을 자동 register_event.

### 7. `image_handle` 경로에서 sprite당 매 프레임 `Arc` 재할당 — **medium** (신규, 이전 #9 수정 잔여)
- 근거: `src/renderer/sprite.rs:341`, `src/renderer/sprite.rs:499`
- 문제: `Handle<T>`가 내부에 이미 `Arc<str>` path를 보유하지만 `Arc::from(h.path())`를 호출해 매 프레임 매 sprite마다 새 `Arc<str>` 블록을 heap-allocate한다. v5.0.0에서 `Sprite.texture`를 `Arc<str>`로 전환한 성과를 `image_handle` 경로가 되돌린다.
- 권고: `Handle<ImageAsset>`에 `fn path_arc(&self) -> Arc<str> { Arc::clone(&self.path) }` 추가, 두 호출 지점 교체.

### 8. `BlendTreeSystem → AnimationSystem` 순서 제약이 `docs/PATTERNS.md` 누락 — **medium** (신규 문서 갭)
- 근거: `docs/PATTERNS.md:106–115`, `src/animation/blend_system.rs:12`, `src/animation/system.rs:32`
- 문제: 코드 doc comment에는 "BlendTreeSystem must run before AnimationSystem"이 명시돼 있으나, 공식 레퍼런스인 PATTERNS.md의 ordering 표에 해당 행이 없다. 표만 참고한 개발자가 `BlendTreeSystem.after(AnimationSystem::LABEL)`로 등록하면 같은 프레임의 파라미터 변경이 다음 프레임 UvRect에 반영되고, StateMachineSystem도 한 프레임 지연된 상태를 읽는다.
- 권고: PATTERNS.md ordering 표에 "`BlendTreeSystem` before `AnimationSystem`" 행 추가.

### 9. 렌더 루프 내 `HashSet` 두 개 매 프레임 신규 할당 — **medium** (신규)
- 근거: `src/renderer/sprite.rs:477–482`
- 문제: `let live_material_entities: HashSet<Entity>`와 `let mut seen_new_hashes: HashSet<u64>`가 `render()` 내부에 매 프레임 새로 생성된다. ShaderMaterial 엔티티가 0개일 때도 두 HashSet이 heap-allocate된다.
- 권고: `SpriteRenderer` struct 필드로 승격, 매 프레임 `clear()` 후 재사용.

### 10. `AmbientLight`/`PointLight` WASM 무음 no-op — **medium** (신규 문서 갭)
- 근거: `src/resources.rs:387–414`, `src/components.rs:238–267`, `src/app/render.rs:210`
- 문제: `AmbientLight`·`PointLight`는 모든 타겟에서 컴파일·노출되지만 WASM에서는 `use_lighting = false` 고정과 라이팅 렌더러 `#[cfg(not(target_arch = "wasm32"))]` 제거로 완전히 동작하지 않는다. 두 타입의 doc comment에 WASM 제한이 없어 WASM 개발자에게 silent no-op을 유발한다. `CLAUDE.md`에는 "native-only"라고 적혀 있지만 API 레벨 doc에는 없다.
- 권고: `AmbientLight`, `PointLight`, `LightingRenderer` doc comment에 "native-only; WASM에서는 no-op" 명시.

---

## 3. 나머지 findings (low / 문서 갭 / 구조)

> 형식: 파일:라인 | 요약 | 실패 시나리오 | 제안 수정 방향

### renderer / particle

- `src/particle.rs:188` — `ParticleEmitter::texture: Option<String>`, 매 프레임 `.clone()` **medium** | v5.0.0에서 `Sprite.texture`는 `Arc<str>`로 전환됐으나 `ParticleEmitter`는 미처리 — emitter당 프레임당 String heap copy. | `Option<Arc<str>>`로 전환.

- `src/renderer/sprite.rs:310–460` — Sprite/AtlasSprite GlobalTransform+Transform 컬링 블록 4회 복사 **low** | 컬링 기준 변경 시 4곳 수정 필요. | `fn collect_sprite_entry(...)` 헬퍼 추출.

### physics

- `src/physics/system.rs:131–180` — 접촉쌍 vs 센서쌍 정규화 비대칭 **medium (PLAUSIBLE)** | 접촉쌍은 Rapier 반환 순서 그대로, 센서쌍은 `ordered_pair` 정규화. sleeping→waking 전환 시 Rapier가 센서쌍 핸들 순서를 바꾸면 `Exited`+`Entered`가 같은 프레임에 이중 발행. | 접촉쌍도 `ordered_pair`로 통일하거나, 두 경로 모두 "Rapier 보장 여부" 주석 추가.

### ecs

- `src/ecs/world.rs:202–203` — `despawn` 시 change-tracking 셋 O(N×M) `retain` **low** | N개 엔티티 동시 despawn(씬 리셋) × M개 변경 추적 항목 = O(N×M) 스캔; 1000×1000 = 100만 비교. (이전 분석의 "despawn linear scan" — `entity_location` 미활용 — 과 다른 경로.) | `added_this_tick`/`changed_this_tick`을 `HashSet<Entity>` → `HashMap<Entity, _>` 전환으로 O(1) remove, 또는 씬 리셋 시 전체 clear.

- `src/app/schedule.rs:269` — `HierarchySystem` LABEL 상수 없음 **low** | 사용자 시스템이 `GlobalTransform`을 중간에 읽어야 하는 경우 `.after(HierarchySystem::LABEL)` 선언 불가. (이전 분석에서 "대부분 내장 시스템 LABEL 미보유" 문제의 잔여; v4.6.0 sweep에서 PhysicsSystem 등은 추가됐으나 HierarchySystem은 파이프라인 밖 실행이라 미처리.) | `HierarchySystem::LABEL` 상수 추가.

### audio

- `src/audio/playback.rs:183` — `update()` 내 매 프레임 `Vec<String>` 할당 **low** | `self.fades.keys().cloned().collect()` — 페이드 0개일 때도 Vec heap-allocate. | scratch `Vec<String>` 필드 `AudioManager`에 추가, `clear()` 후 재사용.

- `src/audio/bus.rs:65–69` — `fade_current_vol` 헬퍼를 `bus.rs`가 inline 재구현 **low** | `playback.rs`에 `fade_current_vol(fades, channel) -> Option<f32>` 자유 함수 존재; `bus.rs`의 `fade_out`·`fade_volume`이 3줄 동일 로직 복사 — 한쪽 수정 시 이탈 위험. | `bus.rs`에서 `fade_current_vol` 호출.

### animation

- `src/animation/system.rs:38` — 3개 animation 시스템 매 프레임 `Vec<Entity>` 재할당 **low** | `AnimationSystem`(line 38), `BlendTreeSystem`(line 18), `StateMachineSystem`(line 294)가 각각 `world.query::<X>().map(|(e,_)| e).collect()` — 시스템 struct에 scratch 필드 없음. | 각 시스템에 `entities: Vec<Entity>` 필드 추가, 매 프레임 `clear()` + extend.

- `src/animation/state_machine.rs:258` — 트랜지션 발동마다 `transition.to.clone()` **low** | `evaluate()`가 `Option<(String, usize, f32)>` 반환 시 `to` 소유권 복제. | `Option<(&str, usize, f32)>` 반환 후 `sm.current` 기록 시에만 소유권 취득.

- `src/animation/state_machine.rs:94–98` — `set_bool`/`set_float` 매 호출마다 파라미터 키 `String` 할당 **low (altitude)** | 게임 AI가 매 프레임 `sm.set_bool("is_running", ...)` 호출 시 지속적 할당. | 파라미터 키를 `Arc<str>` 또는 등록 시 인덱스로 인턴화.

- `src/behavior.rs:385–388` — `BehaviorSystem` 엔티티당 매 프레임 archetype 마이그레이션 2회 **low** | `take_component::<BehaviorTree>` (archetype 축소) → tick → `add_component` (archetype 복원). BehaviorTree 엔티티 수가 많을 때 선형 마이그레이션 비용. | BehaviorTree tick에 `&mut self`가 필요한지 검토; 가능하면 `get_mut` 직접 접근으로 마이그레이션 제거.

### scripting

- `src/scripting/execution.rs:85` — `bb_snap.insert(key.to_string(), ...)` per Blackboard 항목·프레임 **low** | outer HashMap은 `std::mem::take`/restore로 재사용하지만 키는 매번 새 String 할당. | Blackboard 키를 `Arc<str>`로 인턴화.

- `src/scripting/api.rs` (전체) — SCRIPT_CTX 접근 보일러플레이트 15회 copy-paste **low** | 15개 등록 함수 모두 동일한 `SCRIPT_CTX.with(|c| { borrow_mut().as_mut().expect("...") ... })` 패턴 복사 — panic 메시지 변경 시 15곳 수정 필요. | `fn with_ctx_mut<R>(f: impl FnOnce(&mut ScriptCommands) -> R) -> R` 헬퍼 추출.

### ui

- `src/ui/system/button_pass.rs`, `checkbox_pass.rs`, `slider_pass.rs`, `text_input_pass.rs` — UiNode 추출 블록 4회 중복 **low** | 4개 위젯 패스 파일 각각의 첫 ~25줄이 구조적으로 동일(`query2 + collect → UiNode get → (pos, size, z, visible) 추출 → !visible continue`). UiNode 필드 추가 시 4파일 동시 수정 필요. | `fn node_layout(world, entity, viewport) -> Option<(Vec2, Vec2, f32, bool)>` 헬퍼 추출.

### editor

- `src/app/editor/ui/mod.rs:30, 261, 431` — 엔티티 레이블 포맷 불일치 **low** | 씬 그래프·태그 에디터는 `"Entity {}:{}"`, entity list 패널은 `"E{}:{}"` — 동일 엔티티가 패널에 따라 다르게 표시됨. | `fn entity_label(e: Entity) -> String` 헬퍼로 통일.

### 핫 리로드 / WASM

- `src/asset/hot_reload.rs:16–21` — 핫 리로드 WASM 무음 no-op, 콜 사이트 미문서화 **low (문서 갭)** | `poll_reloads()` WASM 브랜치가 `Vec::new()` 반환; `AssetServer` doc(line 135–137)은 WASM 예외를 언급하지 않음. | `poll_reloads()` doc 또는 `AssetServer` doc에 "WASM에서는 항상 빈 Vec 반환" 명시.

---

## 4. 모듈별 건강 요약

| 모듈 그룹 | 한 줄 평가 | 이번 라운드 findings (H/M/L) |
|---|---|---|
| network | 로직 정교하나 오버플로 경로에 데이터 손실 2건 | H2 |
| animation | crossfade 품질 향상됐으나 중단·완료 케이스 잔류 + 3시스템 per-frame alloc | M2 / L4 |
| physics | 핸들 newtype 완료; 충돌 이벤트 silent drop + 정규화 비대칭 신규 | M2 + 1 PLAUSIBLE |
| renderer | `Arc<str>` 전환 후 `image_handle` 경로 잔류 + HashSet 프레임당 할당 | M2 / L1 |
| particle | `Sprite`와 동일 패턴 미적용 (`Option<String>` 잔존) | M1 |
| scripting | bb_snap 키 할당 + SCRIPT_CTX boilerplate | M1 / L2 |
| ui | 위젯 패스 UiNode 추출 중복 | L1 |
| audio | fade helper 재구현 + per-frame Vec 할당 | L2 |
| ecs / scheduling | despawn change-tracking O(N×M) + HierarchySystem LABEL 없음 | L2 |
| docs/PATTERNS.md | BlendTree 순서 제약 누락 | M1 |

---

## 5. Top-10 우선순위 표

| 순위 | 파일:라인 | 요약 | 심각도 |
|---|---|---|---|
| 1 | `network.rs:62–68` | `dropped` 항상 1, 실제 손실 수 영구 불명 | **high** |
| 2 | `network.rs:62–64` | 오버플로 시 `pop_back()`이 마지막 정상 이벤트 삭제 | **high** |
| 3 | `animation/player.rs:76–97` | crossfade 중단 후 새 blend → FROM 클립으로 visual pop | **medium** |
| 4 | `animation/system.rs:76` | crossfade 완료 시 `to_timer` 버림 → 저fps 스터터 | **medium** |
| 5 | `physics/character_movement.rs:51` | lag spike 시 `drop_timer` 즉시 소멸 → 낙하 실패 | **medium** |
| 6 | `physics/system.rs:156–164` | `CollisionEvent` 미등록 시 이벤트 무음 드롭 (경고 없음) | **medium** |
| 7 | `renderer/sprite.rs:341, 499` | `image_handle` 경로 sprite당 매 프레임 `Arc` 재할당 | **medium** |
| 8 | `docs/PATTERNS.md:106–115` | `BlendTreeSystem` 순서 제약 표 누락 | **medium** |
| 9 | `renderer/sprite.rs:477–482` | 렌더 루프 내 `HashSet` 2개 매 프레임 신규 할당 | **medium** |
| 10 | `resources.rs:387–414` | `AmbientLight`/`PointLight` WASM no-op 미문서화 | **medium** |

---

## 6. 기각된 후보 (재조사 불필요)

검증 단계에서 실제 코드를 재확인한 결과 버그가 아닌 것으로 판정된 항목들. 다음 세션에서 재조사하지 않도록 기록한다.

| 항목 | 기각 근거 |
|---|---|
| `audio/playback.rs` — `stop()` 후 `fades.remove` 이중 호출 | HashMap remove는 멱등 — no-op, 데이터 손실 없음 |
| `audio/types.rs:16` — `release_secs` 최솟값 항상 `0.001` 클램프 | `Fade::stop_fade` minimum-duration 규칙과 의도적으로 일치 — 문서화된 설계 |
| `save.rs:252–253` — `<=` 경계 조건 AEAD underflow | 0-byte ciphertext는 ChaCha20Poly1305 태그 검증 실패 → 올바르게 `Corrupted` 반환 |
| `resources.rs:483–484` — `FadeTransition` WASM 비주얼 없음 | `resources.rs:483`·`app.rs:123` 양쪽에 이미 문서화됨 — 의도적 |
| `ui/localized.rs` — `LocalizationSystem` PATTERNS.md 누락 | `PATTERNS.md:115` ordering 표에 이미 존재 |
| `renderer/sprite.rs:9–19` — 렌더러가 animation 모듈 import | `UvRect`/`BlendUv`를 `renderer/uv.rs`로 이전 후 해당 import 제거됨 — 의존 방향 clean |
| `animation/player.rs:144` — 빈 클립의 `is_finished` underflow | line 144에 guard 존재 — 실제 도달 불가 |
| `app/scenes.rs:87–97` — Push 씬 라벨 정렬 문제 | 라벨 기반 정렬이 전역 — Push로 추가된 시스템도 올바르게 정렬됨 |
| `app/schedule.rs:305–311` — 이벤트 플러시가 씬 전환 전 발생 | 의도된 실행 순서: systems → event flush → scene transition; 신씬은 다음 프레임부터 시작 |
| `asset/hot_reload.rs:34` — dedup `!seen.contains` O(n) 스캔 | 정확성 버그 아님; 핫 리로드는 드물게 발생 → 실질 impact 미미 |
| `physics/system.rs:131–144` 접촉쌍 순서 보장 가정 | Rapier 문서가 같은 접촉쌍의 핸들 순서를 프레임 간 보존한다고 명시 — 접촉 경로 자체는 설계 의도에 부합; 센서 경로 비대칭 문제(§3)는 별도 PLAUSIBLE로 보존 |
