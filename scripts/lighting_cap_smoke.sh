#!/usr/bin/env bash
# Native headless-render smoke for the `lighting_cap` example (configurable point-light cap).
#
# Why this exists: CI is ubuntu-only and cannot render (no GPU), so it never exercises the real
# GPU lighting path. This renders the `lighting_cap` example twice with NO window and NO display
# (works with the monitor off/asleep/locked) — once at the historical 16-light cap and once at
# the raised 40-light cap — and asserts the 40-cap frame lights up substantially more of the
# scene than the 16-cap frame. That proves `LightingConfig::max_lights` actually drives how many
# point lights the GPU pass renders, beyond the old hardcoded 16.
#
# Needs: a working GPU (Metal/Vulkan/DX). Does NOT need a display, a window, or Chrome.
#
# Usage:
#   scripts/lighting_cap_smoke.sh
#   SMOKE_KEEP=1 scripts/lighting_cap_smoke.sh   # keep the PNGs to eyeball
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"
SHOT16="${SMOKE_SHOT16:-/tmp/lighting_cap_16.png}"
SHOT40="${SMOKE_SHOT40:-/tmp/lighting_cap_40.png}"
rm -f "$SHOT16" "$SHOT40"

run_cap() { # $1=cap $2=path  → echoes the example's stdout
  LIGHTING_CAP="$1" HEADLESS_SHOT="$2" cargo run --quiet --example lighting_cap 2>&1
}

lit_of() { grep -oE 'lit_pixels=[0-9]+' <<<"$1" | grep -oE '[0-9]+'; }

echo ">>> [1/3] rendering the 16-light cap headlessly (no window / no display)..."
OUT16="$(run_cap 16 "$SHOT16")" || { echo "$OUT16"; echo "FAIL: cap=16 run exited non-zero" >&2; exit 1; }
echo "$OUT16"

echo ">>> [2/3] rendering the 40-light cap headlessly..."
OUT40="$(run_cap 40 "$SHOT40")" || { echo "$OUT40"; echo "FAIL: cap=40 run exited non-zero" >&2; exit 1; }
echo "$OUT40"

echo ">>> [3/3] verifying both PNGs + the lit-pixel ratio..."
for f in "$SHOT16" "$SHOT40"; do
  [[ -s "$f" ]] || { echo "FAIL: no PNG written at $f" >&2; exit 1; }
  file "$f" | grep -q "PNG image" || { echo "FAIL: $f is not a PNG" >&2; exit 1; }
done

LIT16="$(lit_of "$OUT16")"
LIT40="$(lit_of "$OUT40")"
[[ -n "$LIT16" && -n "$LIT40" ]] || { echo "FAIL: could not parse lit_pixels from output" >&2; exit 1; }

# The 16-cap frame must light something (the central cluster) but the 40-cap frame must light
# noticeably more (all 40 lights ≈ 2.5× the lit area; require at least 1.5× to absorb overlap).
if (( LIT16 < 10000 )); then
  echo "FAIL: 16-cap frame looks unlit (lit_pixels=$LIT16)" >&2
  exit 1
fi
if (( LIT40 * 100 < LIT16 * 150 )); then
  echo "FAIL: raising the cap to 40 did not light substantially more (16: $LIT16, 40: $LIT40)" >&2
  exit 1
fi

echo "PASS: cap 16 lit $LIT16 px, cap 40 lit $LIT40 px (40-cap renders >1.5x the lit area)"
[[ "${SMOKE_KEEP:-0}" == "1" ]] || rm -f "$SHOT16" "$SHOT40"
