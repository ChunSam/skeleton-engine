#!/usr/bin/env bash
# Build the netplay_game client to wasm and generate the web bindings.
#
# ⚠️ The SERVER stays native. `netplay_server` is a TCP listener — tungstenite and `std::net` are
# native-only by construction, and a browser tab cannot listen on a socket. So playing this on the
# web means: native server here, browser client there.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$PROJECT_ROOT"

echo ">>> building netplay_game → wasm (release)..."
cargo build --release --example netplay_game --target wasm32-unknown-unknown

echo ">>> generating web bindings (wasm-bindgen --target web)..."
wasm-bindgen --target web --out-dir "$SCRIPT_DIR/pkg" \
  target/wasm32-unknown-unknown/release/examples/netplay_game.wasm

echo ">>> done. Now, in two terminals:"
echo "    1) cargo run --example netplay_server"
echo "    2) python3 -m http.server 8080 --directory \"$SCRIPT_DIR\""
echo "    then open http://localhost:8080          (play)"
echo "            or http://localhost:8080/check.html  (the WebSocket check)"
