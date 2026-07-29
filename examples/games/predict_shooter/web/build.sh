#!/usr/bin/env bash
# Build the predict_shooter client to wasm and serve it in the browser.
#
# Same reusable "ship an engine *example* to the web" path as coin_race/web: `wasm-pack` only
# builds the library crate, so an example game goes through `cargo build --example` + `wasm-bindgen`.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
cd "$PROJECT_ROOT"

echo ">>> building predict_shooter → wasm (release)..."
cargo build --release --example predict_shooter --target wasm32-unknown-unknown

echo ">>> generating web bindings (wasm-bindgen --target web)..."
wasm-bindgen --target web --out-dir "$SCRIPT_DIR/pkg" \
  target/wasm32-unknown-unknown/release/examples/predict_shooter.wasm

echo ">>> done. Now, in two terminals:"
echo "    1) cargo run --example predict_shooter_server"
echo "    2) python3 -m http.server 8080 --directory \"$SCRIPT_DIR\""
echo "    then open http://localhost:8080 (one tab per player)"
