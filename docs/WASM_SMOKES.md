# Optional wasm smoke checks

These are **optional local** checks, **not CI gates** — CI has no Chrome/GPU. CI builds wasm
but never *runs* it, so a wasm-only regression (HiDPI viewport halving, missing font, broken
audio graph, save round-trip) stays invisible until something renders/runs a wasm frame and
looks at it. Each script below does exactly that for one example.

Common prerequisites (all scripts):

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
# Google Chrome or Chromium (set $CHROME to override auto-detection)
```

- **Render check — `./scripts/wasm_smoke.sh`** builds the `coin_race` example to wasm, runs it
  headless on a simulated Retina (DPR=2) display, and asserts the app **connects + renders a
  non-blank frame**, saving the screenshot to eyeball for subtle geometry/text bugs. Run after
  wasm-affecting changes.
- **Save check — `./scripts/wasm_save_smoke.sh`** builds the `wasm_save` example to wasm, runs it
  headless, and asserts the **AEAD save/load `localStorage` round-trip** — encrypt→store→`load`
  round-trip, stored value is hex ciphertext (not plaintext), AEAD tamper detection,
  `save_versioned`/`load_migrated` round-trip (verdict read from the page title; **7/7**). Run
  after touching the wasm path in `src/save.rs`.
- **Audio check — `./scripts/wasm_audio_smoke.sh`** builds the `web_audio` example to wasm, runs
  it headless, and asserts the `WebAudio` **lifecycle** at runtime — `AudioContext` create +
  `resume`→running, master-volume set/clamp, looping-music decode+start, `suspend`/`resume`
  toggle (reads the verdict from the page title over the DevTools endpoint; real-time, *not*
  virtual-time — audio state transitions race a virtual clock). Acoustic output is **not**
  auto-checked (no audio capture) — open the page to actually hear it. Run after touching
  `src/audio_wasm.rs`.
- **Centered-text check — `./scripts/centered_text_smoke.sh`** builds the `centered_text` example
  to wasm, serves it (`?autostart=1`), renders one headless DPR=2 frame, and asserts a **non-blank
  frame** (no server/network — pure render), saving the screenshot to eyeball the **EW-001**
  centering (each `DrawText::centered` label's center on its guide line — the centering itself is
  *not* auto-checked, same subtle-class limit as `wasm_smoke.sh`). Run after touching the text
  renderer or the example.
