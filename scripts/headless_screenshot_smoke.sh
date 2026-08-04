#!/usr/bin/env bash
# Native headless-render smoke for the `headless_screenshot` example.
#
# Why this exists: nothing else runs a whole *example* through the real GPU render path — the
# render job's `tests/render.rs` exercises the engine, not a shipped example end to end.
# (This header used to say CI cannot render at all. That stopped being true when the lavapipe
# render job was added, and since v0.143.11 this script runs there.)
# This runs the `headless_screenshot` example — which renders a frame into an
# offscreen texture with NO window and NO display (works with the monitor off/asleep/locked),
# reads the pixels back, and writes a PNG — then asserts the frame is non-blank.
#
# Needs: a working GPU (Metal/Vulkan/DX). Does NOT need a display, a window, or Chrome.
#
# Usage:
#   scripts/headless_screenshot_smoke.sh
#   SMOKE_KEEP=1 scripts/headless_screenshot_smoke.sh   # keep the PNG to eyeball
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"
SHOT="${SMOKE_SHOT:-/tmp/headless_screenshot_smoke.png}"
rm -f "$SHOT"

echo ">>> [1/2] rendering headlessly (no window / no display)..."
OUT="$(cargo run --quiet --example headless_screenshot "$SHOT" 2>&1)" || {
  echo "$OUT"
  echo "FAIL: example exited non-zero" >&2
  exit 1
}
echo "$OUT"

echo ">>> [2/2] verifying the PNG..."
if ! grep -q "HEADLESS_SCREENSHOT: PASS" <<<"$OUT"; then
  echo "FAIL: example did not report PASS" >&2
  exit 1
fi
if [[ ! -s "$SHOT" ]]; then
  echo "FAIL: no PNG written at $SHOT" >&2
  exit 1
fi
# Confirm it is actually a PNG.
if ! file "$SHOT" | grep -q "PNG image"; then
  echo "FAIL: $SHOT is not a PNG" >&2
  exit 1
fi

echo "PASS: headless render wrote a non-blank PNG to $SHOT"
[[ "${SMOKE_KEEP:-0}" == "1" ]] || rm -f "$SHOT"
