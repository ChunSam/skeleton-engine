#!/usr/bin/env bash
# wasm render smoke for the `hdr_render_target` example.
#
# Why this exists: CI builds wasm but never *runs* it, so a wasm render regression stays invisible.
# This one ALSO answers a real portability question: does an `Rgba16Float` color **render target**
# work on the WebGL2 backend? It requires the `EXT_color_buffer_float` extension. This builds the
# example to wasm, serves it, renders one frame headless under SwiftShader (the lowest-common-
# denominator WebGL2 backend), asserts the frame is non-blank, and saves the screenshot to eyeball.
#
# What it AUTOMATICALLY catches: wasm fails to load / panics on start (blank shot), or the HDR
# render target can't be created (blank/degenerate frame). What to EYEBALL in the saved shot: the
# HDR (left) monitor should keep the bright core distinct from the mid square while the LDR (right)
# monitor collapses them — that is the HDR target preserving > 1.0 values through the float texture.
#
# Optional *local* check, not a CI gate (CI has no Chrome/GPU).
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the Cargo.lock crate
#   Google Chrome or Chromium (set $CHROME to override auto-detection)
#
# Usage:
#   scripts/hdr_web_smoke.sh                 # build + render + assert
#   CHROME=/path/to/chrome scripts/hdr_web_smoke.sh
#   SMOKE_KEEP=1 scripts/hdr_web_smoke.sh    # keep the screenshot
#
# Exit codes: 0 = pass · 1 = render assertion failed · 2 = environment
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WEB_DIR="$PROJECT_ROOT/examples/hdr_render_target/web"

PORT="${SMOKE_PORT:-8088}"
SHOT="${SMOKE_SHOT:-/tmp/hdr_render_target_smoke.png}"
MIN_PNG_BYTES="${SMOKE_MIN_BYTES:-15000}"  # blank ~4KB · a rendered frame is well above this

# ── locate a Chrome/Chromium binary ──────────────────────────────────────────
CHROME="${CHROME:-}"
if [[ -z "$CHROME" ]]; then
  for c in \
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    "/Applications/Chromium.app/Contents/MacOS/Chromium" \
    "$(command -v google-chrome 2>/dev/null || true)" \
    "$(command -v chromium 2>/dev/null || true)" \
    "$(command -v chromium-browser 2>/dev/null || true)"; do
    if [[ -n "$c" && -x "$c" ]]; then CHROME="$c"; break; fi
  done
fi
if [[ -z "$CHROME" || ! -x "$CHROME" ]]; then
  echo "FAIL: no Chrome/Chromium found — set \$CHROME to its path" >&2
  exit 2
fi

PROFILE="$(mktemp -d -t chrome_hdr_smoke.XXXXXX)"
HTTPD_PID=""
cleanup() {
  [[ -n "$HTTPD_PID" ]] && kill "$HTTPD_PID" 2>/dev/null || true
  pkill -f "$PROFILE" 2>/dev/null || true
  rm -rf "$PROFILE"
  [[ "${SMOKE_KEEP:-0}" == "1" ]] || rm -f "$SHOT"
}
trap cleanup EXIT

echo ">>> [1/4] building hdr_render_target -> wasm (release)..."
"$WEB_DIR/build.sh" >/dev/null

# Refuse to render against a *stale* server on $PORT (would FALSE-PASS off another page).
if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  echo "FAIL: port $PORT is already in use — stop it (or set SMOKE_PORT) so we serve OUR page" >&2
  exit 2
fi

echo ">>> [2/4] serving $WEB_DIR on :$PORT..."
# `--directory` (not a `( cd && python3 )` subshell) so $! IS the python process (else kill orphans it).
python3 -m http.server "$PORT" --directory "$WEB_DIR" >/dev/null 2>&1 &
HTTPD_PID=$!
sleep 1
if ! kill -0 "$HTTPD_PID" 2>/dev/null; then
  echo "FAIL: http.server failed to start on :$PORT" >&2
  exit 2
fi

echo ">>> [3/4] rendering one frame headless (SwiftShader WebGL2)..."
rm -f "$SHOT"
# ?autostart=1 makes index.html call run_hdr_render_target() without the (human-only) Start click.
# SwiftShader supports EXT_color_buffer_float, so the Rgba16Float render target works here.
"$CHROME" --headless=new \
  --enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader \
  --window-size=900,520 --hide-scrollbars --virtual-time-budget=6000 \
  --user-data-dir="$PROFILE" \
  --screenshot="$SHOT" \
  "http://localhost:$PORT/?autostart=1" >/dev/null 2>&1 &
CHROME_PID=$!
prev=-1
for _ in $(seq 1 60); do
  if [[ -f "$SHOT" ]]; then
    cur=$(wc -c < "$SHOT" | tr -d ' ')
    [[ "$cur" -gt 0 && "$cur" == "$prev" ]] && break
    prev="$cur"
  fi
  kill -0 "$CHROME_PID" 2>/dev/null || break
  sleep 0.5
done
kill "$CHROME_PID" 2>/dev/null || true
pkill -f "$PROFILE" 2>/dev/null || true

echo ">>> [4/4] checking the rendered frame..."
fail=0
if [[ -f "$SHOT" ]]; then
  bytes=$(wc -c < "$SHOT" | tr -d ' ')
  if (( bytes >= MIN_PNG_BYTES )); then
    echo "  ok  : screenshot is $bytes bytes (>= $MIN_PNG_BYTES) -> rendered a real frame (HDR RT created)"
  else
    echo "  FAIL: screenshot only $bytes bytes (< $MIN_PNG_BYTES) -> blank/degenerate frame" >&2
    fail=1
  fi
else
  echo "  FAIL: no screenshot produced" >&2
  fail=1
fi

if (( fail )); then
  echo ">>> HDR_RENDER_TARGET SMOKE: FAIL" >&2
  echo "    screenshot : $SHOT" >&2
  exit 1
fi
echo ">>> HDR_RENDER_TARGET SMOKE: PASS"
echo "    eyeball the HDR vs LDR monitors: SMOKE_KEEP=1 scripts/hdr_web_smoke.sh ; open $SHOT"
