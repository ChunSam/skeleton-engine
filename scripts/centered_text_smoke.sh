#!/usr/bin/env bash
# wasm render smoke for the `centered_text` example (wishlist EW-001 visual demo).
#
# Why this exists: CI builds wasm but never *runs* it, so a wasm render regression
# (HiDPI viewport halving, missing font, canvas stretch) stays invisible — nothing
# renders a wasm frame and looks at it. This builds `centered_text` to wasm, serves
# it, renders one frame headless on a simulated Retina (DPR=2) display, asserts the
# frame is non-blank, and saves the screenshot to eyeball.
#
# Unlike scripts/wasm_smoke.sh (coin_race) this example has NO server / WebSocket —
# it is pure local render — so there is no network path to assert, just the frame.
#
# What it AUTOMATICALLY catches: wasm fails to load / panics on start (blank shot),
# or rendering produces nothing (blank/degenerate frame). What it does NOT catch:
# the EW-001 centering itself — a mis-centered label still yields a non-blank frame.
# For that, EYEBALL the saved screenshot: each cyan/green `DrawText::centered` label
# should straddle its white guide line symmetrically.
#
# Optional *local* check, not a CI gate (CI has no Chrome/GPU).
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the Cargo.lock crate
#   Google Chrome or Chromium (set $CHROME to override auto-detection)
#
# Usage:
#   scripts/centered_text_smoke.sh                 # build + render + assert
#   CHROME=/path/to/chrome scripts/centered_text_smoke.sh
#   SMOKE_KEEP=1 scripts/centered_text_smoke.sh    # keep the screenshot
#
# Exit codes: 0 = pass · 1 = render assertion failed · 2 = environment
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WEB_DIR="$PROJECT_ROOT/examples/centered_text/web"

PORT="${SMOKE_PORT:-8085}"                 # static-file server port (configurable)
SHOT="${SMOKE_SHOT:-/tmp/centered_text_smoke.png}"
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

PROFILE="$(mktemp -d -t chrome_ct_smoke.XXXXXX)"
HTTPD_PID=""
cleanup() {
  [[ -n "$HTTPD_PID" ]] && kill "$HTTPD_PID" 2>/dev/null || true
  pkill -f "$PROFILE" 2>/dev/null || true
  rm -rf "$PROFILE"
}
trap cleanup EXIT

echo ">>> [1/4] building centered_text -> wasm (release)..."
"$WEB_DIR/build.sh" >/dev/null

# Refuse to render against a *stale* server on $PORT (e.g. an orphaned http.server
# from a previous run): it would serve a different page and the byte-size check
# below would FALSE-PASS. wasm_smoke.sh guards its fixed server port the same way.
if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  echo "FAIL: port $PORT is already in use — stop it (or set SMOKE_PORT) so we serve OUR page" >&2
  exit 2
fi

echo ">>> [2/4] serving $WEB_DIR on :$PORT..."
# `--directory` (not a `( cd && python3 )` subshell) so $! IS the python process,
# not a subshell whose python *child* would survive `kill $HTTPD_PID` and orphan
# itself onto $PORT — which is exactly what made a later run false-pass off a stale
# server. (Python 3.7+; system python is 3.9.)
python3 -m http.server "$PORT" --directory "$WEB_DIR" >/dev/null 2>&1 &
HTTPD_PID=$!
sleep 1
# Confirm OUR server actually came up (a silent bind failure must not FALSE-PASS).
if ! kill -0 "$HTTPD_PID" 2>/dev/null; then
  echo "FAIL: http.server failed to start on :$PORT" >&2
  exit 2
fi

echo ">>> [3/4] rendering one frame headless (DPR=2)..."
rm -f "$SHOT"
# --force-device-scale-factor=2 reproduces the Retina/HiDPI path. SwiftShader lets
# headless Chrome do WebGL2 without a real GPU. ?autostart=1 makes index.html call
# run_centered_text() without the (human-only) Start click. Run Chrome backgrounded:
# under SwiftShader it often hangs on *exit* after the screenshot is on disk, so we
# poll for the PNG to appear + stop growing, then reap it ourselves.
"$CHROME" --headless=new \
  --enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader \
  --force-device-scale-factor=2 --window-size=960,540 \
  --hide-scrollbars --virtual-time-budget=5000 \
  --user-data-dir="$PROFILE" \
  --screenshot="$SHOT" \
  "http://localhost:$PORT/?autostart=1" >/dev/null 2>&1 &
CHROME_PID=$!
prev=-1
for _ in $(seq 1 60); do          # up to ~30s
  if [[ -f "$SHOT" ]]; then
    cur=$(wc -c < "$SHOT" | tr -d ' ')
    [[ "$cur" -gt 0 && "$cur" == "$prev" ]] && break   # file written and stable
    prev="$cur"
  fi
  kill -0 "$CHROME_PID" 2>/dev/null || break            # Chrome exited on its own
  sleep 0.5
done
kill "$CHROME_PID" 2>/dev/null || true
pkill -f "$PROFILE" 2>/dev/null || true

echo ">>> [4/4] checking the rendered frame..."
fail=0
if [[ -f "$SHOT" ]]; then
  bytes=$(wc -c < "$SHOT" | tr -d ' ')
  if (( bytes >= MIN_PNG_BYTES )); then
    echo "  ok  : screenshot is $bytes bytes (>= $MIN_PNG_BYTES) -> rendered a real frame"
  else
    echo "  FAIL: screenshot only $bytes bytes (< $MIN_PNG_BYTES) -> blank/degenerate frame" >&2
    fail=1
  fi
else
  echo "  FAIL: no screenshot produced" >&2
  fail=1
fi

if (( fail )); then
  echo ">>> CENTERED_TEXT SMOKE: FAIL" >&2
  echo "    screenshot : $SHOT" >&2
  exit 1
fi
echo ">>> CENTERED_TEXT SMOKE: PASS (run + non-blank render)"
echo "    eyeball $SHOT — each cyan/green label's center should sit on its white guide line (EW-001)"
