#!/usr/bin/env bash
# Build the orbital_dodger client to wasm and serve it in the browser.
#
# Same reusable "ship an engine *example* to the web" path as coin_race/predict_shooter: `wasm-pack`
# only builds the library crate, so an example game goes through `cargo build --example` +
# `wasm-bindgen`.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
cd "$PROJECT_ROOT"

echo ">>> building orbital_dodger → wasm (release)..."
cargo build --release --example orbital_dodger --target wasm32-unknown-unknown

echo ">>> generating web bindings (wasm-bindgen --target web)..."
wasm-bindgen --target web --out-dir "$SCRIPT_DIR/pkg" \
  target/wasm32-unknown-unknown/release/examples/orbital_dodger.wasm

echo ">>> done. Now, in two terminals:"
echo "    1) cargo run --example orbital_dodger_server"
echo "    2) python3 -m http.server 8080 --directory \"$SCRIPT_DIR\""
echo "    then open http://localhost:8080"
