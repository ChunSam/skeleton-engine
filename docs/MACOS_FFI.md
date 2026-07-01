# macOS Apple-framework FFI (objc2)

How to add a binding to an Apple framework (GameController, AVFoundation, …) via the
`objc2` crate family, the way the macOS **GameController gamepad backend**
(`src/input/gamepad_macos.rs`, v0.47.0) was built. The reason this is worth a doc: the
`objc2-*` framework crates are large, auto-generated, strongly typed, and feature-gated
per type, so a little up-front discovery turns "guess and recompile" into a clean
first-try compile. (objc2's strong typing means **a clean compile ≈ correct API usage** —
only runtime behavior remains to verify, which on an OS-gated path means a manual/hardware
check; see the CI-is-ubuntu rule in `CLAUDE.md`.)

## 1. Pin to the objc2 major already in the tree

`winit`/`wgpu` already pull objc2 crates. Adding a framework crate at a *different* objc2
major silently duplicates the whole objc2 ecosystem (compiles, but bloats and can cause
trait-mismatch confusion). Check first:

```bash
grep -A2 '^name = "objc2"' Cargo.lock | grep version          # e.g. 0.5.2 AND 0.6.4 both present
grep -A1 '^name = "objc2-foundation"' Cargo.lock | grep version  # e.g. 0.2.2  (0.5-era)
```

Then choose the framework-crate version whose objc2 dependency matches one already there.
Example: `objc2-foundation 0.2` pairs with `objc2 0.5`, so `objc2-game-controller = "0.2"`
reused the existing objc2 0.5 — **no new objc2 version added**. (objc2 `0.2.x` framework
crates → objc2 0.5; `0.3.x` → objc2 0.6.)

Declare it **macOS-only** so it never touches the wasm/Linux/Windows builds or those
platforms' published crate:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.5"
objc2-foundation = { version = "0.2", features = ["NSArray", "NSEnumerator"] }
objc2-game-controller = { version = "0.2", features = [ /* see step 3 */ ] }
```

## 2. Discover the exact API from the registry source (don't guess)

Every type/method is generated, so the source is the authoritative reference — read it
instead of guessing method names or return types:

```bash
GC=~/.cargo/registry/src/*/objc2-game-controller-*/src/generated
grep -A2 'fn controllers\b\|fn extendedGamepad\b' "$GC/GCController.rs"
grep -A2 'fn leftThumbstick\b\|fn buttonA\b' "$GC/GCExtendedGamepad.rs"
```

Note the return shape per method — it matters for the call site:

- `Retained<T>` — always present (e.g. `buttonA()`, `leftShoulder()`, dpad `up()`).
- `Option<Retained<T>>` — absent on some hardware; handle with `if let Some(..)`
  (e.g. `buttonOptions()`, `leftThumbstickButton()`).
- Accessors that read values are usually `unsafe fn` returning `c_float`/`bool`
  (`GCControllerAxisInput::value() -> c_float`, `GCControllerButtonInput::isPressed() -> bool`).
  `c_float` is `f32`, so it assigns to an `f32` field directly.

## 3. Enable a feature per type you touch

The framework crates gate **every type** behind a Cargo feature; using a type without its
feature is a "cannot find type" error, not a missing-method one. List every type you
reference. For the gamepad backend:

```toml
features = [
  "GCController", "GCExtendedGamepad", "GCControllerButtonInput",
  "GCControllerAxisInput", "GCControllerDirectionPad", "GCControllerElement",
  "GCPhysicalInputProfile",
]
```

`objc2-foundation` is gated too (`"NSArray"`, `"NSEnumerator"`, `"NSString"`, …).

## 4. Collections + main-thread safety

- `NSArray<T>` has safe Rust additions: `len()`, `first_retained() -> Option<Retained<T>>`,
  `get_retained(i) -> Option<Retained<T>>`, `iter_retained()`. Prefer these over raw
  `objectAtIndex:`.
- The class method that *produces* the array is usually `unsafe` (e.g.
  `GCController::controllers()`), as are the element accessors. Wrap reads in one `unsafe`
  block with a `// SAFETY:` note. For GameController, the safety condition is **main
  thread** — call from the winit event loop (engine systems run there), where the framework
  services its run loop and updates the element snapshots. No background thread, no escaping
  pointers — each call reads a retained snapshot value.

## 5. Linking

The `objc2-*` crate's build emits the right `#[link]` for the framework, so
`cargo build --example … ` / `cargo build --lib` link `GameController.framework`
automatically — no `build.rs` or `-framework` flag needed.

## 6. Verify

- `cargo build --lib` / `--example <name>` on macOS confirms the FFI compiles **and links**
  (≈ correct API usage).
- CI is ubuntu and will **not** compile this path — run the macOS build locally, and verify
  runtime behavior by hand/hardware (for the gamepad backend, `cargo run --example
  gamepad_probe`). See the "CI is ubuntu only" rule in `CLAUDE.md`.
