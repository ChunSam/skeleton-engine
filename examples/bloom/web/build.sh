#!/usr/bin/env bash
# Build the bloom example to wasm and emit web bindings next to index.html.
#
# This is the "ship an engine *example* to the web" path: cargo build --example + wasm-bindgen
# (wasm-pack only builds the library crate). The mip-chain "dual filter" bloom is most worth
# showing here — it renders the scene into an Rgba16Float HDR intermediate and blurs the highlights
# through a pyramid of Rgba16Float mip targets, all of which are renderable on WebGL2 only with the
# EXT_color_buffer_float extension. The web build confirms that pipeline actually runs in a browser.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$PROJECT_ROOT"

echo ">>> building bloom -> wasm (release)..."
cargo build --release --example bloom --target wasm32-unknown-unknown

echo ">>> generating web bindings (wasm-bindgen --target web)..."
wasm-bindgen --target web --out-dir "$SCRIPT_DIR/pkg" \
  target/wasm32-unknown-unknown/release/examples/bloom.wasm

echo ">>> done. Serve it:"
echo "    python3 -m http.server 8080 --directory \"$SCRIPT_DIR\""
echo "    open http://localhost:8080   (click \"Start\"; the tab title shows the self-check verdict)"
