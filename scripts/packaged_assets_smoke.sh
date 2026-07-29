#!/usr/bin/env bash
# Native smoke for the `packaged_assets` example — the packaged-build asset bug, reproduced.
#
# Why this exists: the bug only appears when the executable is launched from a directory that is
# NOT the repo root (double-clicked, a desktop shortcut, `cd / && game.exe`). Running the example
# the normal way — `cargo run` from the checkout — is exactly the case that always worked, so it
# proves nothing. This builds the binary and then runs it **from `/`**, which is what a shipped
# build's working directory looks like.
#
# The example itself is the assertion (it exits non-zero unless):
#   - a texture AND a data table both resolve, by RELATIVE path, from that foreign working dir,
#   - the data table actually carries rows (an empty table is what the silent failure looked like),
#   - a deliberately MISSING data table is *reported* via asset_failures() rather than swallowed.
#
# Needs: a working GPU (Metal/Vulkan/DX) for the headless render. No display, window, or Chrome.
#
# Usage:
#   scripts/packaged_assets_smoke.sh
#   SMOKE_KEEP=1 scripts/packaged_assets_smoke.sh   # keep the PNG to eyeball
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"
SHOT="${SMOKE_SHOT:-/tmp/packaged_assets_smoke.png}"
rm -f "$SHOT"

echo ">>> [1/3] building the example..."
cargo build --quiet --example packaged_assets

BIN="$PROJECT_ROOT/target/debug/examples/packaged_assets"
if [[ ! -x "$BIN" ]]; then
  echo "FAIL: no binary at $BIN" >&2
  exit 1
fi

echo ">>> [2/3] running it from / (the launch that used to render a magenta window)..."
OUT="$(cd / && HEADLESS_SHOT="$SHOT" "$BIN" 2>&1)" || {
  echo "$OUT"
  echo "FAIL: example exited non-zero — assets did not resolve from a foreign working directory" >&2
  exit 1
}
echo "$OUT"

echo ">>> [3/3] verifying the output..."
if ! grep -q "^OK: texture + data table" <<<"$OUT"; then
  echo "FAIL: example did not report OK" >&2
  exit 1
fi
if [[ ! -s "$SHOT" ]] || ! file "$SHOT" | grep -q "PNG image"; then
  echo "FAIL: no PNG written at $SHOT" >&2
  exit 1
fi

echo "PASS: relative assets resolved from /, and the missing data table was reported"
[[ "${SMOKE_KEEP:-0}" == "1" ]] || rm -f "$SHOT"
