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

- **HDR render-target check — `./scripts/hdr_web_smoke.sh`** builds the `hdr_render_target` example
  to wasm, serves it (`?autostart=1`), renders one headless frame under **SwiftShader**, and asserts
  a **non-blank frame** — which confirms an `Rgba16Float` color **render target** can be created on
  the WebGL2 backend (it needs `EXT_color_buffer_float`; SwiftShader has it, so modern browsers do
  too). Eyeball the saved shot (`SMOKE_KEEP=1`): the HDR (left) monitor keeps the bright core vs the
  mid square distinct while the LDR (right) monitor collapses them. Run after touching render targets,
  the offscreen pass, or the example.

## Web examples without a dedicated smoke

- **`audio_facade`** ships to the web (`examples/audio_facade/web/build.sh` → click **Start**, then
  the keys) to demonstrate the cross-platform [`Audio`](../src/audio_facade.rs) facade running the
  **same code** on the web as native — but has **no headless smoke** on purpose: its web audio path
  is thin pass-throughs to `WebAudio`, whose runtime lifecycle is already covered by
  `wasm_audio_smoke.sh` above, and the facade demo is interactive (audio needs a user gesture and is
  not headless-capturable). Open the page to hear it; run `wasm_audio_smoke.sh` after touching
  `src/audio_wasm.rs`.
