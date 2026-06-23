#!/usr/bin/env bash
# Build the positional_audio example to wasm and emit web bindings next to index.html.
#
# Like examples/audio_facade/web/build.sh, this is the "ship an engine *example* to the web"
# path: cargo build --example + wasm-bindgen (wasm-pack only builds the library crate).
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$PROJECT_ROOT"

echo ">>> building positional_audio -> wasm (release)..."
cargo build --release --example positional_audio --target wasm32-unknown-unknown

echo ">>> generating web bindings (wasm-bindgen --target web)..."
wasm-bindgen --target web --out-dir "$SCRIPT_DIR/pkg" \
  target/wasm32-unknown-unknown/release/examples/positional_audio.wasm

echo ">>> done. Serve it:"
echo "    python3 -m http.server 8080 --directory \"$SCRIPT_DIR\""
echo "    open http://localhost:8080   (click \"Start\", then arrow keys to move the listener)"
