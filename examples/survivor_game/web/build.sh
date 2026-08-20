#!/usr/bin/env bash
# Build survivor_game to wasm and generate the web bindings.
#
# `wasm-pack` only builds the library crate, so an example game goes through
# `cargo build --example` + `wasm-bindgen` — the same path every deleted web example used.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
#
# ⚠️ The CLI version has to match the `wasm-bindgen` crate in Cargo.lock. A mismatch does not fail
# here — it fails at page load as an unreadable binding error, which is why CI derives the version
# from Cargo.lock rather than pinning a literal.
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$PROJECT_ROOT"

echo ">>> building survivor_game → wasm (release)..."
cargo build --release --example survivor_game --target wasm32-unknown-unknown

echo ">>> generating web bindings (wasm-bindgen --target web)..."
wasm-bindgen --target web --out-dir "$SCRIPT_DIR/pkg" \
  target/wasm32-unknown-unknown/release/examples/survivor_game.wasm

echo ">>> done. Serve it with:"
echo "    python3 -m http.server 8080 --directory \"$SCRIPT_DIR\""
echo "    then open http://localhost:8080          (play)"
echo "            or http://localhost:8080/check.html  (the Web Audio check)"
