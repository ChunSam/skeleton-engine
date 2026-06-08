#!/usr/bin/env bash
# Build the coin_race client to wasm and serve it in the browser.
#
# This is the reusable "ship an engine *example* to the web" path. The engine's
# bundled lib demo (examples/wasm/) is built with wasm-pack, but wasm-pack only
# builds the library crate — so to put an example game on the web we drive
# `cargo build --example` + `wasm-bindgen` directly.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen
#                                                    # crate version in Cargo.lock
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
cd "$PROJECT_ROOT"

echo ">>> building coin_race_game → wasm (release)..."
cargo build --release --example coin_race_game --target wasm32-unknown-unknown

echo ">>> generating web bindings (wasm-bindgen --target web)..."
wasm-bindgen --target web --out-dir "$SCRIPT_DIR/pkg" \
  target/wasm32-unknown-unknown/release/examples/coin_race_game.wasm

echo ">>> done. Now, in two terminals:"
echo "    1) cargo run --example coin_race_server"
echo "    2) python3 -m http.server 8080 --directory \"$SCRIPT_DIR\""
echo "    then open http://localhost:8080"
