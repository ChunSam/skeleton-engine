#!/usr/bin/env bash
# Build the existing RPG example and its browser save-check exports. No new Cargo target.
# wasm-bindgen-cli must match the wasm-bindgen version in Cargo.lock.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/../../.."
cargo build --release --example rpg_quest_game --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir "$SCRIPT_DIR/pkg" \
  target/wasm32-unknown-unknown/release/examples/rpg_quest_game.wasm
