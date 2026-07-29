#!/usr/bin/env bash
# Build the audio_reactive example to wasm and serve it in the browser.
#
# This is the reusable "ship an engine *example* to the web" path. The engine's
# bundled lib demo (examples/wasm/) is built with wasm-pack, but wasm-pack only
# builds the library crate — so to put an example on the web we drive
# `cargo build --example` + `wasm-bindgen` directly.
#
# What this demo proves: `Audio::levels` reports a channel's live loudness on the
# WEB too, through a Web Audio `AnalyserNode`, using the very same game code that
# runs natively through a rodio `Source` tap. Compiling for wasm would only show
# it type-checks; rendering the page shows the meters actually move.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen
#                                                    # crate version in Cargo.lock
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$PROJECT_ROOT"

echo ">>> building audio_reactive → wasm (release)..."
cargo build --release --example audio_reactive --target wasm32-unknown-unknown

echo ">>> generating web bindings (wasm-bindgen --target web)..."
wasm-bindgen --target web --out-dir "$SCRIPT_DIR/pkg" \
  target/wasm32-unknown-unknown/release/examples/audio_reactive.wasm

echo ">>> done. Now serve it:"
echo "    python3 -m http.server 8087 --directory \"$SCRIPT_DIR\""
echo "    then open http://localhost:8087 and click Start"
