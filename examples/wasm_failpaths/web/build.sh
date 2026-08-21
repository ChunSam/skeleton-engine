#!/usr/bin/env bash
# Build the wasm_failpaths harness to wasm and generate the web bindings.
#
# ⚠️ The echo server stays NATIVE (`cargo run --example wasm_failpaths_echo_server`) — a browser tab
# cannot listen on a socket.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$PROJECT_ROOT"

echo ">>> building wasm_failpaths → wasm (release)..."
cargo build --release --example wasm_failpaths --target wasm32-unknown-unknown

echo ">>> generating web bindings (wasm-bindgen --target web)..."
wasm-bindgen --target web --out-dir "$SCRIPT_DIR/pkg" \
  target/wasm32-unknown-unknown/release/examples/wasm_failpaths.wasm

echo ">>> done. Now, in two terminals:"
echo "    1) cargo run --example wasm_failpaths_echo_server"
echo "    2) python3 -m http.server 8092 --directory \"$SCRIPT_DIR\""
echo "    then open http://localhost:8092"
