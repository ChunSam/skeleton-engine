# Next Work — the live backlog

> Status: living document. Derived from `docs/VISION.md` (reset 2026-05-29), under its core loop:
> **a feature is not done until a small, playable example game in `examples/` exercises it in real
> play.** ⚠️ As of 2026-08-19 the `examples/` tree is empty, so **nothing in this engine currently
> meets that bar** — see the top section below.
>
> **This file holds only what is still open.** The completed candidate A–O playable-examples program
> and its release/hardening follow-ups moved to **`docs/PROGRAM_HISTORY.md`** on 2026-08-03 — they
> had grown to 84% of a file named *Next* Work, so the live decisions were buried under 400 lines of
> finished ones.
>
> Session narrative belongs in commit bodies and `docs/CHANGELOG.md`, durable lessons in
> `docs/PATTERNS.md` / `docs/VERIFICATION.md`. What has no other home is the **decision backlog**
> below — and that is exactly what kept getting buried.

## ⚠️ Top of the backlog — rebuild the examples tree (opened 2026-08-19)

**All 22 playable games and ~85 feature demos were deleted on 2026-08-19 at the maintainer's
request, to rebuild a smaller set of feature-test games from scratch.** This is the one open item
that is *not* gated on a trigger, and it outranks everything below it.

What went with them, because it was built on them:

| Deleted | Consequence |
|---|---|
| 11 `<NAME>_SELFTEST` acceptance tests + `scripts/selftests.sh` | the 11 tests are gone; **the runner is back** (phase 0, 2026-08-19) and enforces an empty set |
| 16 `scripts/*_smoke.sh` (12 browser, 4 native) + the `wasm-smokes` CI job | nothing runs engine code in a browser; no render smokes |
| `scripts/build_wasm_examples.sh` + the CI step calling it | an example's wasm path is unbuilt and unchecked |
| `scripts/hot_reload_smoke.sh` + the `DATA_ANIM` / `DATA_PARTICLES` selftests | hot-reload has no coverage |

✅ **Branch protection is clean — re-verified 2026-08-19.** The required set is the seven jobs that
still exist, and `Browser smokes (Chrome + swiftshader)` is not among them, so nothing is waiting on
a check that can never report. Re-check with the API, never with this line:
`gh api repos/ChunSam/skeleton-engine/branches/main/protection --jq '.required_status_checks.contexts'`

⚠️ **Phase 5 puts the trap back.** Restoring the `wasm-smokes` job means re-adding its context to the
required set — and a job added without its context is a check nobody is gated on, which is the
mirror-image failure. Adding a *step* to an existing job (phase 0's runner) needs no protection
change; adding a *job* does.

Nothing was migrated into `tests/`. Before rebuilding a game, read `docs/PROGRAM_HISTORY.md` (what
each one covered and why) and `docs/VERIFICATION.md` § *A skip is not a pass* (the runner shape —
derived, never hardcoded — that this repo already paid for twice).

## Board gate — check this first, every session

Both channels were **empty** as of 2026-08-10 (re-checked; neither has moved):

- `../dungeon-merchant/docs/engine-wishlist.md` — next free **EW-012**, unmoved since 2026-07-27
- `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` — `_None._`, unmoved since 2026-07-14

Check whether they **moved** (`git log -1 --date=short -- <file>`), not whether they look empty.
A filed request preempts everything below.

## Open — engineering

**Three items, all deliberately unscheduled** — each gated on a trigger, none on a decision, and
all three are the standing ones below. **The 2026-08-18 ECS review's efficiency remainder is now
empty**: its last two items closed on 2026-08-19, one by the measurement it was gated on and one by
shipping (v0.152.6). That section is kept below as the record of what measuring did to it. The
follow-up review of that work left nine small items of its own — they have their own section below
and are **not** gated the way these three are. Neither is the 2026-08-19 **render** review, which
added a section of its own after shipping three fixes as v0.152.9.

A backlog this short is still the *expected* state, not a gap to fill: two programs closed in
v0.150.7 and v0.151.1, and the board gate above is empty. Manufacturing work to fill it would be a
new analysis — say so out loud and scope it, rather than letting it arrive as "the backlog said so".
The ECS rows below are **not** that: they came out of a review that shipped twelve fixes across
v0.152.1–v0.152.4, and they are what that review deliberately did not do. The render rows are the
same kind of residue, from a review the user asked for — read them as *what a full read found and
did not fix*, not as a queue that has to be drained.

| Item | State |
|---|---|
| **4th procgen mode** (drunkard's walk) | Unchanged, still the lowest marginal value: the engine cannot fail at it, so nothing is learned. `src/mapgen.rs` already ships three generators over one shared `DungeonMap` (BSP rooms, cellular cave, perfect maze), each with its own example and each guaranteed-connected by a different mechanism. |
| **`add-facade-capability` skill** | n=5 now (the facade + native + wasm + policy-module shape has repeated that many times). Deferred; the next facade capability makes the case by itself. **Building it now would ship a skill with nothing to apply it to** — no facade capability is queued, so do it *alongside* the next one, not before. |
| **Last-seen eviction helper** (`RemoteEntities` #5) | **n=0 as of 2026-08-19 — its one call site was deleted with the examples tree.** It was n=1, gated on a 2nd staleness example (the same bar that held `SnapshotBuffer` until its 2nd call site); the gate is now unreachable until a networked game is rebuilt. Keep the row: the shape below is the useful part, and re-deriving it costs more than reading it. Historical detail — `salvage_run`'s AOI streaming produces **removal-by-omission**: the server never sends a `Bye`, an entity just stops appearing in snapshots, so the client infers eviction from `last_seen` + timeout. Candidate shape (`touch(key, t)` / `expired(now - timeout) -> Vec<K>`) is written up in `docs/REMOTE_ENTITIES_DESIGN.md` § *5th example*, **flagged not built**. Surfaced here 2026-08-10 because that doc was its only home — the four sibling verdicts in the same section all resolved to *keep minimal / zero engine change*, and this is the one that did not. |

### Open — the 2026-08-19 render review's remainder

A full read of the render subsystem (`src/renderer/**` + `src/app/render/**`, 10,549 lines across
41 files, WGSL included) on 2026-08-19. **Three shipped as v0.152.9** — the `interleave_runs` NaN
hang, the draw queues surviving a skipped frame, and the hot-reload sRGB downgrade. **A fourth
shipped as v0.153.1** (the `debug_draw` step-count hang). **Eight more shipped as v0.153.2** — every
row a read or a CPU-side measurement could settle. All twelve are struck through below. The rest is
below, split by what settles it. **None is gated on a decision**; they are ordered by how cheaply
each can be proven, not by how much they are worth.

⚠️ **What is left is exactly what needs hardware**: the `render` job for the UI-primitive
allocations, a docked screenshot for the transition aspect, a windowed drag for the bloom timing.
None can be closed from a headless machine, so the cheap-to-prove ordering has run out — the
remainder is gated on the GPU coverage the gate does not have (see the top-level *not covered* list
in `CLAUDE.md`), not on anyone's willingness.

The efficiency rows are the ones to be careful with. This repo's own habit — see the closed section
below — is that an efficiency claim that has not been measured is a hypothesis, and five of them
reversed under measurement between v0.150.7 and v0.152.5. Two rows here **are** measured and say so;
treat the rest as unproven until the named instrument runs.

| Item | Where | What settles it |
|---|---|---|
| ~~**`Arc::from("")` per untextured sprite per frame.**~~ **DONE v0.153.2 — the row was right, its byte figure was not.** Re-measured with a counting allocator: 1000 allocations / **16,000 bytes requested** per 1000 sprites, all pointers distinct → 0 and 1 after interning in a `OnceLock`. The row's "32 KB" is the allocator's bucket, not the requested layout. ⚠️ Its *justification* also expired — "`Sprite::colored` is common across the examples" cites a tree deleted the same day; the measurement stands on its own, the impact estimate does not. | `src/renderer/sprite/collect.rs:74` | ~~Measured already~~ — re-measured, plus a pointer-identity test |
| ~~**The shaped-text cache keys on `position` where position provably cannot affect layout.**~~ **DONE v0.153.2 — and the fix is bigger than the row asked for.** Keying on the computed `(width, height)` is right, and it turns out `position`, the viewport, `bounds` **and** `anchor` all reach shaping *only* through that pair — `shape_text` is a pure function of `ShapeSpec` and the `FontSystem`. So the key is now `ShapeSpec` field-for-field and those four dropped out entirely. Two wins the row did not name: bounded text survives a resize in the cache, and two anchors landing on the same layout size share one shaping. `cache_key_miss_when_position_differs` moved with it, as predicted. | `src/renderer/text/cache.rs:26`, `src/renderer/text/renderer.rs:406` | ~~Read off the two pure functions~~ — key tests moved, plus a real-font test comparing every glyph at two positions |
| **The UI-primitive path allocates 4 `Vec`s + a `String` per image, per frame.** `sorted_ui_primitives` builds a `Vec<UiPrimitive>` and then `zs`/`keys`/`instances`, and `DrawImage::texture_key()` returns an owned `String`. Every frame with a HUD or a debug draw pays it. The sprite path solved the same problem with scratch fields. | `src/renderer/sprite/ui_primitives.rs:55`, `:149`, `src/renderer/ui.rs:151` | Needs a GPU (`prepare_ui_primitives` takes a device) — the render job, or extract the sort |
| **`BloomRenderer::resize` recompiles the shader and all four pipelines.** `*self = Self::new(…)` rebuilds everything, but only the mip pyramid and its bind groups depend on size. During a live window drag that is a full pipeline rebuild per frame. `PostProcessRenderer::resize` already does the narrow thing. | `src/renderer/bloom.rs:265` | Read; a timing claim needs a windowed drag |
| ~~**The GPU-particle renderer is never torn down.**~~ **DONE v0.153.2 — by stopping the work, not the renderer.** The per-frame cost was the point, and the pass now runs only while there is something to simulate. Tearing the renderer *down* was rejected: its pipelines and buffer are a cache, so a game whose emitters blink off would trade a per-frame cost for a shader recompile. ⚠️ The gate is **not** `has_emitters` — the frame after the last emitter despawns, its particles are still alive. Death happens on the GPU, so liveness is bounded CPU-side by counting the longest uploaded `life` down by the same `dt` the shader uses, which errs only towards "maybe alive". The capacity half is fixed too: a `GpuParticleConfig` change now rebuilds (discarding particles in flight). | `src/app/render/frame.rs:454`, `src/app/render_state.rs:56` | ~~Read; cost needs the render job~~ — read; the *saving* still needs the render job to quantify |
| ~~**One `queue.write_buffer` per new particle.**~~ **DONE v0.153.2.** The row's reasoning held exactly: uploads are grouped into contiguous ring runs, so an emission is one write, or two across a wrap. Four tests, including the control that non-consecutive slots are never merged — merging them would overwrite the particles in between. | `src/app/render/frame.rs:487` | ~~Read~~ — read + 4 tests |
| ~~**The docked transition overlay uses the surface aspect.**~~ **FIXED v0.153.3, NOT YET PROVEN — the row's own gate has not run.** The diagnosis held exactly, and the correct value turned out to be in the same function already (the text passes' target size, now renamed `scene_target_w`/`_h` and documented as "what any pass rendering into `scene_target` must use"). ⚠️ It is still owed a docked capture, and that capture needs a **control**: the bug is only observable when the docked RT's aspect differs from the surface's, so a capture taken at a window size where they coincide passes vacuously. Scoped into the examples-rebuild plan's `rpg_quest_game` phase, with the aspect-difference assertion written into the spec; the fix shipped ahead of it because that phase has no date. Re-open this row if the capture disagrees. | `src/app/render/frame.rs:650` | ~~A docked screenshot~~ — still a docked screenshot, now as a regression guard rather than a discovery |
| ~~**`live_material_entities_scratch` excludes `Hidden`, and two comments above it say it does not.**~~ **DONE v0.153.2 — the comments were right.** Keeping a hidden entity's buffers is the cheaper behaviour and was the stated intent; the code was the half that was wrong. Both sets now come from one pass, split into a free function so a test can reach it — `SpriteRenderer` needs a GPU to construct, so the fix was otherwise unprovable. | `src/renderer/sprite/collect.rs:232` | ~~Read~~ — read + a test over `&World` |
| ~~**`setup_lighting`'s arm order swallows a same-frame cap change.**~~ **DONE v0.153.2.** The exclusive `match` arms became independent steps decided by a pure `lighting_fixups`. The ordering needs no re-checking and the doc says why: `reconfigure` already rebuilds at the new size, `set_max_lights` preserves size and format, `resize` early-returns on a match. Five tests, including the control that an unchanged frame rebuilds nothing. | `src/app/render/post_lighting.rs:120` | ~~Read~~ — read + 5 tests |
| **`load_texture_with_format` ignores `format` on a cache hit** — the cache key has no format, so a path can only ever hold one, silently. The v0.152.9 hot-reload fix depends on that being true, so changing it means revisiting `reload_format`. | `src/renderer/sprite/textures.rs:48` | Read |
| ~~**A non-finite debug-draw coordinate hangs the frame.**~~ **DONE v0.153.1 — and the row named one of three failure modes.** The hang was real and exactly as described. But the same saturating cast sends a **NaN to `0`**, so a NaN never hung — it drew a single garbage quad at NaN coordinates, a second bug the row did not see. And a **finite** `len` is not safe either: `1e18` asks for 9.4e17 iterations without saturating anything. So the guard the row implies — reject non-finite input — is half a fix; the step count is capped as well, which is what makes termination a property of the loop rather than of its input. ⚠️ Also: the finiteness check belongs on the **length**, not on the endpoints. Two *finite* endpoints far enough apart overflow `length()` on their own, since it squares them. | `src/app/render/debug_draw.rs:62` | ~~A unit test (pure function)~~ — six added, eleven existing untouched |
| ~~**The lighting bind-group cache keys on a reference's address.**~~ **DONE v0.153.2 — made local, not removed.** Recreating the intermediate now calls `LightingRenderer::invalidate_bind_group` explicitly, so correctness no longer rests on the standing invariant that every such path also resizes or reconfigures. The pointer check stays as a same-frame fast path, with its doc now saying plainly that the address identifies the caller's *slot*, not the view. Still unreachable today; the change is that it cannot become reachable silently. | `src/renderer/lighting.rs:479` | ~~Read; latent~~ — read; no test (the failure needs a GPU to observe) |
| ~~**`RenderStats::draw_calls` counts only the sprite pass.**~~ **DONE v0.153.2 — scoped, not counted.** Counting the UI pass means adding a parameter to the public `render_ui_primitives_from_slices`, and even then the text pass draws through `glyphon`, whose internal draw count the engine cannot observe **at all** — so no honest frame total is available. The field's doc and the Engine Stats label ("sprite draw calls") now say what the number is. | `src/renderer/sprite/draw.rs:77` | ~~Read~~ — read; doc + label only |
| **Smaller, no home of their own**: `upload_particles` drops an out-of-range write with no log (`src/renderer/gpu_particle.rs:340`); `custom_pipelines` and `rt_cache` are never evicted (`material.rs:15`, `textures.rs:21`); the offscreen camera swap is the remove → call → reinsert pattern v0.152.2 fixed three times elsewhere (`offscreen.rs:57`); nine-slice sub-quads are LOD-culled individually, so `CullConfig::min_pixel_size` can drop a panel's corners while its centre stays (`collect.rs:98`); `Texture::from_rgba_with_format` builds a fresh identical sampler per texture (`texture.rs:217`). | — | Read |
| **Two per-frame allocations found while doing the v0.153.2 batch** — noticed in passing, not part of the original review, and deliberately left alone to keep that change reviewable. (a) `PlainTextCacheKey` owns a `String`, so every plain `DrawText` clones its text once per frame **to build the lookup key**, hit or miss — now the most expensive thing left on a cache hit, since v0.153.2 made hits the common case for moving text. An `Arc<str>` key, or a borrowed-key lookup, removes it. (b) `mat_ids` is a fresh `Vec` per frame in the material collect path; free when no `ShaderMaterial` exists (an empty `collect` does not allocate), so it only bites scenes that use them. Both are the scratch-field pattern the repo already applies elsewhere. | `src/renderer/text/renderer.rs:401`, `src/renderer/sprite/collect.rs:263` | `tests/per_frame_alloc.rs` cannot reach either (both need a GPU renderer) — measure as v0.153.2 did, with a standalone counting allocator |

### Open — the 2026-08-19 follow-up review's remainder

A review of everything v0.152.1–v0.152.7 changed (`25c49e5..13ce809`; 88 src lines, 484 test lines)
produced 13 findings. **Two shipped as v0.152.8** — the `move_entity` drop-order unwind hole and the
editor's `EntitySortMode::Insertion` that no longer meant insertion. **One was a false positive**
(the row above). The **nine below were deliberately not done**: none changes what correct code does,
and bundling them would have buried the two that do. Each is one edit; take them opportunistically
when next in the file, not as a program.

| Item | Where |
|---|---|
| `with_resource_mut`'s two doc guarantees contradict each other on replace-then-panic (the replacement wins; the entry value's mutations are dropped). Untested combination. Also worth stating: a deliberate `remove_resource::<R>()` inside `f` is undone by the restore. | `src/ecs/world/resources.rs:41` |
| The self-label warning cannot tell the `.label(X).after(X)` typo from the shared-label barrier idiom — which the same release's own test blesses. `by_label` is built 30 lines above, so `by_label[l].len() > 1` separates them for free. | `src/ecs/schedule.rs:104` |
| A new test was inserted mid-doc-comment: `dangling_after_label_creates_no_constraint` has no doc, and `self_referencing_label_creates_no_constraint` carries four paragraphs about the dangling case. | `src/ecs/schedule.rs:290` |
| The take → put-back cost claim cites `tests/per_frame_alloc.rs`, which has no coverage of that path (`grep -c 'take_component' → 0`). The accounting is right; the citation cannot settle it. | `src/ecs/world/components.rs:97` |
| Both archetype-transition ratio tests divide a fixed `spawn` cost by `width`, so part of `wide < narrow` is structural. Solving the shipped numbers: ~0.13 of the 0.226 allowance is spent before `move_entity` is measured, and the bytes variant would pass if per-component bytes quadrupled. They do still catch width-scaling (sabotage-checked); subtracting a spawn-only baseline would make them mean what they say. | `tests/per_frame_alloc.rs:776`, `:832` |
| `debug_assert!(extracted.is_empty(), "scratch left dirty by a previous call")` is unreachable — the restore is unconditionally preceded by `clear()`, and a panic leaves the field as the empty `Vec` the `mem::take` put there. It reads as a re-entrancy guard it cannot be. | `src/ecs/world.rs:252` |
| `entity_list.sort_unstable_by_key(|e| e.index())` is copy-pasted at three call sites and prescribed by a fourth doc comment. n=4 now; a `World::entities_sorted()` or an editor-local helper would carry the policy the type system currently does not. | `src/app/editor/ui/mod.rs:250`, `:307`, `docked/save_load.rs:122` |
| `query3_mut`'s assert prints all three type names but not which pair collided — the eyeballing `query2_mut`'s message was written to remove. The check is already pairwise. | `src/ecs/world/queries.rs:123` |
| The batch's one genuinely breaking change (`query2_mut::<A, A>()` now panics) is documented in full prose but without the ⚠️ marker the file uses 20 times elsewhere — including twice in v0.152.6 for softer caveats. Presentation only; the `assert` is the right call. | `docs/CHANGELOG.md` 0.152.4 |

**One finding was rejected outright and is not in that table**: `query_added` / `query_changed` were
read as allocating a `Vec` on every call because `clear_change_tracking` retains keys with emptied
sets, so the `is_empty()` fast path rarely fires. The scan is real; the allocation is not — an empty
filtered iterator collects with a lower bound of 0 and never touches the heap. Only the word
"immediately" in that doc is overstated, and it predates this diff.

### Closed 2026-08-19 — the 2026-08-18 ECS review's efficiency remainder

A full read of `src/ecs` (2,626 lines, 14 files) on 2026-08-18 produced 15 findings. **Twelve
shipped** across v0.152.1–v0.152.4 — the take → put-back change-tracking bug, three panic-unsafe
remove → call → reinsert pairs, two `HashMap`-seed determinism leaks, and four silent failures made
loud. **One was a false positive** (see the closed row below). Three were the remainder, and they were
remainder *by decision*: every one is an efficiency claim, and this repo's own habit —
`tests/per_frame_alloc.rs`, the v0.151.1 debug-draw row — is that an efficiency claim ships with a
number or not at all. **All three now have their number, and all three are closed** — one shipped
as a fix (v0.152.5), one closed by the measurement it was gated on, one shipped as a cleanup
(v0.152.6). Nothing in this section is open.

**One of the three closed the same day (v0.152.5), and this row's own reading of it was wrong.** It
said the fix was `mem::take` on `move_entity`'s two `type_set` clones, "which would leave 5". The
clones were real but they were the *constant* half; the half that made building an entity O(N²) was
the fresh `HashMap` of extracted components, which grows with the entity's width. Measuring first —
which is what the row's own gate demanded — showed per-component cost climbing 5.01 → 5.75 → 6.50
as an entity widened, and the finished fix landed at **1.38–1.51, flat**. Writing the test first is
what turned a guess into a number.

**Both of the remaining two were measured on 2026-08-19, and the archetype row's stated mechanism
was wrong as well** — a third consecutive reversal in this section, which is the point of the
advisory rule in `CLAUDE.md`. Instrumentation and the `survivor` input script are kept in
`.claude/instrumentation.patch` and `.claude/survivor_play.ron` (gitignored); the tree was returned
byte-identical to HEAD afterwards.

| Item | State |
|---|---|
| **Empty archetypes are never reclaimed** | ✅ **MEASURED — do neither. Recommend closing.** The gate this row named ("nobody has counted archetypes in a real game") is now closed: all 22 game examples were run headlessly with a per-frame archetype dump. **The Vec is bounded and saturates.** `survivor` — driven through a real 1800-frame session by an input script (invulnerability on, `B` waves every 90 frames) rather than the 3.8 s death an unscripted capture gets — reaches **34 archetypes by frame ~150 and never gains another**, while its entities grow 2 → 665. `salvage_run` (against its live server, 109 streamed entities) reaches **4**. No other game exceeds 20. Nothing removes an entry, but the reachable set is the distinct signature *prefixes the game's own spawn code can produce*, which is a property of the code, not of runtime. **The row's mechanism was wrong.** The cost is not "a binary search across the whole Vec": a *non-matching* empty archetype is rejected by that search in a few ns. Only an empty archetype whose signature still **matches** costs anything — it passes the filter and pays two `HashMap<TypeId, _>` column lookups in the `flat_map` body to iterate zero entities, ≈**19 ns** each. `survivor` steady state: **115 filter passes per frame, 98 of them empty (85%)** → ≈**1.9 µs/frame**, **0.57%** of a 326 µs `App::update`. A/B with the one-line guard applied to all 14 sites: **328.2 µs guarded vs 326.3 µs baseline** (3 release runs each) — no measurable change, the ±1.4% run-to-run spread swamps it. ⚠️ An isolated micro-bench *did* show −17%, and it overstated the case: every prefix in that synthetic world contained the queried pair, so all 18 empty archetypes matched. Real games do not have that shape. **Do not reopen without a fork that actually has hundreds of entity kinds** — the scan is linear in archetype count, so a different regime would need re-measuring, but no game here is near it. |
| **`query2`/`query3`/`query4`/`query_opt2` index where their neighbours zip** | ✅ **CLOSED 2026-08-19 (v0.152.6)** — `query2`/`query3`/`query4` now zip, so the file has one spelling of one operation. Measured against a copy of the exact spelling it replaced: **2131 → 1748 ns per 650-entity pass, −18%** (0.59 ns/entity), reproducing to ±0.2% across runs. ⚠️ **The percentage is not portable**: an earlier four-variant harness put the same change at −8.5%, and since runs within each harness agree to ±0.2%, the spread is code layout between harnesses, not noise. Direction and per-entity size are what generalise. It shipped as a cleanup — nobody has shown it moving `App::update`. `query_opt2` keeps its index by design (`B` is optional, so its column has no per-element iterator to zip); its mandatory pair zips anyway. ⚠️ Indexing panicked on an `entities`/column length desync where zip silently yields the shorter; six sites already had zip's behaviour, so this made the file consistent rather than adding the exposure. **That follow-up shipped the same day (v0.152.7)**: `Archetype::debug_assert_columns_aligned` now guards all **fourteen** iteration sites — the row said ten and missed `parallel.rs`'s four, which zip the same data through rayon — and ships with a sabotage-verified test that makes it fire. |

### Closed — do not reopen without new information

⚠️ **All three of these sat in the table above marked closed**, which is the exact burial this file
exists to prevent: a reader scanning *Open — engineering* for work met 3 finished rows out of 5.
Kept verbatim rather than trimmed, because each carries a lesson whose only home is this row.

| Item | Verdict |
|---|---|
| **`<NAME>_SELFTEST` coverage** | **DONE — 10 of 21**, and the one real gap is now closed (`beat_crawler`, `survivor`, `data_anim`, `data_particles`, `salvage_run`, `predict_shooter`, `orbital_dodger`, `coin_race`, + `settings_menu` and `scene_flow` on 2026-08-06). The remaining 11 games' headline features are all visible in a screenshot (`sokoban`, `platformer`, `maze_escape`, `dig_quest`, `shooter`, `lit_dungeon`, `multi_terrain`, `tile_paint`, `ui_layout_editor`, `stat_editor_game`, `script_steering`), so chasing the number past 10 is effort against failures that are already visible. **Do not reopen this as a coverage target.** Durable findings from the four networked ones are in `docs/MODULE_MAP.md`'s `src/network.rs` row; the two that generalise beyond networking: **`InputState` has no public press setter**, so held input comes from `InputScript` (the `ENGINE_INPUT` replay path), keeping the real input read under test; and **assert an invariant, not an end state**, when a background process (a coin respawner, an entity spawner) can add to what you are counting. |
| **`DialogueChoice.cond` cannot express a conjunction** | ✅ **CLOSED 2026-08-10 (v0.152.0)** — `cond_all` / `cond_any` ship alongside `cond`, and `rpg_quest`'s `can_buy_lantern` workaround is deleted. ⚠️ **This row's own fix shape was impossible**, which is why it is worth reading twice: it specified `All([…])`/`Any([…])` *variants* on `DialogueCond`, "additive if the single-cond form still parses" — and in RON 0.8 those are mutually exclusive. An externally tagged enum rewrites every existing `cond: (var: …)` into `cond: Cmp((var: …))`; `#[serde(untagged)]`, the usual way out, cannot even re-read its own serialized output. **One throwaway test settled it in a minute; no amount of reading would have.** Durable homes: `docs/PATTERNS.md` § *Extend a type that is authored in RON*, `docs/CHANGELOG.md` 0.152.0. **Do not reopen for arbitrary nesting** — `a && (b || c)` is expressible, deeper trees need a helper var, and no example has asked. |
| **`System::name()`'s "anonymous" fallback** | ❌ **FALSE POSITIVE — the doc is correct, do not "fix" it.** The 2026-08-18 ECS review flagged `src/ecs/system.rs:10` (*An empty string displays as "anonymous"*) as documenting a fallback that does not exist. It does exist, in `src/app/editor/ui/mod.rs` as `tr("anonymous", "익명")` — and that is the only profiler renderer in the crate, so the doc holds for every path. The finding survived because the grep that "proved" absence was piped through `head` and truncated before reaching it. Recorded here because the next reviewer will grep for the bare literal and reach the same wrong conclusion: `rg -c 'anonymous' src/` (a count, not a listing) settles it. Same trap as `docs/VERIFICATION.md`'s trailing-`tail` rule, in a different tool. |
| **`take_component` → transfer → fresh `add_component`** | ❌ **FALSE POSITIVE — the classification is correct, do not "fix" it.** The 2026-08-19 follow-up review of v0.152.1–v0.152.7 read `add_component`'s `put_back_this_tick` check (`src/ecs/world/components.rs:157`) as mis-classifying a *genuinely new* component when the taken value had been handed to a different entity: `query_added` would miss it. It does miss it, and that is right. `query_added` means **first added this tick**, and the entity carried the component when the tick began — so a plain in-place replace reports `changed` too, and the take path agreeing with it is the consistent outcome, not the bug. Reversed by running it, not by re-reading it: a 30-line probe printed `added=[chest] changed=[hero]` and, in the same run, `replace: added=0 changed=1`. The one genuine asymmetry it surfaced is `remove_component` → `add_component`, which reports **added** — deliberate, since a removal states the component is gone. All three paths are pinned in `a_mid_tick_replacement_reports_changed_but_a_removal_then_add_reports_added` and spelled out on `take_component` and `query_added`, because reading any one path alone invites the same wrong conclusion again. |
| **2026-08-07 analysis §10** | ✅ **CLOSED 2026-08-10 (v0.151.1). The whole program is finished.** Step 0 of the plan never ran, and nothing recorded that until 2026-08-08; seven shipped across v0.150.1–v0.150.4, two docs/test hygiene items closed on 2026-08-09, the unmeasured v0.150.0 fixes were all measured by v0.150.7, and the last item — `src/app/render/debug_draw.rs:34` — closed in v0.151.1. Nothing from this analysis is open. **Do not reopen it as a source of work**; a new pass would be a new analysis. What it leaves behind is `tests/per_frame_alloc.rs` and the two habits in `docs/PATTERNS.md` / `docs/VERIFICATION.md`. |

### The 2026-08-07 analysis's unverified candidates — the closed record

> **Nothing below is open** as of 2026-08-10 (v0.151.1). It is kept as the record of *how* the
> program closed, because three of its readings were reversed by measurement and one by re-deriving
> a problem the row had written off. Read it for the habits, not for work.

`plans/2026-08-07-analysis-followup.md` had **fourteen** steps, 0 through 13. Steps 1–13 all
shipped (#438–#450, v0.145.1 → v0.150.0). **Step 0 — re-running the 33 verification agents that
died on a session limit, so §10's candidates would get the adversarial pass §1–§8 got — did not
run**, and it left no trace anywhere: `docs/CODE_ANALYSIS_2026-08-07.md` §10 still says only "worth
verifying in a follow-up session", which is not a backlog. That is the burial this file exists to
prevent, so it is written down here instead.

§10 was hand-checked against the tree on **2026-08-08** rather than re-run. Its 21 bullets split
**10 / 1 / 1 / 9**:

- **10 are already closed**, four of them by the very steps that ran while §10 sat unverified —
  `serde_registry` duplicate names now warn and name both types (#442), `audio_wasm::is_channel_playing`
  answers for positional channels (#447), the GPU-particle verification blind spot is closed by
  `tests/render.rs` `gpu_particles_accumulate_across_frames` (#438), and `PATTERNS.md`'s 20-vs-22
  disagreement was amended (#444). The other six are the drift items #450 fixed: `wasm_smoke.sh` into
  CI, `selftests.sh`'s stale counts, `network/system.rs`'s non-existent `world.register_event`,
  this file's seven-vs-eight, `MODULE_MAP`'s `dig_quest`/`tile_paint` target names, and `ci.yml:176`.
- **1 became the process item above** — `ci.yml:7`, "`wasm-smokes` is not a required check". #450
  corrected the false *claim* in three docs; the *decision* it exposed is still open.
- **1 is a false positive** — `src/ron_registry.rs:11`'s "nobody registers the path with the file
  watcher". Hot reload *is* wired: `App::register_hot_reloadable` → `forward_hot_reload` →
  `HotReloadable::reload_path`, and `particle/config_set.rs` has a test pinning the canonical-path
  match. Do not re-open it.
- **9 survived, and all 9 are now closed** — the touch letterbox map and the wasm asset failure
  hook (v0.150.1), the wasm pre-open send drop and the untested wasm event queue (v0.150.2), the
  three per-frame allocation candidates (v0.150.4, measured first), and the last two docs/test
  hygiene items on 2026-08-09 (no version bump; the table below records what the second one found).

⚠️ §10's header says **23**; the section lists **21** bullets. The header is the wrong number —
count the bullets, and do not propagate either figure without counting.

| # | Where | What | Confidence |
|---|---|---|---|
| 1 | `src/input/gamepad.rs` | ~~`GamepadState` is permanently unresponsive on wasm and the type's doc comment says nothing about it~~ **DONE 2026-08-09** — the doc comment now carries a *Native only* section naming every method that stays `false` / `None` / `0.0` on wasm, and says to give web builds a keyboard or touch path. No behaviour change, no version bump. | Confirmed (docs) |
| 2 | `src/renderer/texture.rs:293` | ~~`decode_valid_png_returns_rgba` is vacuous~~ **DONE 2026-08-09 — and the vacuity was hiding a broken fixture.** Replacing `let _ = …` with a real assertion showed the "1×1 red pixel PNG (minimal valid PNG)" had a **wrong IDAT CRC** and had never decoded once, so `decode_image_bytes`' success path had *zero* coverage — only the failure path was tested. The fixture is regenerated (CRCs verified, generator command in the test), and the test now asserts dimensions and the RGBA pixel. Both assertions sabotage-verified red and reverted byte-identical. No behaviour change, no version bump. | **Confirmed** |

The three per-frame allocation candidates shipped in v0.150.4, **measured rather than read** —
all three claims were real (401 / 190 / 1 allocations per steady-state frame). Two went to zero;
`ParticleSystem` was deliberately left at one bulk allocation because the proposed `query3_mut`
fix would stop a hand-spawned `Particle` from ageing, and a particle that never ages never
despawns. The reasoning is in the test that guards it.

⚠️ **Measure before adding to this list.** `tests/per_frame_alloc.rs` exists so a per-frame
allocation claim can be settled in one command instead of another reading of the code. v0.150.5
pointed it at v0.150.0's six "fixed" and three "not addressed" claims and **reversed two of them**:
`HierarchySystem` was still allocating 200×/frame (the scratch buffers were converted; the
`add_component` write at the end of the loop was not), and `LayoutSystem` — named as an
unaddressed hot spot — measures **zero**. Reading got both backwards. It also turned up an
ECS-wide cost nobody had listed at all: `clear_change_tracking` dropped a `HashSet` per changed
entity every frame.

**The three v0.150.0 named as "not addressed" are now accounted for**, and this is where they
should have been recorded in the first place rather than only in a CHANGELOG entry — the same
burial that hid step 0:

| Item | Verdict |
|---|---|
| `src/ui/panel.rs` `LayoutSystem` | **False positive** — measures 0 over 50 panels × 8 children. Do not reopen without a measurement that disagrees. |
| `src/app/assets.rs:262` | **FIXED v0.150.6 — and it was never a measurement problem.** This row asked for a fixture (in-crate unit test or the render job) to *measure* a `pub(crate)` method the harness cannot see. But an allocation you can read off the signature does not need measuring: `image_assets_for_gpu` returned `Vec<(String, ImageAsset)>` from a per-frame call site over `Arc<str>` keys. Yielding `(&str, &ImageAsset)` deletes it in four lines, and leaves nothing to measure. Pinned in-crate by **identity** (`ptr::eq` on the key, `Arc::ptr_eq` on the pixels) — `assert_eq!` on the strings would have passed for a fresh `String`. ⚠️ **Ask "can I just delete this?" before "how do I measure this?"** — the harness's reach is not the only route to a claim. |
| `src/app/render/debug_draw.rs:34` | **FIXED v0.151.1 — and both of this row's own readings of it were wrong.** It was mis-filed as an allocation claim (it is draw-call volume, so `per_frame_alloc.rs` was the wrong instrument — that part this row got right), and then written off as not implementable because `DrawRect` has no rotation. Rotation is only needed for **diagonals**. An axis-aligned segment collapses to one quad with no renderer change, because `push_line`'s step is always `<= thickness`, so the dots' union *is* the rect — an identity. `centered_text`'s three guide columns went 825 → 3 quads/frame, a `Cross` 30 → 2, with a byte-identical capture. ⚠️ **"not implementable" was a claim about the *suggested fix*, not about the problem** — the row never asked whether a different fix existed, and the answer was three lines below in the same file, where the `Rect` arm already drew its four edges as four quads. |

✅ **All six of v0.150.0's fixes are now measured** (v0.150.7 closed the last four). Final tally
across v0.150.4 → v0.150.7: of the six claims, **three were wrong** — `HierarchySystem` and
`LocalizationSystem` never stopped allocating and were fixed properly, and `TilemapSystem` was
still allocating twice per frame. Reading got half of them backwards.

| Claim | Verdict |
|---|---|
| `AnimEffectSystem` — bus snapshot before the registry clone | **Real.** Idle frame 0; reverting the order restores 129 allocations / 9,150 B |
| `ZoneEffectSystem` — same shape | **Real.** Idle frame 0; 129 / 9,662 B reverted |
| `DialogueSystem` — `LocaleResource` clone guarded on a box with keys | **Real.** Frame cost independent of table size; unguarded it goes 8 → 809 |
| `TilemapSystem` (idle, populated) | **Was still allocating** — 2/frame, fixed in v0.150.7 |
| `HierarchySystem` / `LocalizationSystem` | Were still allocating; fixed in v0.150.5 / v0.150.4 |

⚠️ **The tilemap one is the finding worth carrying, and it is not about tilemaps.** The test that
was supposed to guard the grid clone — `tilemap_system_steady_state_does_not_allocate`, shipped in
v0.150.4 — builds a `World` with **no `Tilemap` in it**. `run` collects an empty entity list and
returns, so it never reaches the clone. It passed for four releases, and the v0.150.5 CHANGELOG
reported v0.150.0's tilemap fix "confirmed" on the strength of it. **A green must-be-zero assertion
is two claims glued together — *the code is clean* and *the code ran* — and only the second is cheap
to check.** Every fixture in that file now carries a positive control that drives the guarded path
and requires a non-zero reading; the rule is written up in `docs/VERIFICATION.md` § *a fixture that
omits the subject reads clean*, next to #456's vacuous PNG assert, which is the same family.

The two docs/test hygiene items below were closed on 2026-08-09 with no version bump — neither
changed behaviour. ⚠️ One of them was **not** the bottom of the barrel it was filed as: the
vacuous PNG test was hiding a fixture that had never decoded. **"It only changes a test" is not
the same as "it cannot find anything"** — a check that asserts nothing tells you nothing about the
check itself, and that is exactly where rot hides.

**Both follow-ups are closed** (v0.150.3). The gap was that the wasm halves of v0.150.1 and
v0.150.2 were compile-verified only, because nothing drove them — a 404 was never requested and a
pre-open send was never made. `examples/wasm_failpaths` now does both on purpose and
`scripts/wasm_failpaths_smoke.sh` reads the verdict; it gates in the `wasm-smokes` job. It is
sabotage-verified in both directions, each half reddening only for its own defect.

⚠️ **The standing lesson, which outlived the two items:** every other browser smoke passes when
nothing goes wrong, so a *failure* handler can be entirely broken with every check still green.
Two shipped that way. When adding a check, ask what it does when the thing it guards is removed —
and if a new failure path gets a handler, it belongs in `wasm_failpaths`, not in a new smoke.

## Open — process

Nothing open. **The required-check question closed on 2026-08-08**: `Browser smokes (Chrome +
swiftshader)` is now the **eighth** required context, so the only automated check that exercises
the wasm WebSocket path — and, since v0.150.3, the only one that asserts a *failure* path — can
actually block a merge. Verified against the branch-protection API before and after: the other
settings (`strict`, `enforce_admins`, force-push and deletion bans) are byte-identical, only the
context list changed. Re-read the real list rather than trusting this paragraph:
`gh api repos/ChunSam/skeleton-engine/branches/main/protection --jq '.required_status_checks.contexts'`

⚠️ **Expect ~8 min to merge, not ~4.** That job is the slowest in CI and is now on the critical
path. Reverting is one command (`-X DELETE` on the same endpoint) if the cost stops being worth it.
Measured on the two PRs that followed: 5 m 33 s and 5 m 37 s for that job against 4 m 3 s and
3 m 53 s for `Test (native)`, so it is the critical path but the ~8 min estimate was pessimistic.

⚠️ **A repo-settings change leaves no trace in the tree, so this row went stale invisibly.** It was
closed here on 2026-08-08, and the copy on `main` still said "seven contexts, and the browser-smokes
job is **not** among them" until #456 on 2026-08-09 — which found it only by running the command
this row had itself recorded. #456 also mis-dated the closure to the day it was noticed; the
decision was made on 2026-08-08, and this branch is the record of it.

The three items that used to live here — the `main`-push hook, the oversized skills, and the
required-check decision above — closed on 2026-08-04, 2026-08-04, and 2026-08-08.

## Noted — not scheduled

- **Every doc dated before 2026-06-17 cites a version *higher* than today's, and none of them is
  wrong.** The project ran a `1.0.0` → `10.7.0` SemVer line from 2026-05-26 to 2026-06-16 (ten
  majors in three weeks), then **reset to `0.11.0` on 2026-06-17** — `docs/CHANGELOG.md` § 0.11.0,
  *"Version line reset: pre-1.0"*, no code changes, full prior history preserved below it and in git
  tags. So `## 4.3.0` and `## 10.7.0` are real CHANGELOG entries that are **older** than `0.152.0`,
  and a pre-reset doc citing `v8.11.0` is not ahead of `main`. Recorded so the next person who hits
  one does not "correct" a real version into a wrong one — #464 came within a command of doing
  exactly that. **Do not go marker-ify the other pre-reset docs — they are already covered**, and
  this was checked rather than assumed. Of the **11** tracked docs last touched before the reset,
  **6** carry the date in the filename (`CODE_ANALYSIS_2026-06-16.md` and friends) and the other 5
  open with an explicit status line: `ROADMAP.md` *historical roadmap*, `ENTITY_GENERATION_V2_PLAN.md`
  *Implemented in v2.0.0*, `CODE_ANALYSIS.md` *Generated 2026-06-05*, `REMOTE_ENTITIES_DESIGN.md`
  *minimal helper shipped … deliberately deferred*, while `SKELETAL.md` is a feature reference with
  no status to go stale. `docs/HANDOFF.md` was the only one carrying **no marker of any kind**, which
  is why it alone needed one. (A first draft of this bullet claimed the others self-mark *by
  filename*; 5 of 11 do not. The conclusion held, the reason did not — and only listing them showed
  which.)

- **CLOSED by removal, not by diagnosis: the native job's 130 s swing was `cargo build --examples`.**
  Kept as a record because three successive versions of this entry got the *cause* wrong while the
  *measurement* was right each time, and the shape of that mistake is worth more than the finding.
  - **What it actually was.** The step named `<NAME>_SELFTEST` swung 179–308 s, so all three
    versions hunted a flaky selftest. Splitting the step by its own log timestamps ended it: the
    9 selftests run in **15 s, rock-stable**, and every second of the swing was the build in front
    of them — 142 example targets compiled to run 9. v0.143.18 narrowed that to 14, and **the step
    is now 26 s** with the variance gone. Nothing was diagnosed; the cause was deleted.
  - **The `nproc` canary (v0.143.16) is now moot.** It was added to test a runner-core-count
    hypothesis for a step that no longer dominates. It costs ~1 s and still records real headroom
    numbers, so it stays — but **do not resume that investigation**; the question it was asked to
    settle no longer has stakes.
  - **The transferable lesson: a step's name is not its contents — and a job total is not a step
    measurement.** Two entries pointed at the networked selftests' socket work as a suspected
    timeout on nothing but the step label, and a third read three samples as bimodal before a
    fourth refuted it. Both halves are the same mistake: reading a label or an aggregate where a
    measurement was available. What finally worked was the cheapest thing on hand the whole time —
    per-line timestamps already in the CI log. **Split the step before theorising about it.**

- **The local verify-gate hook's two deliberate residuals** (fixed 2026-08-03, `.claude/` is
  gitignored so this is the only tracked record). It no longer over-matches prose, because it
  ignores everything from the first `<<` onward and requires a delete at a **command position**.
  The cost: a fusion written *after* a heredoc terminator is no longer seen (over-matching was the
  costlier failure), and an inline `-m` message containing a literal command-position delete
  alongside the gate name still trips it — **put that text in a file** rather than fighting the hook.

- **The rest of the `.claude/` inventory** (gitignored, so these lines are the only tracked record
  that any of it exists; rolled off *Recently closed* on 2026-08-05 but kept here for that reason).
  Two more hooks in `.claude/settings.local.json`, both proven to fire by sabotage and checked
  against real commands for false positives: **`git commit` is denied while any `*.sh` in the index
  is not `100755`** (the trap `core.fileMode = false` hides — fixed repo-wide in v0.135.2,
  reintroduced twice in v0.143.4, re-fixed in v0.143.14), and **`main`-push blocking** with
  `--delete` exempt so remote-branch cleanup still works (a branch named `maintenance-branch` does
  not trip the matcher). Skills: `handoff`, `wrap` and `example-selftest` all carry their detail in
  `references/` rather than the body. **Do not record their sizes here** — that number was wrong
  twice in a row in opposite directions (`wc -c` bytes against a character guideline, then a
  correct `wc -m` that was already stale at merge). The durable form is the command:
  `for f in ~/.claude/skills/*/SKILL.md; do wc -m "$f"; done`.

- ~~**Eight directory-based examples silently drop out of `cargo package`.**~~ **Moot 2026-08-19** —
  the examples are deleted and the `examples/**` entries are out of `include`. The lesson survives
  the subject: `include` globs are per-level, so `examples/*.rs` never matched `examples/*/*.rs`,
  and a skipped package target is a *warning*, so CI stayed green over it for months. If a rebuilt
  example is ever meant to ship in the package, glob it explicitly and check `cargo package
  --locked --list` actually contains it.

## Known-unfalsifiable checks — do not mistake these for guarantees

- ~~**`BEAT_CRAWLER_SELFTEST` exit `8`**~~ **deleted 2026-08-19 with the example.** Kept as a
  worked example of the failure mode this section is for: the check ("the two meters are not
  independent") **could not fail on native**, because each meter taps its own channel and the
  spectrum read never sees the mixer output — verified by firing the bass-heavy soundtrack as the
  impact clip and measuring no change at all. It was a tripwire for the **wasm** topology, where
  several sources share one `AnalyserNode`. Ask of every check you rebuild: *on the platform I am
  actually running it, can this assertion fail at all?*

## Standing risks

Context for judging new work — not to-dos. Anything here that becomes actionable belongs in
**Open — engineering** instead; that is where `<NAME>_SELFTEST` coverage went on 2026-08-03.

- **NO audio of any kind is under CI as of 2026-08-19.** v0.143.10 established that **native**
  (rodio/ALSA) audio stays outside CI — that part is unchanged and is what the rest of this bullet
  is about. v0.143.17 had put **Web Audio** under gate — `wasm_audio` (38/38) and `audio_reactive`
  (`rms=0.643`, bands `low=9.41` / `high=0.00` on a 110 Hz tone, real spectral discrimination) both
  passed in CI because Chrome renders the graph in software with no hardware device — but those
  were example-driven browser smokes and were deleted with the examples tree, along with the
  `wasm-smokes` job. **The distinction is still worth keeping** ("audio cannot be tested in CI" is
  false about the *browser* half and true about the native half), and rebuilding the browser audio
  smoke is the cheapest way to get a gated audio claim back. Five CI runs
  tried a PulseAudio null sink (default and at 30 ms latency) and ALSA `snd-dummy`; the full table is
  in `docs/VERIFICATION.md`. Summary: a null sink *does* let rodio open a device and `beat_crawler`'s
  audio chain passes on CI, but it delivers samples in bursts, so the meters with sub-second
  deadlines read silence. `snd-dummy` does not exist on the runner kernel. **Do not re-litigate
  without new information** — a runner image with a real or dummy ALSA card would be new
  information; another sink tweak is not. `SKELETON_REQUIRE_AUDIO=1` exists so a *local* run can
  prove its audio checks ran rather than skipped. **Dead from v0.153.0 until phase 0 restored it on
  2026-08-19** — only `scripts/selftests.sh` has ever read it, so it lives and dies with that file. ⚠️ **`scripts/selftests.sh`'s own header claimed
  the opposite** ("CI provisions a PulseAudio null sink") from v0.143.10 until #426 on 2026-08-05 —
  the sentence was written for the null-sink experiment and survived its revert *in the same
  commit*. `ci.yml` and `docs/VERIFICATION.md` were right the whole time; only the file a reader
  actually opens was wrong. **When an experiment is reverted, grep for prose that described it** —
  the revert diff will not show you the comment three files away.
- **5 of the 16 `scripts/*_smoke.sh` stay local, deliberately.** The other **11 run in CI**: 4 native
  in v0.143.11, 5 self-verdicting browser ones in v0.143.17, `wasm_smoke.sh` in #450 — which had
  been counted among the byte-size-only ones and was not; it self-verdicts, and it is the only
  automated exercise of the wasm WebSocket *success* path — and `wasm_failpaths_smoke.sh` in
  v0.150.3, the only one that asserts a **failure** path. The remaining 5 (`centered_text`,
  `embedded_atlas`, `embedded_image`, `game_feel_web`, `hdr_web`) assert only byte sizes and are
  documented as eyeball-it — a green run would prove nothing. Reopen only if one gains a real
  assertion.
  ✅ **"run in CI" now *is* "gate"** for the browser six: their job became a required check on
  2026-08-08 (see *Open — process*). This line said the opposite until #456 on 2026-08-09. Count
  before quoting a number here; it has now been wrong twice:
  `grep -cE '^\s*[^#]*scripts/[a-z_]*_smoke\.sh' .github/workflows/ci.yml`
- **A headless capture cannot photograph a meter** — fixed dt, no wall clock. Three sessions have
  now reached for `ENGINE_CAPTURE` before remembering this.

---

## Recently closed

> **Roll-off rule:** an entry stays here for **one session**, then goes. Its durable home is
> `docs/CHANGELOG.md` (what shipped) or `docs/PATTERNS.md` / `docs/VERIFICATION.md` (what was
> learned). Without this rule the section regrows the history that was just split out.

Closed 2026-08-18 — **a full read of `src/ecs` (2,626 lines, 14 files) produced 15 findings; 12
shipped as four PATCH releases** (v0.152.1–v0.152.4, #466/#468/#469/#470), one was a false positive
(recorded in *Closed — do not reopen*), and three efficiency items stayed open under *the
2026-08-18 ECS review's efficiency remainder* above. A fifth PR (#467) fixed the CI that the work
ran into, and a seventh registered this backlog. **v0.152.5 then closed the first of the three
remainder items** — `move_entity`'s allocations, where writing the gating test first proved the
row's own diagnosis half wrong and the finished fix cut an 8-component entity from 52.03
allocations / 5,160 bytes to 11.03 / 1,076. Detail is in `docs/CHANGELOG.md` 0.152.1–0.152.5 and
the commit bodies. Three things worth carrying:

- ⚠️ **A conflict resolution is a tree nothing has ever verified.** #469's merge with `main` left
  `tests.rs` uncompilable — git's auto-resolution put its `=======` boundary *inside* a test
  function, so the "resolved" file had an unclosed delimiter. Hand-patching that is how a resolution
  quietly loses a test; the file was rebuilt from both sides instead, after checking each was a pure
  append (`head -n <base-len> <branch> | diff - <base>`). Then the gate was re-run: both branches
  were green *before* the merge, and neither of those greens covers the tree the merge produced.
- **The truncated-output trap has a second form, and it produced a false review finding.**
  `rg 'anonymous' src/ | head` cut off before the one line that disproved the finding, so "this
  fallback does not exist" was reported and was wrong. Landed in `docs/VERIFICATION.md`
  § *Searching so the result means something* — with the `#464` concept-grep habit, which had been
  sitting in this section with no durable home to roll off to.
- **A CI comment that tells you to distrust it is telling the truth.** `ci.yml`'s header said the
  required-check set was seven jobs excluding the browser smokes, then said "read the real list
  rather than this comment — it is the thing that drifts". It had drifted: the API says eight,
  browser smokes included, which is exactly why a wedged smoke job blocked the merge outright.
  Checked with `gh api repos/…/branches/main/protection --jq '.required_status_checks.contexts'`.

Rolled off this session, having served theirs: **the frozen `docs/HANDOFF.md`** (#464, docs-only).
Its two lessons now have homes: the version-line note under *Noted — not scheduled* (which it
already had), and "grep for the concept, not the file you were editing", moved to
`docs/VERIFICATION.md` this session — it had none, so rolling the entry off on schedule would have
deleted it. Before it the
`DialogueChoice.cond` conjunction gap (v0.152.0), `debug_draw.rs:34` with the 2026-08-07 analysis
program (v0.151.1), the RPG genre gap (v0.151.0) and the four unmeasured v0.150.0 allocation claims
(v0.150.7).

⚠️ **Two programs ended here, and the file should stay small now.** The v0.150.x measurement program
closed with v0.150.7 and the 2026-08-07 analysis with v0.151.1. Nothing from either is open. What
survives them is instruments and habits, not work: `tests/per_frame_alloc.rs` settles an allocation
claim in one command, and `docs/PATTERNS.md` carries the two rules they cost — a fail-path check is
worthless until you revert the fix and watch it go red, and a measurement is worthless until a
control proves the instrument can see anything at all.
