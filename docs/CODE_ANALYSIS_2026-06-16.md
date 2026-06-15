# skeleton-engine 개선 기회 분석 보고서 (영속성 작업 외)

> 분석일: 2026-06-16 | 대상: `src/` 전체 14개 서브시스템 (~43.8k LOC) | 패키지 `skeleton-engine` v8.27.0 (crate `engine`)
> 검토 커밋: `42de46c` (main) | lib 단위 테스트 603개
> 방법: 14개 서브시스템별 병렬 finder(실제 코드 file:line 인용 강제) → 후보별 적대적 검증 에이전트(인용 코드 직접 열람, 거짓양성/이미처리/구조상 불가능 기각) → 합성. 원시 84후보 → **확정 80 · 기각 4**.
> 제외 범위: **월드 영속성 브리지**(`World::to_scene`/`apply_scene`, 씬 스냅샷, Reflect-vs-serde 레지스트리 통합)는 별도 발주 작업이라 본 분석에서 제외(영속성-플래그 후보 0건). 본 문서는 "그 외 지금 당장 개선 가능한 지점"만 다룬다.

---

## 1. 종합 진단 — "조용한 실패(fail-quiet)"가 엔진 전반의 지배적 패턴

영속성 발주서가 지목한 **Footgun B**(컴포넌트 등록 누락 → 씬 저장에서 조용히 소실)는 영속성 한정 문제가 아니라 **엔진 전역에 퍼진 동일 계열 결함**이다. 확정 80건 중 가장 두꺼운 군집이 정확히 이것 — *잘못된 입력·데이터·상태를 만나도 panic도 log도 없이 조용히 틀린 동작을 한다.* 즉 발주된 영속성 작업과 **같은 철학(fail-loud)** 을 엔진 전역에 적용하는 것이 지금 가장 가치 있는 방향이다.

검증 후 심각도 분포: **HIGH 2 · MEDIUM 44 · LOW 34.** finder가 HIGH로 올린 다수가 MEDIUM으로 하향됐는데 이유가 일관된다 — "현재 엔진 코드는 안 밟지만 **포크 사용자가 밟는 잠복형 함정**" 또는 "blast radius가 좁음". 이는 *셸 자체는 멀쩡한데 포크하는 사람이 걸려 넘어진다*는 뜻으로, fork-friendly 스켈레톤이라는 정체성에 정면으로 반하는 종류다.

서브시스템별 확정 건수: ecs/reflect/prefab 6 · app-loop 5 · renderer-core 6 · renderer-lighting/camera 6 · physics 6 · animation 6 · audio 6 · input 6 · ui 5 · editor 4 · assets/scripting/tilemap 6 · network 6 · save/behavior/path/timeline 6 · cross-cutting(build/CI) 6.

> 전체 80건은 **부록 A** 표 참조(심각도·file:line·노력·breaking 포함). 본문 §2~§8은 우선순위 군집별 권고다.

---

## 2. 즉시 수정 — 실제 동작 결함 (HIGH + correctness-critical MEDIUM)

| 항목 | 위치 | 문제 | 수정 | 노력 |
|---|---|---|---|---|
| 🔴 **blob_47 자동타일 비트순서 불일치** | `src/tilemap.rs:222-226` vs `compute_tile_mask` `289-318` | `VALID_MASKS`가 `compute_tile_mask`와 **다른 비트 컨벤션**으로 생성됨. 생성 가능한 47마스크 중 **36개가 테이블에 없어** `.unwrap_or(0)`(line 527)로 떨어져 **타일 0 렌더**. 직교 이웃만 있는 평범한 구성(북쪽 이웃 1개=mask 1)이 전부 깨짐. 예제가 Edge4/16만 써서 잠복 | `VALID_MASKS`를 코드 컨벤션의 47값으로 교체: `[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,19,23,27,31,38,39,46,47,55,63,76,77,78,79,95,110,111,127,137,139,141,143,155,159,175,191,205,207,223,239,255]` + 3×3 맵 비-0 단언 테스트 | small |
| 🔴 **게임패드 축 `just_pressed` 무시** | `src/input/map.rs:228-242` | `is_pressed_with_gamepad`는 `gamepad_axes`를 보는데(216-218) `just_pressed/just_released_with_gamepad`는 **축을 안 봄**. docstring은 본다고 적혀 코드리뷰로도 안 잡힘. 스틱 기울임으로 트리거되는 메뉴/원샷 액션이 항상 false | `\|\| b.gamepad_axes.iter().any(\|ab\| ab.is_active(gamepad.axis(pad, ab.axis)))` 추가, `just_released`에도 대칭 적용 | small |
| 애니 RON `columns=0` → **div-by-zero panic** | `src/animation/clip_set.rs:123` | `i % columns`가 RON 값 검증 없이 실행 → 런타임 패닉 | `from_ron_str`에서 `columns==0 \|\| rows==0` 검증 → `ClipSetError::Ron` | small |
| OOB 프레임 인덱스 → **UV [0,1] 밖**(쓰레기 픽셀) | `src/animation/clip_set.rs:123` | 프레임 인덱스 초과 시 경고 없이 sampler clamp/wrap. 흔한 off-by-one | `i/columns >= rows` 검증/로그 또는 `UvRect::FULL` 대체 | small |
| `AnimationPlayer::play(OOB)` → freeze + `is_finished()` 즉시 true | `src/animation/player.rs:64-71, 167` | OOB 인덱스 그대로 set → 스프라이트 freeze. `is_finished()` fallback이 `true`라 SM `AnimationEnd` 즉발 (단, SM은 등록 클립 인덱스로 평가하므로 즉발은 OOB==등록인덱스일 때만) | `play()`에 bounds 가드, `is_finished()` fallback을 `false`로 | small |
| SM **존재하지 않는 target 전이 = 영구 dead edge** | `src/animation/state_machine.rs:166-171, 366` | `to` 상태명 오타가 컴파일/실행 통과 → 조건 충족돼도 `evaluate()`가 `None`. 에디터 문자열 입력에서 특히 위험 | `add_transition_crossfade`에서 `to` 미존재 시 `log::warn!`/거부 | small |
| `SkeletalAnimator::is_finished()` 생성 직후 true | `src/skeletal.rs:160` | `duration=0` 비-루프 클립이 시스템 1틱 전부터 finished 보고 (AnimationPlayer는 `finished` 플래그로 이미 방어) | `started: bool` 플래그 추가 또는 `duration>0.0` 가드 | small |
| 0축 벡터 `add_prismatic_joint` → **NaN 조인트** | `src/physics/world/joints.rs:49` | (검증 정정: 패닉 아님) `new_normalize`가 IEEE NaN 생성 → 이후 물리 스텝 오염. 두 엔티티 위치 일치 시 발생 | `axis.length() < EPSILON` 가드 | small |

---

## 3. ⏳ v1.0 API freeze **이전에** 확정해야 하는 가산형 API (시한성)

지금은 additive지만 freeze 후엔 영원히 breaking이 되는 항목 — **타이밍이 전략적으로 중요**하다.

- **`GamepadButton`/`GamepadAxis`에 `#[non_exhaustive]` 부재** — `src/input/gamepad.rs:4-35`. `ReflectValue`/`DebugShape`는 이미 붙어 있는데 이 둘만 누락. freeze 후 버튼/축 추가는 다운스트림 match exhaustiveness를 깨뜨림. 내부 `map_button/map_axis`는 `_ => None` 와일드카드라 엔진은 무영향. **freeze 전 필수.** (breaking-now-only)
- `InputMap::axis_value(action, gamepad, pad) -> f32` 부재 — `src/input/map.rs`. 아날로그 이동은 `InputMap`을 우회해 `GamepadState::axis()` 직접 호출해야 함 → 추상화 층 무의미. (small)
- `Panel::direction`가 Reflect 미노출 — `src/ui/panel.rs:43-51`. 에디터에서 레이아웃 축 토글 불가. `Anchor::to_i32` 선례대로 `LayoutDir`를 `I32` 인코딩. (small)
- `Track::set_value()/set_easing()` 부재 — `src/timeline.rs:110-146`. 에디터 키프레임 값이 **읽기 전용**(`docked.rs:1332`가 비대화 라벨). remove+re-add만 가능(인덱스 깨짐). (small)
- `RenderTarget` per-target clear color 부재 — `src/app/render.rs:570-574`가 항상 불투명 검정으로 클리어, `WindowConfig::clear_color` 무시. 투명 배경 RT 불가. `clear_color: Option<[f64;4]>` 필드 추가. (small)
- `ParticleEmitter`에 `z` 부재 — `src/particle/mod.rs:188, 315`가 모든 파티클 `z=0.0` 하드코딩. 렌더 레이어 쓰는 게임에서 배경 이펙트가 캐릭터 앞에 그려짐. `pub z: f32`(default 0.0) 추가. (small)
- **UI 위젯이 에디터 factory/remover 맵에 누락** — `src/app/editor.rs:537-564`(Sprite/RenderLayer/ParticleEmitter/PointLight/Tag만). UiNode/Button/Label 등은 serde 등록은 됐는데 "+ Add Component"에 안 뜸. *영속성 발주서의 Footgun B와 동일한 수작업-나열 문제의 또 다른 발현 — 단일 레지스트리 통합 작업과 묶어 처리 가능.* (small)
- `save_versioned_with_key`/`load_migrated_with_key` 부재 — `src/save.rs:413`이 `SaveKey::DEFAULT` 하드와이어. 커스텀 키 + 버전 마이그레이션 조합 불가(`save`/`save_with_key` 쌍 패턴 미러). (small)
- `World::has_component::<T>()` 부재 — `src/ecs/world.rs:786`의 `has_component_typeid`가 `pub(crate)`. 포크는 `get::<T>().is_some()`로 불필요한 downcast. 1줄 공개 래퍼. (small)
- `Camera::is_zooming()/zoom_target()/shake_remaining()` 접근자 부재 — `src/camera.rs:55-57`. tween 상태 관측 불가. (small)

---

## 4. Fail-loud 로깅 스윕 (대부분 `log::warn!` 한 줄 — 묶어서 1 PR 권장)

영속성 발주서 Footgun A/B와 동일 계열. 전부 비파괴적.

| 위치 | 조용히 사라지는 것 |
|---|---|
| `src/app/editor/ui/mod.rs:156, 550-558` | **인스펙터 쓰기 back**: register 이름 ≠ `Reflect::type_name`이면 모든 편집 no-op. write-back을 TypeId 키로 변경(읽기는 `type_name`, 쓰기는 register명). 포크가 "Enemy Stats" 같은 표시명 쓰면 인스펙터 통째 먹통 |
| `src/prefab.rs:199-200` | `serialize_entity`가 `ron::to_string` 실패를 `.ok()`로 삼켜 컴포넌트를 씬에서 누락 (deserialize 쪽은 로그하는 비대칭) |
| `src/data_table.rs:105-118` | DataTable이 2행+ **추가 컬럼을 조용히 폐기** → save 시 디스크에서 영구 소실 |
| `src/prefab.rs:455-459` | `SerdeComponentRegistry` 없는 World에서 `spawn_entity_def`가 모든 컴포넌트 폐기 |
| `src/renderer/text.rs:459, 487` | glyphon `prepare()`/`render()` 에러를 `let _ =`로 삼킴 → 텍스트 누락돼도 단서 없음 (단 `AtlasFull`은 `trim()`로 자가 복구되는 transient) |
| `src/network.rs:349` | 큐 가득 차면 `disconnect()`가 조용히 no-op, `is_connected()` 계속 true |
| `src/network.rs:480-486` | WASM `on_error`가 원인 버리고 고정 문자열만 (`ErrorEvent::message()` 추출 가능) |
| `src/network.rs:599-612` | `Events<NetworkEvent>` 미등록 시 `poll()`이 드레인한 이벤트 전량 폐기 (`resource_or_insert_with`로 자동 등록 권장) |
| `src/ecs/events.rs:5` | 문서가 "다음 프레임에도 읽힘" 주장하나 `flush()`는 매 프레임 끝에 비움 — **문서가 거짓** |
| `src/app/editor/ui/docked.rs:700-720` | 씬 리로드 후 `add_component_selected` stale → "+ Add" 버튼 조용히 no-op |

---

## 5. 입력·카메라 정합성 (조용한 오작동)

- **포커스 상실 시 키 stuck** — `src/app/window.rs`의 `WindowEvent` match에 `Focused` arm 자체가 없음(`_ => {}`로 폐기). Alt-Tab으로 키 쥔 채 전환 시 winit이 release 미발행 → `InputState::pressed`에 영원히 잔류 → 캐릭터 무한 이동. `InputState::release_all()` 추가 + `Focused(false)` arm. **단 `just_released`엔 넣지 말 것**(키마다 유령 release 펄스). (small)
- **유령 `just_released`** — `src/input/state.rs:116-119`의 `release()`가 `pressed`에 없던 키도 무조건 `just_released` 삽입. 도킹 에디터 텍스트 입력 중 뷰포트 클릭 전환(`egui_wants_keyboard` true→false) 시 "누른 적 없는 키 release". `if pressed.remove(&key) { just_released.insert(key) }`. `release_mouse`도 동일. (small)
- **TextInput 포커스가 z-order 무시** — `src/ui/system/text_input_pass.rs:26-35`가 첫 ECS 히트에 `break`(Button은 최상위 z 선택). 겹친 TextInput에서 비결정적. (small)
- **숨겨진 TextInput이 키 입력 삼킴** — `text_input_pass.rs:27`이 `visible` 폐기. focused 상태로 hide되면 `ti.focused` 미해제 → 키 입력 계속 소비. (small)
- **`screen_to_world`/`world_to_screen`가 shake_offset 누락** — `src/camera.rs:92-111` vs `view_proj`(143). 화면은 흔들리는데 마우스 픽킹/기즈모(`gizmo.rs:458,946`, `docked.rs:1448,1472`)는 비-흔들림 좌표 → 셰이크 중 클릭 타깃 어긋남. (small)
- **조명 nearest-16 컬링이 뷰포트 좌상단 모서리 기준** — `src/renderer/lighting.rs:396`에 `camera.position`(=좌상단) 전달. 1280×720에서 중심 대비 640px 어긋나 우하단 조명 부당 탈락. 호출부에 `vp_w/vp_h` 이미 있어 `camera.position + Vec2::new(vp_w,vp_h)/(2*zoom)`로 한 줄 수정. (small)
- **TouchState 물리 픽셀 좌표 vs 논리 픽셀 혼선** — `src/app/window.rs:415`가 scale 분할 없이 저장, `touch.rs:7-9` 문서는 "screen coordinates"로 모호. HiDPI에서 `screen_to_world`/UI hit-test와 좌표계 불일치. swipe 임계값 50.0도 DPI별 부적절. (docs/small)

---

## 6. 성능 핫패스 (매 프레임 불필요 할당)

- **🔥 Tilemap이 변경 없어도 매 프레임 전체 그리드 clone** — `src/tilemap.rs:598-611`(`tm.clone()`) + `708`(`cached_tiles.clone()`), early-out은 `725`에서야. 100×100 맵이 변경 0이어도 ~80KB×2/프레임. **2개 finder가 독립 발견.** `generation: u64` 카운터(set_tile에서 증가) + `TilemapView::cached_generation` 비교로 정지 상태 비용을 u64 비교 1회로. (small~medium)
- **cosmic-text가 매 프레임 모든 `DrawText` 재셰이핑** — `src/renderer/text.rs:335`. glyphon은 글리프 비트맵만 캐시, layout/shape는 매번 `Shaping::Advanced` 풀 bidi. 정적 HUD 라벨 10개 = 초당 ~600회 셰이핑. `(text,size,bounds,align)` 키 셰이프드-버퍼 캐시. (medium)
- **`Arc::from(atlas.texture_path())`가 AtlasSprite마다 매 프레임 문자열 복사** — `src/renderer/sprite.rs:441`. `Sprite` 경로는 이미 `path_arc()`로 O(1)인데(`asset.rs:52-58`이 `Arc::from` 경고까지 함) atlas만 누락. `TextureAtlas::texture_path_arc()` 추가. (small)
- **인스펙터가 매 프레임 `serialize_entity` 풀 RON 직렬화** — `src/app/editor/ui/docked.rs:645-649`(키 이름만 필요한데 전체 직렬화). 이름-전용 type-id 체크 메서드로. (medium)
- 다중 `GpuParticleEmitter` **공유 링버퍼 슬롯 충돌** — `src/gpu_particle.rs:52,67,125`(각 emitter `next_slot=0` 독립) → 멀티 emitter 시 서로 덮어씀. (native-only/experimental 잠복) (medium)
- 매 프레임 scratch 할당 묶음(전부 필드 재사용으로 해결): `SpriteRenderer::render` 5개 Vec/HashSet(`sprite.rs:358,501,510-515`), `PhysicsSystem` 4개 Vec(`system.rs:177-178,224-225`), `SpatialGrid::rebuild`(`grid.rs:101-111`)+`candidates_in_aabb`, `SteeringSystem` 5개 Vec(`steering.rs:118,146,177,214,245`), `AudioManager::update` `Vec<String>`(`playback.rs:179`, `ducking.rs:153,205,227`), `query_added/changed`(`world.rs:682-705`).
- `SolidTiles::Only(Vec)`가 타일마다 O(N) 선형 스캔 — `tile_collider.rs:212`. `HashSet<u32>`로(생성자 `impl IntoIterator`로 additive 유지). (small, 필드 타입은 breaking)
- `TilemapSystem` 제거-엔티티 체크가 `Vec::contains` O(M×N) — `tilemap.rs:567,570-575`. `HashSet`로. (small)
- bloom 셰이더 per-fragment `textureDimensions()` — `post_process.wgsl:65-66`. uniform `texel_size`로 이동. (small)
- 자동타일 UV-refresh가 인접 셀 중복 누적 — `tilemap.rs:730,770,778`. `Vec` → `HashSet`. (small)

---

## 7. 라이프사이클 누수 · 앱 루프 견고성

- **`TilemapColliders` 엔티티 despawn 시 rapier 바디 전량 누수** — `src/physics/world/tile_collider.rs:242-276`. `Drop`도 cleanup 시스템도 없음. (`set_scene`가 PhysicsWorld 재생성하므로 실무 blast radius 좁음) despawn-observer 또는 `drain_into_physics(&mut PhysicsWorld)` 헬퍼. (medium)
- **Resized → `step_frame` 이중 실행** — `src/app/window.rs:127-129`(Resized) + `462`(RedrawRequested)가 같은 이벤트루프 반복에서 둘 다 호출(`step_frame`이 `next_frame` 미진행 → `about_to_wait`의 deadline 체크 통과). macOS 드래그 리사이즈 중 **물리/Tween/Timer가 프레임당 2회** 진행. `stepped_this_iteration` 플래그 가드. (small)
- **`catch_unwind`가 패닉 후 World를 미정의 상태로 두고 잔여 시스템 계속 실행** — `src/app/schedule.rs:326-330`. `DisableSystemAndContinue`는 다음 프레임 재실행만 막고, 같은 프레임 후속 N개 시스템이 반쯤-변형된 archetype을 읽음. 주석이 오해 소지("further damage 방지"=재실행만). 패닉 시 프레임 잔여 시스템 abort + rustdoc 경고. (medium)
- **핫리로드 디스패치 하드코딩** — `src/app/schedule.rs:452-497`에 레지스트리별 복붙 4블록. 포크가 새 RON 레지스트리 추가 시 엔진 `update()` 직접 편집 필요(누락 시 조용히 리로드 안 됨). `trait HotReloadable { fn reload_path(&mut self,&str) }` + `App::register_hot_reloadable::<T>()`로 통합. (medium, fork-friendliness)
- **WASM send가 `max_pending_messages` 무시** — `src/network.rs:526-538`(native는 256 캡). 브라우저 `bufferedAmount` 무한 적재 → 탭 크래시. `max_buffered_bytes: Option<u32>` + `buffered_amount()` 체크. (medium)
- `RemoteEntities`가 외부 despawn 후 stale 핸들 반환 — `network.rs:673-681`. `get_or_spawn`이 생존 확인 없이 캐시 반환 → `set_scene` 후 죽은 엔티티에 `add_component` no-op. `is_alive` 체크. (small)
- NetworkClient **reconnect API 부재** — `network.rs:148-319`(native, `Drop` 없음). 리소스 교체 시 백그라운드 스레드/클로저 누수. `Drop` 구현 + `reconnect()`. (medium)
- 오프스크린 루프의 **raw 포인터 fragile** — `src/app.rs:60-68`, `render.rs:532,546`. `render_targets` HashMap 재할당 시 dangling UB. `Texture::create_view()`(zero-cost)로 owned view 미리 수집하면 `unsafe` 제거 가능. (small)

---

## 8. 오디오 fade 상호작용 (3건, 단일 원인) · 빌드/CI 위생

**오디오 fade** — `update()`가 활성 fade 중 `fade.current_vol()`만 쓰고 `volume_overrides`를 무시(`playback.rs:184`)해 생기는 한 뿌리. `set_*`에서 `fades.contains_key(ch)` 가드 또는 fade 취소 정책으로 일괄 해결:
- `set_bus_volume` mid-fade → 1프레임 볼륨 snap (`audio/bus.rs:49`) — 에디터 믹서 슬라이더 드래그로 재현
- `update_position` mid-fade → 공간 볼륨이 다음 프레임 폐기 (`audio/positional.rs:38`)
- `set_volume` mid-fade → fade 끝까지 무시 (`audio/bus.rs:72-78`)
- (관련) `file_cache` 무한 증가 — `playback.rs:65,394`, eviction/`clear_file_cache()` 없음. 오디오는 wasm 전무인데 문서/스텁 가이드 없음(`lib.rs:5-6,63-64`)

**빌드/CI 위생** (trivial quick wins):
- **`serde_json`이 `[dependencies]`인데 lib 미사용** — `Cargo.toml:145`. grep 0 hit, examples만 사용. `[dev-dependencies]`로 이동(다운스트림 전이 의존 제거). (small)
- **MSRV 1.92 선언 vs CI는 1.95만 테스트** — `Cargo.toml:17` / `ci.yml:28-29`, 1.92 잡 없음. 1.93~1.95 기능 무방비 채택 가능. 1.92 잡 추가 또는 `rust-version`을 1.95로 정정. (small)
- **WASM CI에 clippy 없음** — `ci.yml:56-79`는 build만. `#[cfg(wasm32)]` 블록(17개 파일) lint 무방비. wasm 잡에 `components: clippy` + `cargo clippy --target wasm32`. (small)
- 44k LOC에 **통합 테스트 단 2개** — `tests/`(derive_reflect, editable_component_scene_replace). network/audio/collision/pathfinding/physics/scripting/timeline 크레이트 경계 스모크 테스트 부재. 영속성 발주서의 "풀 월드 라운드트립 테스트"가 첫 단추. (large)

---

## 9. 기각된 후보 (4건, 투명성 기록)

검증 에이전트가 **거짓양성으로 기각** — 이미 처리됐거나 구조상 불가능:
1. *"CollisionEvent/TriggerEvent가 register 없을 때 매 프레임 경고 폭주"* → `physics/system.rs:65-66,84-85,196-202,243-249`에 `warned_missing_*` 플래그로 **이미 1회만 출력**(제안 수정이 이미 구현됨).
2. *"IME preedit 중 Enter가 조합 문자 유실"* → `window.rs:238-243`에서 `Ime::Commit`이 `push_text`+`clear_ime_preedit`를 프레임 전 호출, 조합 문자는 일반 char로 도착. `ti.preedit`는 표시 전용 미러(데이터 경로 아님).
3. *"SM 패널 `.expect()` race 패닉"* → `schedule.rs:296-396` 단일 스레드 직렬 실행, `docked.rs:1156-1195`가 단일 불변 borrow 유지 → 변경 윈도 없음(구조상 불가능).
4. *"Data table 패널 `.expect()` 핫리로드 race"* → `update_editor_ui`(`schedule.rs:396`)가 핫리로드 블록(`460-471`) 전 완료, await 없는 동기 호출, `DataTableRegistry`는 persistent.

---

## 10. 권장 1차 배치 (높은 ROI · 대부분 small)

1. **진짜 버그 2건** — `blob_47` 마스크 테이블, 게임패드 축 `just_pressed`(회귀 테스트 포함).
2. **애니메이션 RON 검증 묶음** — `columns=0` 패닉 / OOB 프레임 / `play(OOB)` / dead transition / skeletal `is_finished`. 데이터 파일 기인 크래시·쓰레기 렌더를 한 PR로 차단.
3. **입력 견고성** — `Focused(false)→release_all` + guarded `just_released`. 체감 큰 버그.
4. **freeze 전 가산형 API** — 최소 `#[non_exhaustive]` 2개 enum은 freeze 전 확정(이후 영구 breaking).
5. **Fail-loud 스윕** — §4 전체를 `log::warn!` 한 PR로. 영속성 작업의 fail-loud 요구와 철학 동일 → 묶으면 시너지.
6. **성능** — Tilemap `generation` dirty-guard(2 finder 합치), 그 다음 text shaping 캐시.

> 검증 신뢰성: 모든 항목은 검증 에이전트가 인용 file:line을 직접 열어 확인했고, 기각 4건(§9)은 *제안 수정이 이미 구현됨* 또는 *단일 스레드 구조상 불가능*으로 정확히 걸러졌다.

---

## 부록 A — 확정 80건 전체 (심각도 → 서브시스템 정렬)

| # | SEV | 분류 | 노력 | brk | 위치 | 항목 |
|---|-----|------|------|-----|------|------|
| 1 | HIGH | robustness | small | - | src/tilemap.rs:222-226 | blob_47 VALID_MASKS uses wrong bit-order convention: 36 reachable masks silently fall back to atlas tile 0 |
| 2 | HIGH | robustness | small | - | src/input/map.rs:228-242 | just_pressed_with_gamepad silently ignores all axis bindings — always returns false for axis-only actions |
| 3 | MEDIUM | robustness | small | - | src/animation/clip_set.rs:123 | AnimationClipSet panics on atlas columns=0 from RON data file |
| 4 | MEDIUM | robustness | small | - | src/animation/player.rs:64-71 | AnimationPlayer::play(OOB) silently marks animation finished, causing immediate AnimationEnd transitions |
| 5 | MEDIUM | robustness | small | - | src/animation/clip_set.rs:123 | Out-of-bounds frame index in RON clip set silently produces UV coordinates outside [0,1] |
| 6 | MEDIUM | robustness | small | - | src/skeletal.rs:160 | SkeletalAnimator::is_finished() returns true at construction for a zero-duration non-looping clip |
| 7 | MEDIUM | api-ergonomics | small | - | src/animation/state_machine.rs:166-171 | StateMachineSystem: add_transition to a nonexistent target state creates a permanently silent dead edge |
| 8 | MEDIUM | robustness | medium | - | src/app/schedule.rs:326-330 | AssertUnwindSafe catch_unwind leaves World in undefined state for all subsequent systems that frame |
| 9 | MEDIUM | fork-friendliness | medium | - | src/app/schedule.rs:452-497 | Hot-reload dispatch is hardcoded: forks adding a new RON-registry must modify the engine's update() loop |
| 10 | MEDIUM | robustness | small | - | src/app/window.rs:127-129 | Resized event calls step_frame unconditionally, causing double game-logic execution per drag event on macOS |
| 11 | MEDIUM | robustness | small | - | src/data_table.rs:105-118 | DataTable parse silently discards extra columns present in rows 2+ without any warning |
| 12 | MEDIUM | api-ergonomics | small | - | src/particle/mod.rs:188 | ParticleEmitter z coordinate is not propagated to spawned particles: all particles render at z=0.0 |
| 13 | MEDIUM | performance | small | - | src/tilemap.rs:598-611 | TilemapSystem clones the full tile grid (and autotile HashMap) unconditionally every frame even when nothing changed |
| 14 | MEDIUM | robustness | small | - | src/audio/bus.rs:49 | set_bus_volume during an active fade produces a one-frame volume snap |
| 15 | MEDIUM | api-ergonomics | small | - | src/audio/bus.rs:72-78 | set_volume() mid-fade is silently ignored until the fade completes |
| 16 | MEDIUM | robustness | small | - | src/audio/positional.rs:38 | update_position() during an active fade silently discards spatial volume |
| 17 | MEDIUM | robustness | small | - | Cargo.toml:17 | MSRV 1.92 declared but CI only tests 1.95 — no MSRV gate exists |
| 18 | MEDIUM | performance | medium | - | src/tilemap.rs:598-612 | TilemapSystem clones entire tile grid every frame even when nothing has changed |
| 19 | MEDIUM | wasm-portability | small | - | .github/workflows/ci.yml:56-79 | WASM CI job has no clippy pass — wasm-only code paths are never linted |
| 20 | MEDIUM | performance | small | - | Cargo.toml:145 | serde_json in [dependencies] is never used by the lib crate — only by examples |
| 21 | MEDIUM | robustness | small | - | src/app/editor/ui/mod.rs:156 | Inspector write-back silently discards edits when register name != Reflect::type_name |
| 22 | MEDIUM | robustness | small | - | src/prefab.rs:199-200 | SerdeComponentRegistry::serialize_entity silently drops component on ron::to_string failure |
| 23 | MEDIUM | performance | medium | - | src/app/editor/ui/docked.rs:645-649 | Per-frame serialize_entity in inspector component list builds full serialization every render frame |
| 24 | MEDIUM | api-ergonomics | small | - | src/app/core_resources.rs:73-96 | UI widget components missing from editor factory/remover maps — Add/Remove buttons silently absent |
| 25 | MEDIUM | fork-friendliness | small | **BRK** | src/input/gamepad.rs:4-35 | GamepadButton and GamepadAxis are not #[non_exhaustive] — adding variants breaks downstream match exhaustiveness |
| 26 | MEDIUM | api-ergonomics | small | - | src/input/map.rs:103-261 | InputMap has no axis_value() method — analog movement requires bypassing the abstraction layer |
| 27 | MEDIUM | robustness | small | - | src/input/state.rs:116-119 | InputState::release() emits just_released for keys that were never pressed — spurious events in Docked editor mode |
| 28 | MEDIUM | robustness | small | - | src/app/window.rs:86-465 | No WindowEvent::Focused handler — held keys stick on Alt-Tab / focus loss |
| 29 | MEDIUM | robustness | small | - | src/network.rs:349 | Native disconnect() silently no-ops when send queue is full |
| 30 | MEDIUM | api-ergonomics | small | - | src/network.rs:480-486 | WASM on_error emits no diagnostic detail — error cause is always lost |
| 31 | MEDIUM | wasm-portability | medium | **BRK** | src/network.rs:526-538 | WASM send path ignores max_pending_messages — unbounded browser bufferedAmount |
| 32 | MEDIUM | robustness | medium | - | src/physics/world/tile_collider.rs:242-276 | Despawning a TilemapColliders entity silently leaks all its rapier bodies |
| 33 | MEDIUM | robustness | small | - | src/physics/world/joints.rs:49 | add_prismatic_joint panics in debug (silent NaN in release) on a zero axis vector |
| 34 | MEDIUM | performance | small | - | src/renderer/sprite.rs:441 | Arc::from(atlas.texture_path()) copies the string on every AtlasSprite per frame |
| 35 | MEDIUM | robustness | small | - | src/renderer/text.rs:459 | Glyphon prepare() and render() errors silently swallowed |
| 36 | MEDIUM | robustness | medium | - | src/gpu_particle.rs:52,67,125 | Multiple GpuParticleEmitters collide on shared ring-buffer slots |
| 37 | MEDIUM | performance | medium | - | src/renderer/text.rs:335 | cosmic-text Buffer is re-created and re-shaped from scratch every frame per DrawText |
| 38 | MEDIUM | robustness | small | - | src/renderer/lighting.rs:396 | Nearest-16 light cull measures distance from camera top-left corner, not viewport center |
| 39 | MEDIUM | api-ergonomics | small | - | src/app/render.rs:570-574 | Offscreen RT always clears to opaque black, ignoring WindowConfig::clear_color |
| 40 | MEDIUM | robustness | small | - | src/camera.rs:92-111 | screen_to_world/world_to_screen ignore shake_offset, view_proj includes it |
| 41 | MEDIUM | robustness | small | - | src/behavior.rs:226-229 | Sequence and Selector reset current=0 on completion without calling child.reset() |
| 42 | MEDIUM | api-ergonomics | small | - | src/timeline.rs:110-146 | Track lacks set_value() and set_easing() mutators; editor keyframe values are read-only |
| 43 | MEDIUM | robustness | small | - | src/pathfinding.rs:183-191 | find_path / find_path_diagonal return Some([blocked]) when start == goal and that cell is blocked |
| 44 | MEDIUM | robustness | small | - | src/ui/system/text_input_pass.rs:27 | Invisible TextInput can receive keyboard focus on click |
| 45 | MEDIUM | api-ergonomics | small | - | src/ui/panel.rs:43-51 | Panel::direction not exposed in Reflect — editor inspector cannot toggle layout axis |
| 46 | MEDIUM | robustness | small | - | src/ui/system/text_input_pass.rs:26-35 | TextInput focus selection ignores z-order (first-ECS-hit wins) |
| 47 | LOW | robustness | small | - | src/animation/blend_tree.rs:58 | BlendTree1D silently selects wrong clip when entries are unsorted |
| 48 | LOW | wasm-portability | small | - | src/app.rs:132-135 | FadeTransition silently no-ops on WASM with no public API warning |
| 49 | LOW | robustness | small | - | src/app.rs:60-68 | Unsafe raw pointer for TextureView in offscreen render loop is silently fragile |
| 50 | LOW | performance | small | - | src/tilemap.rs:730,770,778 | Autotile neighbor UV-refresh Vec accumulates duplicate cells when multiple adjacent tiles change in one frame |
| 51 | LOW | code-quality | small | - | src/scripting/context.rs:15-17 | ScriptCommands::spawned_ids is a write-only dead field documented by its own TODO comment |
| 52 | LOW | wasm-portability | small | - | src/lib.rs:5-6 | Audio subsystem is entirely absent on wasm with no stub or documentation guidance |
| 53 | LOW | performance | medium | - | src/audio/playback.rs:179 | Per-frame Vec<String> allocations in the audio update hot path |
| 54 | LOW | robustness | small | - | src/audio/playback.rs:65 | file_cache grows without bound — no eviction or clear API |
| 55 | LOW | test-gap | large | - | tests/ | Only 2 integration tests exist for a ~44k-LOC multi-subsystem engine |
| 56 | LOW | performance | small | - | src/tilemap.rs:567 | TilemapSystem removed-entity check is O(views × entities) — a quadratic scan |
| 57 | LOW | docs | small | - | src/ecs/events.rs:5 | Events doc claims cross-frame durability that does not exist |
| 58 | LOW | api-ergonomics | small | - | src/ecs/world.rs:786 | No public World::has_component::<T>() forces wasteful downcast in existence checks |
| 59 | LOW | performance | medium | - | src/ecs/world.rs:682-687 | query_added and query_changed allocate a Vec<Entity> on every call even when tracking is empty |
| 60 | LOW | robustness | small | - | src/prefab.rs:455-459 | spawn_entity_def silently drops all def.components when SerdeComponentRegistry is absent |
| 61 | LOW | robustness | small | - | src/app/editor/ui/docked.rs:700-720 | add_component_selected stale state silently no-ops Add button after scene reload |
| 62 | LOW | performance | medium | - | src/app/editor.rs:458-460 | draw_pathfinding_overlay clones all Tilemap components every visible frame |
| 63 | LOW | docs | small | - | src/app/window.rs:415 | TouchState stores physical pixel coordinates but swipe threshold and public docs use ambiguous 'screen coordinates' |
| 64 | LOW | robustness | small | - | src/network.rs:599-612 | NetworkSystem silently drops all events when Events<NetworkEvent> is not registered |
| 65 | LOW | api-ergonomics | medium | - | src/network.rs:148-319 | No reconnect API — replacing NetworkClient leaks background thread and closures |
| 66 | LOW | robustness | small | - | src/network.rs:673-681 | RemoteEntities returns stale entity handle after external despawn |
| 67 | LOW | performance | small | - | src/physics/system.rs:177-178 | PhysicsSystem allocates four fresh Vec<(Entity,Entity)> every frame |
| 68 | LOW | performance | small | **BRK** | src/physics/world/tile_collider.rs:212 | SolidTiles::Only performs an O(N) linear scan per tile ID during every sync call |
| 69 | LOW | performance | small | - | src/collision/grid.rs:101-111 | SpatialGrid::rebuild allocates a scratch Vec every frame instead of reusing a field |
| 70 | LOW | robustness | small | - | src/physics/system.rs:163-173 | contact_pairs() pairs are not normalized with ordered_pair, asymmetric to intersection_pairs() |
| 71 | LOW | performance | small | - | src/renderer/shaders/post_process.wgsl:65-66 | Bloom shader calls textureDimensions() per-fragment to compute texel size |
| 72 | LOW | performance | small | - | src/renderer/sprite.rs:358 | Per-frame scratch Vec/HashSet allocations in SpriteRenderer::render have no reuse |
| 73 | LOW | performance | small | - | src/renderer/lighting.rs:384-396 | No frustum pre-filter before nearest-16 light selection wastes light budget on invisible lights |
| 74 | LOW | api-ergonomics | small | - | src/camera.rs:55-57 | zoom_target and zoom_tween_speed are fully private with no read accessor — tween state unobservable |
| 75 | LOW | robustness | small | - | src/camera.rs:167-169 | zoom_to(target, 0.0) silently does nothing — speed=0 is a footgun |
| 76 | LOW | performance | small | - | src/steering.rs:118, | SteeringSystem allocates five Vec<Entity> per frame |
| 77 | LOW | docs | small | - | src/steering.rs:91-98 | Wander direction update uses low-quality deterministic hash with no API-level warning |
| 78 | LOW | api-ergonomics | small | - | src/save.rs:413 | save_versioned / load_migrated always use SaveKey::DEFAULT — no custom-key variant |
| 79 | LOW | feature-gap | small | - | src/ui/localized.rs:104-113 | LocalizationSystem cannot bind to TextInput.placeholder — localized search boxes require manual wiring |
| 80 | LOW | api-ergonomics | small | - | src/ui/slider.rs:65-67 | Slider::set_field("initial_value") does not update the live thumb position |

---

> 원시 감사 데이터(84후보, 검증 verdict 포함)는 세션 워크플로 결과로 보존됨. 본 표의 file:line은 커밋 `42de46c` 기준이며, 수정 착수 전 해당 라인을 재확인할 것(이후 커밋에서 이동 가능). 항목별 상세 PROBLEM/FIX는 §2~§8 본문 참조.
