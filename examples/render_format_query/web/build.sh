#!/usr/bin/env bash
# Build the render_format_query example to wasm and emit web bindings next to index.html.
#
# This is the "ship an engine *example* to the web" path: cargo build --example + wasm-bindgen
# (wasm-pack only builds the library crate). The render-format query is most meaningful here —
# on WebGL2 a float render target like Rgba16Float needs the EXT_color_buffer_float extension,
# so a backend's actual renderability differs from the desktop case.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$PROJECT_ROOT"

echo ">>> building render_format_query -> wasm (release)..."
cargo build --release --example render_format_query --target wasm32-unknown-unknown

echo ">>> generating web bindings (wasm-bindgen --target web)..."
wasm-bindgen --target web --out-dir "$SCRIPT_DIR/pkg" \
  target/wasm32-unknown-unknown/release/examples/render_format_query.wasm

echo ">>> done. Serve it:"
echo "    python3 -m http.server 8080 --directory \"$SCRIPT_DIR\""
echo "    open http://localhost:8080   (click \"Start\"; the tab title shows the self-check verdict)"
