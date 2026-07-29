#!/usr/bin/env bash
# wasm render smoke for the `embedded_atlas` example (`App::load_atlas_bytes`).
#
# Why this exists: CI builds wasm but never *runs* it, so a wasm render regression
# stays invisible — nothing renders a wasm frame and looks at it. More specifically,
# `load_atlas_bytes` exists so a gridded sprite sheet can ship INSIDE the .wasm
# module, and "it compiles for wasm32" does not prove the tiles actually draw in a
# browser. This builds the example to wasm, serves it, renders one frame headless on
# a simulated Retina (DPR=2) display, and asserts both halves of the claim.
#
# The two assertions:
#   1. NO image file is served beside the page. The whole point is that the sheet is
#      embedded, so if a .png ever appears in web/ the demo has silently stopped
#      proving anything — a non-blank frame alone could then come from a fetch.
#   2. The rendered frame is non-blank, so the tiles really did decode from the
#      embedded bytes and reach the GPU.
#   Together those two say: the atlas rendered, and it cannot have come from a file.
#
# What this does NOT catch: a wrong TILE (bad grid maths) still yields a non-blank
# frame. For that, EYEBALL the saved screenshot — it should show 12 tiles in three
# rows (blue idle / green walk / orange run) labelled 0-11, plus a red
# `App::asset_failures()` panel naming the deliberate corrupt embed.
#
# Optional *local* check, not a CI gate (CI has no Chrome/GPU).
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the Cargo.lock crate
#   Google Chrome or Chromium (set $CHROME to override auto-detection)
#
# Usage:
#   scripts/embedded_atlas_smoke.sh                 # build + render + assert
#   CHROME=/path/to/chrome scripts/embedded_atlas_smoke.sh
#   SMOKE_KEEP=1 scripts/embedded_atlas_smoke.sh    # keep the screenshot
#
# Exit codes: 0 = pass · 1 = assertion failed · 2 = environment
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WEB_DIR="$PROJECT_ROOT/examples/embedded_atlas/web"

PORT="${SMOKE_PORT:-8086}"                 # static-file server port (configurable)
SHOT="${SMOKE_SHOT:-/tmp/embedded_atlas_smoke.png}"
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

PROFILE="$(mktemp -d -t chrome_ea_smoke.XXXXXX)"
HTTPD_PID=""
cleanup() {
  [[ -n "$HTTPD_PID" ]] && kill "$HTTPD_PID" 2>/dev/null || true
  pkill -f "$PROFILE" 2>/dev/null || true
  rm -rf "$PROFILE"
}
trap cleanup EXIT

echo ">>> [1/5] building embedded_atlas -> wasm (release)..."
"$WEB_DIR/build.sh" >/dev/null

# Assertion 1 — structural, and it must run BEFORE the render so a stray asset can
# never be masked by a frame that happened to draw.
echo ">>> [2/5] checking nothing image-like is served beside the page..."
strays="$(find "$WEB_DIR" \( -name '*.png' -o -name '*.jpg' -o -name '*.jpeg' -o -name '*.gif' -o -name '*.webp' \) -print 2>/dev/null || true)"
if [[ -n "$strays" ]]; then
  echo "  FAIL: an image file is served beside the page — the embedded-atlas claim is void:" >&2
  echo "$strays" | sed 's/^/         /' >&2
  exit 1
fi
echo "  ok  : no image file under $WEB_DIR -> a rendered tile CANNOT have been fetched"

# Refuse to render against a *stale* server on $PORT (e.g. an orphaned http.server
# from a previous run): it would serve a different page and the byte-size check
# below would FALSE-PASS. centered_text_smoke.sh guards its port the same way.
if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  echo "FAIL: port $PORT is already in use — stop it (or set SMOKE_PORT) so we serve OUR page" >&2
  exit 2
fi

echo ">>> [3/5] serving $WEB_DIR on :$PORT..."
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

echo ">>> [4/5] rendering one frame headless (DPR=2)..."
rm -f "$SHOT"
# --force-device-scale-factor=2 reproduces the Retina/HiDPI path. SwiftShader lets
# headless Chrome do WebGL2 without a real GPU. ?autostart=1 makes index.html call
# run_embedded_atlas() without the (human-only) Start click. Run Chrome backgrounded:
# under SwiftShader it often hangs on *exit* after the screenshot is on disk, so we
# poll for the PNG to appear + stop growing, then reap it ourselves.
"$CHROME" --headless=new \
  --enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader \
  --force-device-scale-factor=2 --window-size=820,470 \
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

# Assertion 2 — the frame actually drew.
echo ">>> [5/5] checking the rendered frame..."
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
  echo ">>> EMBEDDED_ATLAS SMOKE: FAIL" >&2
  echo "    screenshot : $SHOT" >&2
  exit 1
fi
echo ">>> EMBEDDED_ATLAS SMOKE: PASS (embedded sheet rendered on the web, with nothing to fetch)"
echo "    eyeball $SHOT — 12 tiles in three rows labelled 0-11, plus the red asset_failures() panel"
