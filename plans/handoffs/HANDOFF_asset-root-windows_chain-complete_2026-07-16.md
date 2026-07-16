# Chain complete: hot-reload under an asset root (v0.129.0) + audio codec decode coverage (v0.129.1)

**Date:** 2026-07-16
**Status:** COMPLETED
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `asset-root-windows` seq `4`
**Parent:** `plans/handoffs/HANDOFF_asset-root-windows_embedded-images_2026-07-15.md` (seq 3)
**Prior chain:** seq 1 `downstream-bug-report` (#358/#359) > seq 2 `loud-loader-failures` (#361) > seq 3 `embedded-images` (#363) > **this (seq 4)**: hot-reload (#365) + codec coverage (#366)

---

## Stale References

Everything the parent named still exists and was used this session: `asset_path::resolve`, `resolve_in`, `record_failure`, `asset_failures`, `set_strict_assets`, `set_asset_root`, `asset_key`, `AssetServer`, `watch_path`, `watched_paths`, `path_to_id`, `atlas_path_to_id`, `poll_reloads`, `reload_rx`, `_watcher`, `decode_image_with_state`, `decode_bytes_with_state`, `image_assets_for_gpu`, `upload_asset_server_images_to_gpu`, `HotReloadable`, `hot_reload_forwarders`, `RonRegistry`/`RonLoadable`, `DataTableRegistry`, `DataTable::load`, `reload_texture`. **New this session:** `AssetServer::watched_resolved_to_logical` (field), `AssetServer::logical_for_changed` (`pub(super)`), `codec_decode_tests` (module in `src/audio/playback.rs`), `src/audio/fixtures/{tone.wav,tone.ogg,tone.mp3,README.md}`. No stale identifiers.

## Since Last Handoff

- **The parent's plan (seq-3 PLAN) was to close the ONE EW-008 acceptance clause the engine did not meet — F2 hot-reload watching whatever path `resolve()` found.** This session executed that plan exactly (Phases 1→2→3), shipped it as v0.129.0 (#365), then — on the user's instruction ("남은것 지금 진행 해줘") — closed the chain's remaining candidate (the vorbis/`.ogg` decode test gap) as v0.129.1 (#366).
- **The board-check gate ran FIRST and both channels were empty of NEW work** (dm EW-007/008 still `Shipped` awaiting the game's verify; rust-survivors `_None._`), so the seq-3 plan was not preempted.
- **A cross-repo board note also landed this session** (dungeon-merchant PR #30): an append-only `[Engine]` reply on the EW-008 thread noting the hot-reload gap is now closed in v0.129.0 — the engine had explicitly promised "file it and I'll thread the resolved path into the watchers," and this shipped it proactively instead.
- **The `asset-root-windows` chain is now FULLY COMPLETE** — 6 engine PRs across 4 handoff-seqs served the whole rust-survivors + dungeon-merchant packaged-build bug report. Nothing is pending in either request channel; the self-pick shelf tied to this chain is exhausted.
- **Trajectory:** what began as a downstream "packaged Windows build renders magenta / boots empty" bug report is fully served, down to the last acceptance clause and the last untested codec. The next session starts from a clean, board-gated blank slate.

## Reference Documents

- `CLAUDE.md` — conventions, module map, verify-gate rules. **Updated twice this session** (header → v1.6.222 then v1.6.223; the `asset_path` module-map row extended for the hot-reload watcher fix; the audio row extended for the codec decode tests).
- `docs/CHANGELOG.md` — 0.129.0 + 0.129.1 entries written this session.
- `docs/VISION.md` — the feature+example loop. Leaned on it: `examples/hot_reload_asset_root` is the hot-reload acceptance proof.
- Grandparent seq-1 handoff — the `resolve()`-at-read-sites-only + identity-stays-logical constraint that governs the whole chain. **Read before touching image identity or the watchers again.**
- The seq-3 PLAN (`PLAN_asset-root-windows_embedded-images_2026-07-15.md`) — the hot-reload design this session executed; its Phase 1 mapping and Anti-Goals were followed verbatim.

---

## The Goal

Two goals, executed in order:

1. **Close EW-008's last acceptance clause (the seq-3 plan): "F2 hot-reload keeps watching whatever path was resolved."** Since v0.126.0, `resolve()` maps a relative asset path onto an engine-determined root at the *filesystem read* only — never at the cache key, `Handle::path()`, or the `notify` watcher. So the watcher registered the caller's *logical* path. In a packaged / foreign-cwd layout that path does not exist relative to the working directory, and `notify` cannot watch a nonexistent path, so the watch silently failed and F2 hot-reload never fired (only guaranteed dev-from-repo-root, where logical == resolved). The fix had to make the watcher resolve WITHOUT disturbing asset identity (the 2026-05-29 white-sprite cache-key bug is what rewriting identity re-breaks).

2. **Close the chain's remaining candidate: the vorbis/`.ogg` decode test gap.** After the rodio 0.19 → 0.22 swap (seq 1, #358) moved decoding onto symphonia, the engine's enabled codec features (`wav`/`vorbis`/`mp3`) had no end-to-end test — `.ogg`/vorbis in particular was decoded by nothing, so a dropped feature or a symphonia regression could ship silently. The historical blocker was "no license-clean `.ogg` fixture" (afconvert can't encode ogg; the game's music is licensed → can't be committed to an MIT repo).

End state: both closed, shipped, merged; the chain complete; both request channels empty; trees clean.

---

## Where We Are

- **`main @ 2adfa2e`, package v0.129.1, CLAUDE.md header v1.6.223, clean tree, all gates green.**
- **Engine PR #365 — MERGED** `506efd8` (2026-07-15, watch-and-confirm, CI **6/6** incl. Build Windows/DX12 + Render lavapipe). v0.129.0 — hot-reload under an asset root.
- **Engine PR #366 — MERGED** `2adfa2e` (2026-07-15, async auto-merge, CI green). v0.129.1 — audio codec decode coverage.
- **dungeon-merchant PR #30 — MERGED** `bfa470e` (2026-07-15). The EW-008 board note.
- **Lib tests: 1245 (seq-3 close) → 1247 (hot-reload +2) → 1250 (codec +3).**
- **Memory bumped twice:** `engine-current-state` seq 184 (hot-reload) then seq 185 (codec).
- **Both downstream request channels remain empty of open engine items.** Next free board ID **EW-009**.

### Hot-reload fix (v0.129.0) — the three source edits

- **`src/asset.rs`** — new native-only field `watched_resolved_to_logical: HashMap<Arc<str>, Arc<str>>` on `AssetServer` + init in the native `new()` branch (wasm branch omits it, like `_watcher`/`reload_rx`).
- **`src/asset/hot_reload.rs`** — `watch_path` now `w.watch(resolve(&path_str))` (the file that exists) and inserts `asset_key(&resolved) → asset_key(logical)` into the reverse map; `poll_reloads` translates each notify event via the new `pub(super) logical_for_changed(&self, &Path) -> Arc<str>` BEFORE the `is_known` membership check.
- **`src/asset/image_loading.rs`** — the image watch site (only fires on `AssetLoadState::Loaded`) watches `resolve(&*key)` and registers the same reverse-map entry; the image's `path_to_id` key stays the logical `key`.

### Codec coverage (v0.129.1) — one source edit + fixtures

- **`src/audio/playback.rs`** — new `#[cfg(test)] mod codec_decode_tests`: `include_bytes!` each fixture, `rodio::Decoder::new(Cursor::new(bytes))`, assert `sample_rate().get()==22050` + `channels().get()==1` + `count()>1000`.
- **`src/audio/fixtures/{tone.wav,tone.ogg,tone.mp3,README.md}`** — synthesized CC0 fixtures + provenance/regen doc, under `src/**` so they ship with the crate.

---

## What We Tried (Chronological)

### Chunk 1 — Onboarding, board gate, git recovery (early)

1. **Synced and hit a stale-branch git error.** `git pull --ff-only` failed: the local branch `docs/handoff-seq3-embedded-images` was configured to track a ref not on the remote (the seq-3 handoff had squash-merged as #364 → `254e9f7`, a different hash than the local pre-squash `388fa38`). Recovered by `git checkout main && git pull --ff-only` (fast-forwarded `5d5bf61..254e9f7`), then `git checkout -b feat/hot-reload-asset-root` off fresh main.
2. **Board-check gate (the plan's precondition).** Read `../dungeon-merchant/docs/engine-wishlist.md` (EW-007/008 both `Shipped`, no NEW request, next free EW-009) and `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` (`_None._`). No preemption → executed the seq-3 plan.
3. **Read the Phase 1 source files** (`hot_reload.rs`, `asset.rs`, `image_loading.rs`, `asset_path.rs`, `schedule.rs`) plus `ron_registry.rs` and `data_table.rs` to confirm how registries key/match.

### Chunk 2 — Phase 1: map the watcher↔reload matching (mandated design-before-code)

4. **Traced every dispatch key to `asset_key(logical)`.** Image `path_to_id` key = `asset_key(logical)` (`load_image`); `watched_paths` stores `asset_key(logical)`; `RonRegistry::load` stores `paths[name] = asset_key(logical)` and `reload_path` re-computes `asset_key(p)` to match. So the ONE key `poll_reloads` must emit for a changed file is `asset_key(logical)`.
5. **Confirmed why it works dev-from-repo-root and breaks packaged.** `asset_key(p)` canonicalizes if `p` exists relative to cwd, else returns the raw string. Dev: logical exists rel cwd → both sides canonicalize to the same abs → match (accidental). Packaged: `watch(logical_relative)` fails to register (nonexistent rel cwd); even if it fired, `asset_key(notify_abs)` ≠ the raw stored key → no match. Double-broken.
6. **Chose the reverse-map + translate-back strategy** and wrote it as a scratch note (`scratchpad/phase1_mapping.md`) before any code: map keyed `asset_key(resolve(logical)) → asset_key(logical)`; both sides go through `asset_key`'s canonicalize so macOS `/private` normalizes identically; dev-from-repo-root entries are identities → byte-identical dispatch.

### Chunk 3 — Phase 2: implement the fix (3 files)

7. **Added the field + init** (`asset.rs`), the two watch-site rewrites + `logical_for_changed` + `poll_reloads` translation (`hot_reload.rs`, `image_loading.rs`). `cargo build --lib` clean; all 20 pre-existing asset tests green (dev-from-repo-root unchanged).

### Chunk 4 — Phase 3: tests + real-notify proof

8. **Two deterministic unit tests** (`src/asset/tests.rs`): `logical_for_changed_translates_a_resolved_path_back_to_the_logical_key` (a HAND-POPULATED non-identity map — needs no process-global root, per the seq-1/2 rule) and `watch_path_registers_a_resolved_to_logical_map_entry` (registration proof).
9. **Proved both non-vacuous** — `perl`-neutralized the map insert + the lookup → both tests went red → restored → green.
10. **Real notify round-trip proof.** Confirmed `DataTable::load` reads via `resolve` + stores the logical `t.path`, and `reload_path` canonicalizes both sides, so the full packaged chain composes. Wrote `examples/hot_reload_asset_root.rs` (headless, no GPU) + `scripts/hot_reload_smoke.sh`: pins a temp asset root, loads a data table by a relative logical path, edits it from a foreign cwd, drives `AssetServer::poll_reloads` + `DataTableRegistry::reload_path` (the schedule.rs forwarder inlined), bounded-retry poll. Output: `OK: … reloaded=["data/items.ron"]` — the LOGICAL key, confirming the translation.

### Chunk 5 — Ship + land #365 (watch-and-confirm)

11. **`cargo fmt` (the reflow trap) → full verify → exit 0** (7/7). `/ship` bumped 0.128.0 → 0.129.0 (MINOR: hot-reload behavior fix + new example), lock + CHANGELOG + CLAUDE.md header v1.6.222 + asset_path module-map note; re-verified exit 0.
12. **`/land-pr` with WATCH-AND-CONFIRM, not async** — hot-reload is a NAMED judgment gate ("anything CI can't exercise"); the smoke was the real-behavior confirmation, done pre-PR. Committed, pushed, PR #365. Watched CI: **6/6 green** (Windows/DX12 1m37s, Render lavapipe 1m28s, native 6m33s), `mergeStateStatus == CLEAN` → squash-merged `506efd8`, synced main, pruned branch, memory seq 184.

### Chunk 6 — The board note (dungeon-merchant PR #30)

13. **User approved the follow-up ("후속 제안 진행해").** Re-read the current EW-008 thread (unchanged; append-only), confirmed dm is a clean git repo with a remote. Appended a dated `[Engine]` reply on the EW-008 thread (v0.129.0 closes the gap; bump to ≥ v0.129.0 makes F2 hot-reload fire from a packaged layout; no game-side change).
14. **Branched (not pushed to default), committed, pushed, PR #30, squash-merged `bfa470e`, synced dm main.** dm has no CI/branch protection, so no wait.

### Chunk 7 — Codec coverage: unblock the fixture

15. **User: "남은것 지금 진행 해줘"** — do the chain's last candidate (vorbis/.ogg test gap). Probed the machine: no `ffmpeg`/`oggenc`/`sox`/`opusenc`; zero audio fixtures in the repo; `rodio` features `["playback","wav","vorbis","mp3"]`; decode path is `Decoder::new(Cursor::new(bytes))` (device-free); `brew` available (vorbis-tools not installed); Python 3.9 stdlib `wave` only.
16. **Dissolved the blocker by synthesizing from scratch.** `brew install vorbis-tools lame` (local, one-time). Python `wave` wrote a ~0.15 s / 22050 Hz / mono / 440 Hz sine (CC0 — no third-party audio). `oggenc -q 0` → `tone.ogg` (3.6 KB); `lame -m m -b 32` → `tone.mp3` (836 B); kept `tone.wav` (6.6 KB). Committed under `src/audio/fixtures/` + a provenance/regen `README.md`.

### Chunk 8 — Codec decode test + ship/land #366 (async)

17. **Checked rodio 0.22 types in the vendored source** before writing: `SampleRate = NonZero<u32>`, `ChannelCount = NonZero<u16>` (use `.get()`); `Decoder::new(data: R)` requires `R: Read + Seek + Send + Sync + 'static` — `Cursor<&'static [u8]>` satisfies it directly; `Decoder` is a `Source + Iterator`.
18. **Wrote `codec_decode_tests`** — 3 tests, one per codec, sharing an `assert_decodes` helper; device-free so CI-runnable. All 3 pass (including the previously-untested vorbis).
19. **Proved non-vacuous** — `perl`-dropped `vorbis` from the Cargo.toml rodio features → `vorbis_ogg_fixture_decodes` reds with "is its rodio codec feature still enabled?" → restored.
20. **`/ship` 0.129.0 → 0.129.1 (PATCH, test-only)** + lock + CHANGELOG + CLAUDE.md header v1.6.223 + audio module-map note; re-verified exit 0.
21. **`/land-pr` ASYNC auto-merge** — decode is CI-verifiable (device-free, runs on the native job), so green CI IS the confirmation → NOT a judgment gate (contrast audio PLAYBACK). Committed, pushed, PR #366, armed `--auto`; CI green → merged `2adfa2e`; synced main, pruned branch, memory seq 185.

---

## Key Decisions

- **The reverse map is keyed `asset_key(resolve(logical)) → asset_key(logical)`, and only the watch TARGET + this internal map become resolved — cache keys and `Handle::path()` stay logical.** This is the whole hot-reload design: it keeps the chain's load-bearing invariant (rewriting identity re-breaks the 2026-05-29 white-sprite bug) while making the watch register a real file. Dev-from-repo-root entries are identities, so dispatch is byte-identical there.
- **`logical_for_changed` is a pure `pub(super)` fn.** Extracting the translation as a pure function makes it unit-testable without a real watcher (the notify event loop is timing-dependent and not deterministically testable). `pub(super)` is the tightest visibility that reaches the sibling `asset::tests` module.
- **Non-vacuity is proven by neutralizing the fix, not assumed.** Both hot-reload unit tests and the vorbis test were shown to go red without their respective fix. A test that can't fail proves nothing.
- **Watch-and-confirm for hot-reload (#365), async auto-merge for codec (#366) — the distinction is what CI can actually exercise.** Hot-reload PLAYBACK/behavior is CI-unverifiable (a named judgment gate) → confirmed via the local smoke before merge. Codec DECODE is device-free and runs on CI's native job → green CI is the real-behavior confirmation, so async is correct. "Audio" being in the judgment-gate list is about *playback you can't hear on CI*, not decode.
- **Codec fixtures are synthesized from scratch (CC0), not any licensed sample.** That is what dissolves the historical blocker. A 440 Hz sine we generate is unambiguously public-domain and safe in an MIT repo. The encoders (`oggenc`/`lame`) are needed only LOCALLY, one-time, to create the fixtures; CI only decodes (rodio+symphonia already in the tree).
- **Fixtures live under `src/**` (the Cargo.toml `include` list) so they ship with the crate; the `include_bytes!` is `#[cfg(test)]`.** `cargo package`'s verify build compiles the lib WITHOUT tests, so it never reads the include — the DejaVu-style include-list gotcha (which bites un-gated lib includes) does not apply. But the files still ship, so a downstream `cargo test` on the vendored crate works.
- **Version calls: v0.129.0 MINOR** (hot-reload behavior fix + a new example = observable new capability), **v0.129.1 PATCH** (test-only, no API/runtime change). Both consistent with the pre-1.0 rule.
- **The board note went via a branch + PR in the dungeon-merchant repo, not a direct push to its default branch** (global git convention), even though dm's main is unprotected.

---

## Evidence & Data

### The Phase 1 design artifact (written before any code, verbatim intent)

The mandated design note (`scratchpad/phase1_mapping.md`) established, in order:

1. **Every dispatch key is `asset_key(logical)`** — image `path_to_id`, `watched_paths`, and `RonRegistry.paths` all store it, so `poll_reloads` must emit exactly that one key for a changed file.
2. **`asset_key(p)` = `canonicalize(p)` if `p` exists rel cwd, else the raw string** (`src/asset.rs:237`) — this is the hinge. Dev: logical exists rel cwd → canonical both sides → accidental match. Packaged: `watch(logical_relative)` never registers (nonexistent rel cwd) + `asset_key(notify_abs)` ≠ raw stored key → double-broken.
3. **The fix: watch `resolve(logical)`, record `asset_key(resolve(logical)) → asset_key(logical)`, translate events back before the membership check.** Fallback to `asset_key(path)` when unmapped = today's behavior.
4. **Both sides go through `asset_key`'s canonicalize** → macOS `/private` normalizes identically on the stored key and the reported event (the risk the plan flagged, mitigated by construction).
5. **Scope: exactly TWO watch sites** (`grep '.watch('` → `image_loading.rs:54` + `hot_reload.rs:80`); `load_atlas` registers no watcher, so nothing to change there.

### The registry composition (why returning the logical key reloads correctly)

`DataTable::load(path)` reads via `std::fs::read_to_string(resolve(path))` and stores `t.path = path` (the LOGICAL string). `DataTableRegistry::reload_path(p)` canonicalizes BOTH sides (`canon(p) == canon(t.path)`) before matching, then re-loads via `DataTable::load(p)` → `resolve(p)`. So when `poll_reloads` hands the registry the logical key: in packaged, `canon("data/items.ron")` == `canon(t.path=="data/items.ron")` (both raw, canonicalize fails) → match → `resolve` reads the real file. The chain composes end to end.

### The dispatch-key mapping (Phase 1 conclusion)

| Consumer | Key it stores / matches on | In `poll_reloads` the changed file must map to |
|---|---|---|
| Image (`load_image`) | `path_to_id` key = `asset_key(logical)` | `asset_key(logical)` |
| `watch_path` | `watched_paths` entry = `asset_key(logical)` | `asset_key(logical)` |
| `RonRegistry::load` | `paths[name] = asset_key(logical)`; `reload_path` re-canonicalizes | `asset_key(logical)` |

All three = `asset_key(logical)`. The reverse map turns the notify event (`asset_key(resolve(logical))`) back into that one key.

### Why dev-from-repo-root is byte-identical

`resolve("assets/x")` in dev = the repo-root-joined abs path (exists). `asset_key(that)` = canonical. `asset_key(logical)` (logical exists rel cwd) = the SAME canonical. So the map entry is `canonical → canonical` (identity); `logical_for_changed` returns exactly what `asset_key(notify_path)` returned before → dispatch unchanged. The 20 pre-existing asset tests key on `watched_paths` (still logical) → untouched.

### Hot-reload smoke output (the real notify round-trip, from a foreign cwd)

```
OK: hot-reload fired under a foreign asset root — tier 'common' -> 'legendary'. reloaded=["data/items.ron"]
```

`reloaded=["data/items.ron"]` is the LOGICAL key — the watcher fired on the resolved temp file and `poll_reloads` translated it back. This would never register against pre-fix code (the relative logical path is unwatchable under `/`).

### Codec fixtures (synthesized, CC0)

| File | Codec | Encoder | Size |
|---|---|---|---|
| `tone.wav` | PCM RIFF/WAVE | Python `wave` (stdlib) | 6658 B |
| `tone.ogg` | Vorbis in Ogg | `oggenc -q 0` (vorbis-tools) | 3629 B |
| `tone.mp3` | MPEG-2 Layer III | `lame -m m -b 32` | 836 B |

Same source: ~0.15 s, 22050 Hz, mono, 440 Hz sine.

### Codec non-vacuity proof

Dropping `vorbis` from `features = ["playback","wav","vorbis","mp3"]`:

```
thread '...::vorbis_ogg_fixture_decodes' panicked at src/audio/playback.rs:632:
ogg/vorbis must decode — is its rodio codec feature still enabled? An IO error occurred while reading...
test result: FAILED. 0 passed; 1 failed
```

`Decoder::new` only recognizes a container whose feature is compiled in → the test literally guards the codec feature line.

### rodio 0.22 API reference (looked up in the vendored source before writing the codec test)

`~/.cargo/registry/src/index.crates.io-*/rodio-0.22.2/`:

| Item | Fact | Consequence for the test |
|---|---|---|
| `SampleRate` | `pub type SampleRate = NonZero<u32>` (`common.rs:5`) | `sample_rate().get()` → `u32`; compare `== 22_050` |
| `ChannelCount` | `pub type ChannelCount = NonZero<u16>` (`common.rs:8`) | `channels().get()` → `u16`; compare `== 1` |
| `Decoder::new` | `fn new(data: R) -> Result<Self, DecoderError>` where `R: Read + Seek + Send + Sync + 'static` (`decoder/mod.rs:358,386`) | `Cursor<&'static [u8]>` (from `include_bytes!`) satisfies the bound directly — no `.to_vec()` |
| `Decoder` | `impl Iterator` + `impl Source` (`decoder/mod.rs:560,577`) | `.count()` pulls the whole stream; `.sample_rate()`/`.channels()` are `&self` |

The dropped-feature test proves the point: `Decoder::new` on `.ogg` returns `Err` when `vorbis` isn't compiled in.

### The merge-mode decision (why #365 watched, #366 async)

| PR | Change | CI verifies the behavior? | Mode |
|---|---|---|---|
| #365 hot-reload | notify watcher + F2 reload | No — notify round-trip not in CI (latency); a named judgment gate | **Watch-and-confirm** (smoke done pre-PR = the real-behavior confirmation) |
| #366 codec decode | device-free decode via `Decoder` | **Yes** — the 3 tests run on the native CI job | **Async auto-merge** (green CI IS the confirmation) |

The judgment gate names "audio", but that means audio *playback you can't hear on CI* — decode is CI-run, so #366 is correctly async.

### Git recovery at session open

`git pull --ff-only` on the local `docs/handoff-seq3-embedded-images` branch failed: it tracked a ref not fetched (the seq-3 handoff squash-merged as #364 `254e9f7`, a different hash than the local pre-squash `388fa38`). Recovery: `git checkout main && git pull --ff-only` (`5d5bf61..254e9f7`), then branch fresh. A squash-merge always leaves the local pre-squash branch orphaned — expected, not an error to debug.

### Test-count trajectory

| Point | Lib tests |
|---|---|
| seq-3 close (v0.128.0) | 1245 |
| hot-reload (v0.129.0) | 1247 (+2) |
| codec coverage (v0.129.1) | 1250 (+3) |

### Merge log

| Repo | PR | Commit | What |
|---|---|---|---|
| skeleton-engine | **#365** | `506efd8` | v0.129.0 — hot-reload under an asset root |
| dungeon-merchant | **#30** | `bfa470e` | EW-008 board note (gap closed) |
| skeleton-engine | **#366** | `2adfa2e` | v0.129.1 — audio codec decode coverage |

### Gate/CI history

| Run | Result |
|---|---|
| verify (baseline / pre-#365 / post-bump) | 0 / 0 / 0 |
| CI #365 | 6/6 (incl. Build Windows/DX12, Render lavapipe) |
| verify (codec / post-bump) | 0 / 0 |
| CI #366 | green (async auto-merge on the 5 required checks) |

---

## Code Analysis

- **`AssetServer::watched_resolved_to_logical: HashMap<Arc<str>, Arc<str>>`** (`src/asset.rs`, native-only) — keyed `asset_key(resolve(logical))`, value `asset_key(logical)`. Populated by both watch sites; read by `logical_for_changed`.
- **`AssetServer::watch_path`** (`src/asset/hot_reload.rs`) — idempotency guard on `watched_paths.contains(key)` unchanged; now computes `resolved = resolve(&path_str)`, watches `&resolved`, inserts `asset_key(&resolved) → Arc::clone(&key)`, then `watched_paths.insert(key)`.
- **`AssetServer::logical_for_changed(&self, changed: &Path) -> Arc<str>`** (`pub(super)`) — `let key = asset_key(changed); self.watched_resolved_to_logical.get(&key).cloned().unwrap_or(key)`. Pure; the Phase-3 regression guard.
- **`AssetServer::poll_reloads`** — the `while let Ok(path)` loop now calls `self.logical_for_changed(&path)` instead of `asset_key(&path)` before the `is_known` check + `seen` dispatch. The second (re-decode) loop is unchanged (operates on the logical key).
- **`AssetServer::load_image` watch site** (`src/asset/image_loading.rs`) — only on `Loaded`, watches `resolve(&*key)` and inserts the reverse-map entry; `path_to_id` key stays logical.
- **`codec_decode_tests`** (`src/audio/playback.rs`, `#[cfg(test)]`) — `assert_decodes(name, bytes: &'static [u8])`: `Decoder::new(Cursor::new(bytes))` → assert `sample_rate().get()==22050`, `channels().get()==1`, `count()>1000`. Three `#[test]` fns (wav/ogg/mp3).
- **The verify gate** (`./scripts/verify.sh`): fmt → clippy --all-targets → wasm build (lib+bins) → wasm clippy --lib → test --all-targets → test --doc → rustdoc `-D warnings`. Read its exit from a non-piped call (zsh `$pipestatus` is 1-indexed).

---

## Files Changed

### Source (v0.129.0)
- `src/asset.rs` — new `watched_resolved_to_logical` field + native init.
- `src/asset/hot_reload.rs` — `watch_path` resolves + registers reverse map; `poll_reloads` translates; new `logical_for_changed`.
- `src/asset/image_loading.rs` — image watch site resolves + registers reverse map.

### Source (v0.129.1)
- `src/audio/playback.rs` — new `codec_decode_tests` module.
- `src/audio/fixtures/{tone.wav,tone.ogg,tone.mp3,README.md}` — NEW synthesized CC0 fixtures + provenance doc.

### Tests
- `src/asset/tests.rs` — +2 hot-reload unit tests. (Codec tests live inline in `playback.rs`.)

### Examples / scripts
- `examples/hot_reload_asset_root.rs` — NEW headless real-notify proof.
- `scripts/hot_reload_smoke.sh` — NEW driver for the above.

### Release paperwork
- `Cargo.toml` / `Cargo.lock` — 0.128.0 → 0.129.0 → 0.129.1.
- `docs/CHANGELOG.md` — 0.129.0 + 0.129.1 entries.
- `CLAUDE.md` — header v1.6.221 → v1.6.222 → v1.6.223; asset_path row (hot-reload watcher) + audio row (codec tests) extended.

### Cross-repo
- `dungeon-merchant/docs/engine-wishlist.md` — EW-008 thread `[Engine]` append (PR #30).

### Memory
- `engine-current-state.md` — seq 184 then seq 185.

---

## User Feedback & Preferences (REQUIRED)

- **The execution prompt was terse and directive** ("Execute the plan. … Do NOT re-derive the design — the plan and handoff have it. Build."). Honored: followed the plan's Phase 1 mapping and Anti-Goals verbatim, no re-litigation.
- **"후속 제안 진행해"** — approved the optional cross-repo board note. Terse approval; proceeded (branch → PR → merge).
- **"남은것 지금 진행 해줘"** — do the chain's remaining candidate (the codec test gap). This authorized the `brew install` needed to generate fixtures (a local, reversible, one-time system change necessary for the task).
- **"handoffplan"** — write the handoff + a next-session plan, then commit and close. (This file is the handoff half.)
- **Standing: user-facing reports in Korean; code, docs, commit messages, PR bodies, handoffs in English.** Followed throughout.
- **Standing: merge authority delegated** (async on green CI where CI verifies; watch-and-confirm on judgment gates). Applied both modes this session by their correct trigger.
- **Standing: the board takes priority; if both channels empty, ASK for direction.** The self-pick shelf for this chain is now exhausted → the next session must ask.

---

## Where We're Going

*(Paired with `PLAN_asset-root-windows_chain-complete_2026-07-16.md`. Summary; the plan is the authority.)*

1. **The `asset-root-windows` chain is COMPLETE. There is no pre-decided next feature.** The next session's Phase 1 is the board-check gate: read `../dungeon-merchant/docs/engine-wishlist.md` (next free EW-009) and `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md`. A newly-filed request is served first.
2. **If both channels are empty, ASK the user for a new direction** — the self-pick shelf tied to this chain is exhausted. Candidate NEW areas the memory/prior handoffs have floated (none started): more procgen modes (cave/cellular-automata, multi-level + stairs), audio-driven gameplay hooks, a 2nd capstone game, tilemap streaming.
3. **Two low-value loose ends exist but neither is blocking:** the `load_image_bytes` wasm path is compiled but not runtime-exercised (a `wasm_smoke.sh`-style browser check would close it); a `load_image_bytes` `_with_format` / atlas-from-bytes parity is unrequested. Note them only if a use case appears.

---

## Risks & Blockers

- **`dungeon-merchant` has no CI and no branch protection** — its board PR convention is discipline, not enforcement. Don't read "MERGED" there as "verified" beyond the docs append it is.
- **`rust-survivors` has auto-merge DISABLED** and pins engine rev `a3369ee` (v7.0.0, pre-0.x-reset) — merge its PRs by hand; an engine fix can't reach it without a 100+-release migration (a paused project, deliberately not chased).
- **The codec test needs the encoders only to REGENERATE fixtures, never to run** — CI decodes the committed files. If a fixture is ever lost, `src/audio/fixtures/README.md` has the exact regen recipe (`brew install vorbis-tools lame` on macOS).
- **The hot-reload smoke is a real-notify test with latency** — it uses a bounded 5 s retry, not a single poll. It is NOT wired into CI (notify latency); the deterministic regression guards are the two unit tests. Don't move the smoke into `cargo test`.
- **EW-007/008 are still `Shipped`, awaiting the GAME's verify + `[x]`** — the ball is with dungeon-merchant (they're on pin v0.116.0 and must bump to ≥ v0.129.0 to pick up all of it, incl. the hot-reload fix). Not the engine's action.

## Open Questions

- **Does the chain close here, or does the user want a fresh breadth direction next?** The downstream bug report is fully served; everything remaining is self-picked and the shelf is empty. The next session should ASK rather than assume.
- **Should the codec test grow to assert exact sample counts or decoded content?** Currently it asserts rate/channels/non-empty (lossy codecs make exact counts fragile). Sufficient to guard the feature line; tighten only if a subtler regression appears.
- **Is a wasm decode/playback smoke worth adding?** The audio facade + `load_image_bytes` wasm paths are compiled but not runtime-exercised on CI. Low risk; a browser smoke would close it if a wasm consumer reports trouble.

---

## Quick Start for Next Session

```bash
# Nothing is dangling — #365 + #366 merged, dm #30 merged, all trees clean.

# 1. Engine state
cd ~/Projects/skeleton-engine
git log --oneline -3     # expect 2adfa2e (v0.129.1) at the tip
git status -s            # expect clean

# 2. READ THE BOARD FIRST — both channels (a new request preempts everything)
#   ../dungeon-merchant/docs/engine-wishlist.md          (next free EW-009; EW-007/008 Shipped, awaiting the game's verify)
#   ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md     (currently _None._)

# 3. The plan for this session
cat plans/handoffs/PLAN_asset-root-windows_chain-complete_2026-07-16.md

# 4. Verify starting state (read the exit code — do NOT pipe or ;-chain it)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"

# 5. Re-prove this session's work still holds
cargo test --lib codec_decode          # 3 pass (wav/vorbis/mp3)
cargo test --lib -- asset::tests::logical_for_changed asset::tests::watch_path_registers  # 2 pass
./scripts/hot_reload_smoke.sh           # OK: hot-reload fired ... reloaded=["data/items.ron"]

# 6. First action: run the board-check gate (Phase 1 of the plan). If a request exists, serve it.
#    If both channels are empty, ASK the user for a new direction — the self-pick shelf is exhausted.
```
