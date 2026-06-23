#!/usr/bin/env bash
# Build the hdr_render_target example to wasm and emit web bindings next to index.html.
#
# Like examples/positional_audio/web/build.sh, this is the "ship an engine *example* to the web"
# path: cargo build --example + wasm-bindgen (wasm-pack only builds the library crate).
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
#
# NB: the HDR (Rgba16Float) render target needs a float-renderable backend. On the WebGL2 backend
# that is EXT_color_buffer_float (present in modern Chrome/Firefox; verified headless under
# SwiftShader by scripts/hdr_web_smoke.sh).
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$PROJECT_ROOT"

echo ">>> building hdr_render_target -> wasm (release)..."
cargo build --release --example hdr_render_target --target wasm32-unknown-unknown

echo ">>> generating web bindings (wasm-bindgen --target web)..."
wasm-bindgen --target web --out-dir "$SCRIPT_DIR/pkg" \
  target/wasm32-unknown-unknown/release/examples/hdr_render_target.wasm

echo ">>> done. Serve it:"
echo "    python3 -m http.server 8080 --directory \"$SCRIPT_DIR\""
echo "    open http://localhost:8080   (click \"Start\", then Up/Down to change exposure)"
