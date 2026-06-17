# P2 shipped — particle RON→GPU builder (ParticleConfigSet::gpu_emitter, v0.21.0)

**Date:** 2026-06-18
**Status:** **COMPLETED — shipped + merged, CI-green.** `main` @ `7750da0`, package **v0.21.0**, clean tree.
PR **#108** squash-merged + branch deleted. 841 → **842 lib tests** (+1).
**Chain:** `engine-hardening` seq `23` · **Parent:** `HANDOFF_engine-hardening_p1-dialogue-data_2026-06-18.md` (seq 22)
**Roadmap:** P2 of `PLAN_engine-hardening_p1p2p3-roadmap_2026-06-18.md` (P1✅ · P2✅ · **P3 next**).
**Goal directive (active):** ship roadmap P1→P2→P3, each test+merge+handoff, stop after P3.

---

## What shipped
Closes the seq-20 / 0.18.0 carried gap: RON `gravity`/`emit_shape` reached only the CPU
`ParticleEmitter`; there was no RON→GPU path.

- **`ParticleConfigSet::gpu_emitter(name) -> Option<GpuParticleEmitter>`** (`src/particle/config_set.rs`,
  right after `emitter()`): the GPU-compute counterpart to `emitter()`. Nine shared fields map
  1:1 (`spawn_rate`/`lifetime`/`velocity`/`velocity_spread`/`color_start`/`color_end`/`gravity`/
  `emit_shape`/`emit`). Two differences: the square GPU `size` takes the config **width**
  (`size.0`); `texture`/`z` have no GPU equivalent → ignored.
- **Example `gpu_particles`** now loads from `examples/gpu_particles.ron` via
  `App::load_particle_configs` + `gpu_emitter`, and **auto-spawns one emitter at center** so the
  RON→GPU path shows on launch (more on left-click). The auto-spawn doubles as the playtest hook
  (synthetic mouse clicks don't reach winit; auto-spawn sidesteps that).
- +1 unit test `gpu_emitter_maps_fields_and_uses_width`.

## Two gotchas hit (both fixed)
1. **`clippy::field_reassign_with_default` DID fire** on `let mut e = GpuParticleEmitter::default(); e.x = …`
   — contrary to the assumption that private fields suppress it. Fix: **functional update**
   `GpuParticleEmitter { …pub fields…, ..Default::default() }`, which DOES compile across modules
   even with private `timer`/`next_slot` (the private fields come from `Default`, unnamed). The
   `gpu_particles` example avoided the lint only because its rewrite no longer hand-builds the emitter.
2. **`GpuParticleEmitter` is native-only** — `src/lib.rs:19-20` gates `pub mod gpu_particle;` with
   `#[cfg(not(target_arch="wasm32"))]` (the cfg is on the line ABOVE the `pub mod`, easy to miss
   with a one-line grep). So `gpu_emitter` + its `use` must also be `#[cfg(not(target_arch="wasm32"))]`,
   or the wasm `--lib` build fails E0432. The unit test needs no gate (`#[cfg(test)]` excludes it
   from the wasm build; tests run native only).

## Evidence
- **Verify gate (`./scripts/verify.sh`, EXIT 0):** fmt · clippy --all-targets · **wasm build**
  (gpu_emitter correctly gated out) · wasm clippy · `cargo test --all-targets` (**842 lib**) ·
  doctests (65) · doc. All green.
- **CI (PR #108, run 27704387448): ALL GREEN** — Test (native) 4m0s · Build (WASM) 42s · Rustdoc
  40s · Package 1m16s. Squash-merged on green.
- **Playtest (`/tmp/gp_0.png`, `/tmp/gp_1.png`):** orange `Circle`-scatter sparks at center,
  falling under gravity; two frames 1.4s apart differ → compute-shader velocity+gravity
  integration confirmed. Sparse particle count is the known shell-launched-GUI run-loop artifact
  (seq-20), not a bug.

## Files changed
- `src/particle/config_set.rs` (gpu_emitter + test), `examples/gpu_particles.rs` (RON load +
  center auto-spawn), `examples/gpu_particles.ron` (new), release paperwork (`Cargo.toml`/`Cargo.lock`
  0.21.0, `docs/CHANGELOG.md`, `CLAUDE.md` header v1.6.70 + particle row).

## Risks & Blockers
- None. Tree clean, main green, P2 merged.

## Next: P3 — wasm parity (→ 0.22.0+), two independent axes
Per the roadmap PLAN §P3 (grounded there with file:line):
- **3A wasm AEAD save/load → 0.22.0.** Today `save`/`load`/`save_versioned`/`load_migrated` return
  `SaveError::Unsupported` on wasm; only `write_ron`/`read_ron`/`exists`/`delete` work (localStorage).
  Wire `getrandom`/`OsRng` for the nonce + base64-encode ciphertext into localStorage + implement
  the wasm branches. **OPEN QUESTION (confirm before building):** `src/save.rs:270` says the author
  *deliberately* skipped wasm AEAD ("a hardcoded key in a browser-inspectable store buys little") —
  browser AEAD is obfuscation, not security. The active goal says "do P3"; if 3A's value is in
  doubt, prefer 3B and/or note the obfuscation caveat rather than block.
- **3B wasm audio depth → 0.23.0.** `WebAudio` (src/audio_wasm.rs) is 2 methods (`new`/`play(bytes)`
  one-shot). Extend incrementally: `GainNode` (volume/fade) + `BufferSourceNode.loop` + channel
  handles + `stop`; then `StereoPannerNode` (pan). NOT kira (large swap). Recommend the incremental
  WebAudio path.
- Each axis: example/test as acceptance, ship, PR, merge, handoff. After P3 the goal stops.

## Quick start
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # 7750da0 feat(particle) … (#108)
grep -m1 '^version' Cargo.toml  # 0.21.0
./scripts/verify.sh             # green: 842 lib tests
cargo run --example gpu_particles   # auto-spawns at center; LClick spawn / Space toggle / R clear
# P3: read src/save.rs (wasm Unsupported branches + wasm_storage), src/audio_wasm.rs (WebAudio),
#     PLAN_engine-hardening_p1p2p3-roadmap_2026-06-18.md §P3.
```
