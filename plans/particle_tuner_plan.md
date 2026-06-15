# D-4 — Editor particle live-tuner (v8.22.0)

## Goal

Let a designer tune a `ParticleEmitter` live from the Inspector: drag editors for the emitter's
fields, applied in place so the effect updates on the next spawn while the sim runs.

## Why this design

- `ParticleEmitter` does not derive `Reflect`, so the existing reflect-grid inspector doesn't expose
  its fields. Rather than retrofit `Reflect` (its `Option<Arc<str>>` texture and `pub(crate) timer`
  aren't `ReflectValue`-shaped), add a dedicated **Particle Tuner** inspector section — mirroring the
  existing `Tilemap`-only **Tile Paint** section. This gives sensible per-field ranges/speeds (better
  than generic reflect drags) and keeps the change self-contained.
- The docked sim is **not paused by default** (F2 sets `paused = false`), so `ParticleSystem` keeps
  spawning; mutating the emitter component is genuinely live.

## Scope (additive, native-only)

- `App::reset_particle_emitter(sel)` — reset the emitter to `default()`, preserving `texture`. This
  is the unit-testable core (the slider wiring is direct `&mut` field editing egui guarantees).
- `particle_tuner_grid(ui, app, sel)` + `color_rgba_drags(ui, &mut Color)` in `docked.rs`: a column
  of `DragValue`s for `emit/spawn_rate/lifetime/velocity/velocity_spread/size` and r/g/b/a drags for
  the two colors. Exact (no sRGB round-trip) so linear particle colors stay accurate.
- Inspector section gated on `world.get::<ParticleEmitter>(sel).is_some()`, with a **Reset to Default**
  button calling `App::reset_particle_emitter`.
- Version bump 8.21.0 → 8.22.0 (Cargo.toml, CLAUDE.md header + editor row, CHANGELOG).

## Completion criteria

1. `cargo test --lib` green, +tests:
   - `reset_particle_emitter_restores_defaults_keeping_texture` (fields reset, texture preserved).
   - `reset_particle_emitter_no_emitter_is_noop`.
2. Full Gate6 green.
3. Section appears only for `ParticleEmitter` entities; live edits mutate the component.
4. `rust-survivors` unaffected (additive; new editor-internal method, nothing removed).

## Out of scope

- Saving tuned configs to a `ParticleConfigSet` RON (round-trips through `load_particle_configs`
  already exist; a "save preset" button is a possible follow-up).
- Per-emitter burst preview button.
