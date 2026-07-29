#!/usr/bin/env bash
# Build the embedded_atlas example to wasm and serve it in the browser.
#
# This is the reusable "ship an engine *example* to the web" path. The engine's
# bundled lib demo (examples/wasm/) is built with wasm-pack, but wasm-pack only
# builds the library crate — so to put an example on the web we drive
# `cargo build --example` + `wasm-bindgen` directly.
#
# What this demo proves: the sprite sheet is inside the .wasm module itself
# (`include_bytes!` → `App::load_atlas_bytes`), so the page renders all 12 atlas
# tiles with NO image fetch — the thing a path-loaded atlas cannot do on the web
# without async plumbing. There is no `assets/` directory served beside this page.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen
#                                                    # crate version in Cargo.lock
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$PROJECT_ROOT"

echo ">>> building embedded_atlas → wasm (release)..."
cargo build --release --example embedded_atlas --target wasm32-unknown-unknown

echo ">>> generating web bindings (wasm-bindgen --target web)..."
wasm-bindgen --target web --out-dir "$SCRIPT_DIR/pkg" \
  target/wasm32-unknown-unknown/release/examples/embedded_atlas.wasm

echo ">>> done. Now serve it:"
echo "    python3 -m http.server 8080 --directory \"$SCRIPT_DIR\""
echo "    then open http://localhost:8080 and click Start"
