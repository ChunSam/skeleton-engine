# wasm smoke checks

CI builds wasm; building is not running, so a wasm-only regression (HiDPI viewport halving,
missing font, broken audio graph, save round-trip) stays invisible until something actually
renders or runs a wasm frame and looks at it. Each script below does exactly that for one example.

**5 of these are CI gates** as of v0.143.17, in the `wasm-smokes` job — `wasm_save`,
`render_format_query`, `bloom_web`, `wasm_audio` and `audio_reactive`, i.e. every browser smoke
that reports a `*_CHECK: PASS` verdict. `ubuntu-latest` ships `google-chrome`, the scripts already
request swiftshader so no GPU is needed, and the job installs a `wasm-bindgen-cli` pinned to
`Cargo.lock`. **The rest are local-only on purpose**: they assert byte sizes rather than a verdict,
so a green run would not mean the page was correct. Run those by hand and *look* at the frame.

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
- **Fail-path check — `./scripts/wasm_failpaths_smoke.sh`** is the only one here that asserts a
  **failure** path works. Every other entry on this page drives a path that is supposed to succeed
  and passes when nothing goes wrong — so a failure *handler* can be entirely broken with the whole
  list green, which is exactly how two of them shipped. It builds `wasm_failpaths`, stands up
  `wasm_failpaths_echo_server`, and asserts (1) a **404 image fetch reaches `asset_failures()`**
  (fixed v0.150.1 — it used to set `AssetLoadState::Failed` and stop there, leaving that hook and
  `set_strict_assets` native-only in practice) and (2) a **`send` issued before the socket opens
  survives and echoes back** (fixed v0.150.2 — it used to be handed to a `CONNECTING` socket, which
  throws). Verdict `FAILPATH_CHECK: PASS (2/2)` read from the page title over the DevTools
  endpoint; 20 s in-page deadline so "never resolved" is a named FAIL, not a hang.
  ⚠️ **Sabotage-verified in both directions** — reverting either fix turns it red, and only the
  matching half. Re-run that check if you touch either fix: a fail-path smoke that stays green
  when the fix is gone is worse than no smoke, because it reads as coverage. The script also
  cross-checks the page's PASS against the echo server's own log, so the check cannot agree with
  itself. Run after touching wasm asset loading or `src/network/wasm_impl.rs`.
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
- **Audio-reactive check — `./scripts/audio_reactive_smoke.sh`** builds the `audio_reactive`
  example to wasm, runs it headless with `--autoplay-policy=no-user-gesture-required`, and asserts
  that **`Audio::levels` reports a live non-zero level** for a playing tone **and that
  `Audio::bands` returns a low-biased spectrum** for the 110 Hz kick (verdict
  `AR_CHECK: PASS rms=<n> bands low=<n> high=<n>` read from the page title over the DevTools
  endpoint). This one matters more than most: on wasm both come from a Web Audio `AnalyserNode`
  and share almost no code with the native rodio `Source` tap and hand-written FFT the unit tests
  cover, so a wasm *build* proves very little about them. The spectrum check is a **shape**
  assertion, not just non-zero — a mirrored or mis-scaled band fold fails it. Run after touching
  analysis in `src/audio_wasm.rs`, `src/audio_analysis.rs` or `src/audio/spectrum.rs`.
- **Centered-text check — `./scripts/centered_text_smoke.sh`** builds the `centered_text` example
  to wasm, serves it (`?autostart=1`), renders one headless DPR=2 frame, and asserts a **non-blank
  frame** (no server/network — pure render), saving the screenshot to eyeball the **EW-001**
  centering (each `DrawText::centered` label's center on its guide line — the centering itself is
  *not* auto-checked, same subtle-class limit as `wasm_smoke.sh`). Run after touching the text
  renderer or the example.
- **Game-feel capstone check — `./scripts/game_feel_web_smoke.sh`** builds the `game_feel` example
  to wasm, serves it (`?autostart=1`), renders one headless DPR=2 frame, and asserts a **non-blank
  frame** (centered_text's render-only model). The game-feel behaviors (jump forgiveness, hit-stop,
  trail, the live settings menu) are interactive and *not* auto-checked — play the page (Start
  button) or the native example. Run after touching the example or the widget passes it composes.

- **HDR render-target check — `./scripts/hdr_web_smoke.sh`** builds the `hdr_render_target` example
  to wasm, serves it (`?autostart=1`), renders one headless frame under **SwiftShader**, and asserts
  a **non-blank frame** — which confirms an `Rgba16Float` color **render target** can be created on
  the WebGL2 backend (it needs `EXT_color_buffer_float`; SwiftShader has it, so modern browsers do
  too). Eyeball the saved shot (`SMOKE_KEEP=1`): the HDR (left) monitor keeps the bright core vs the
  mid square distinct while the LDR (right) monitor collapses them. Run after touching render targets,
  the offscreen pass, or the example.
- **Render-format-query check — `./scripts/render_format_query_smoke.sh`** builds the
  `render_format_query` example to wasm, serves it (`?autostart=1`), boots it headless under
  **SwiftShader**, and asserts the [`RenderCapabilities`](../src/renderer/context.rs) query is correct
  on the WebGL2 backend: the example self-checks two backend-independent invariants — the surface
  format **is** a renderable color render target, and a block-compressed format (`Bc1RgbaUnorm`) is
  **not** — and writes `RENDER_FORMAT_QUERY_CHECK: PASS (2/2)` to the page title, which this reads over
  the DevTools endpoint. Also proves the example loads + renders on WebGL2 without panicking (no
  verdict appears otherwise). Run after touching the renderability query path in
  `src/renderer/context.rs` or the example.
- **Bloom check — `./scripts/bloom_web_smoke.sh`** builds the `bloom` example to wasm, serves it
  (`?autostart=1`), boots it headless under **SwiftShader**, and asserts the engine's mip-chain "dual
  filter" [bloom](../src/renderer/bloom.rs) pipeline **renders on the WebGL2 backend** — the example
  survives ~30 frames of HDR + bloom and writes `BLOOM_WEB_CHECK: PASS (1/1)` to the page title, which
  this reads over the DevTools endpoint. This exercises an `Rgba16Float` HDR intermediate **plus** a
  pyramid of `Rgba16Float` mip render targets (all need `EXT_color_buffer_float` on WebGL2); a boot
  panic on an unrenderable target leaves no verdict → FAIL. Run after touching `src/renderer/bloom.rs`,
  `bloom.wgsl`, the HDR post intermediate, or the example.
- **Byte-source asset checks — `./scripts/embedded_atlas_smoke.sh` and
  `./scripts/embedded_image_smoke.sh`** build the `embedded_atlas` / `embedded_image` examples to
  wasm, serve them (`?autostart=1`), and render one headless DPR=2 frame. These two are **paired
  assertions** rather than the usual lone non-blank check: each asserts that **no image file is
  served beside the page** *and* that the **frame is non-blank**. Either alone is weak — a non-blank
  frame could have come from a fetch, and an empty directory proves nothing if nothing drew — but
  together they say the art rendered and *cannot* have come from a file, which is the entire claim of
  [`load_atlas_bytes`](../src/atlas.rs) / [`load_image_bytes`](../src/asset.rs). The byte thresholds
  are measured against a same-page engine-never-drew frame, not guessed. **Not** auto-checked: the
  *right* tiles (atlas grid maths) and the *right* image — the white fallback texture, the failure the
  verbatim-key invariant exists to prevent, still yields a non-blank frame — so eyeball the saved shot.
  Run after touching the byte-source loaders, the verbatim-key path, or the examples.

## Web examples without a dedicated smoke

- **`audio_facade`** ships to the web (`examples/audio_facade/web/build.sh` → click **Start**, then
  the keys) to demonstrate the cross-platform [`Audio`](../src/audio_facade.rs) facade running the
  **same code** on the web as native — but has **no headless smoke** on purpose: its web audio path
  is thin pass-throughs to `WebAudio`, whose runtime lifecycle is already covered by
  `wasm_audio_smoke.sh` above, and the facade demo is interactive (audio needs a user gesture and is
  not headless-capturable). Open the page to hear it; run `wasm_audio_smoke.sh` after touching
  `src/audio_wasm.rs`.
