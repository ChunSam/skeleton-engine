#!/usr/bin/env bash
# Build the centered_text example to wasm and serve it in the browser.
#
# This is the reusable "ship an engine *example* to the web" path. The engine's
# bundled lib demo (examples/wasm/) is built with wasm-pack, but wasm-pack only
# builds the library crate — so to put an example on the web we drive
# `cargo build --example` + `wasm-bindgen` directly.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen
#                                                    # crate version in Cargo.lock
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$PROJECT_ROOT"

echo ">>> building centered_text → wasm (release)..."
cargo build --release --example centered_text --target wasm32-unknown-unknown

echo ">>> generating web bindings (wasm-bindgen --target web)..."
wasm-bindgen --target web --out-dir "$SCRIPT_DIR/pkg" \
  target/wasm32-unknown-unknown/release/examples/centered_text.wasm

echo ">>> done. Now serve it:"
echo "    python3 -m http.server 8080 --directory \"$SCRIPT_DIR\""
echo "    then open http://localhost:8080 and click Start"
