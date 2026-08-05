#!/usr/bin/env bash
# wasm render-format-capability smoke for the `render_format_query` example.
#
# Why this exists: the render-format renderability query (`RenderCapabilities`, v0.65.0) is most
# meaningful on the WebGL2 backend — float render targets like `Rgba16Float` are renderable there
# only with `EXT_color_buffer_float`, so a backend's actual renderability differs from the desktop
# case. CI builds wasm but never *runs* it, and the live `get_texture_format_features` query needs a
# real GPU/backend. This boots the example headless under SwiftShader (the lowest-common-denominator
# WebGL2 backend) and asserts the query is correct cross-platform.
#
# The example self-checks two backend-independent invariants and writes the verdict to the page
# title (`RENDER_FORMAT_QUERY_CHECK: PASS (2/2)` / `... FAIL (n/2)`):
#   - the surface format IS renderable (it is the on-screen render target)
#   - a block-compressed format (Bc1RgbaUnorm) is NOT a color render attachment
# This reads the title live over Chrome's DevTools endpoint and asserts PASS. It also proves the
# example loads + renders on WebGL2 without panicking (no verdict ever appears otherwise).
#
# A **CI gate** as of v0.143.17 (the `wasm-smokes` job; swiftshader supplies the GPU). Run it after touching the
# RenderCapabilities query path (src/renderer/context.rs) or the example.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
#   Google Chrome or Chromium (set $CHROME to override auto-detection)
#
# Usage:
#   scripts/render_format_query_smoke.sh
#   CHROME=/path/to/chrome scripts/render_format_query_smoke.sh
#
# Exit codes: 0 = pass · 1 = query assertion failed / no verdict · 2 = environment
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WEB_DIR="$PROJECT_ROOT/examples/render_format_query/web"

PORT="${SMOKE_PORT:-8089}"
DBG="${SMOKE_DBG:-9225}"

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

PROFILE="$(mktemp -d -t chrome_rfq_smoke.XXXXXX)"
HTTPD_PID=""
cleanup() {
  [[ -n "$HTTPD_PID" ]] && kill "$HTTPD_PID" 2>/dev/null || true
  pkill -f "$PROFILE" 2>/dev/null || true
  rm -rf "$PROFILE"
}
trap cleanup EXIT

echo ">>> [1/4] building render_format_query -> wasm (release)..."
bash "$WEB_DIR/build.sh" >/dev/null

echo ">>> [2/4] serving $WEB_DIR on :$PORT..."
# Refuse a stale server on $PORT (an orphan from a prior run would serve a different page → no
# verdict, a confusing FAIL). `--directory` (not a `( cd && python3 )` subshell) so $! IS python.
if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  echo "FAIL: port $PORT is already in use — stop it (or set SMOKE_PORT) so we serve OUR page" >&2
  exit 2
fi
python3 -m http.server "$PORT" --directory "$WEB_DIR" >/dev/null 2>&1 &
HTTPD_PID=$!
sleep 1
if ! kill -0 "$HTTPD_PID" 2>/dev/null; then
  echo "FAIL: http.server failed to start on :$PORT" >&2
  exit 2
fi

echo ">>> [3/4] running the query check headless (SwiftShader WebGL2)..."
# ?autostart=1 boots the example without the (human-only) Start click. SwiftShader is the
# lowest-common-denominator WebGL2 backend; the DevTools endpoint exposes each tab's live title.
"$CHROME" --headless=new \
  --enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader \
  --window-size=720,520 --hide-scrollbars \
  --remote-debugging-port="$DBG" \
  --user-data-dir="$PROFILE" \
  "http://localhost:$PORT/?autostart=1" >/dev/null 2>&1 &
CHROME_PID=$!

verdict=""
for _ in $(seq 1 60); do          # up to ~30s
  titles="$(curl -s "http://localhost:$DBG/json" 2>/dev/null || true)"
  if [[ "$titles" == *"RENDER_FORMAT_QUERY_CHECK:"* ]]; then
    verdict="$(printf '%s' "$titles" | grep -oE 'RENDER_FORMAT_QUERY_CHECK: [^"]*' | head -1)"
    break
  fi
  kill -0 "$CHROME_PID" 2>/dev/null || break
  sleep 0.5
done
kill "$CHROME_PID" 2>/dev/null || true
pkill -f "$PROFILE" 2>/dev/null || true

echo ">>> [4/4] checking results..."
if [[ "$verdict" == "RENDER_FORMAT_QUERY_CHECK: PASS"* ]]; then
  echo ">>> RENDER_FORMAT_QUERY SMOKE: PASS — $verdict"
  echo "    (RenderCapabilities query runs on WebGL2: surface renderable + compressed rejected)"
  exit 0
fi

if [[ "$verdict" == "RENDER_FORMAT_QUERY_CHECK: FAIL"* ]]; then
  echo "  FAIL: the example reported a failing check: ${verdict#RENDER_FORMAT_QUERY_CHECK: }" >&2
else
  echo "  FAIL: no RENDER_FORMAT_QUERY_CHECK verdict appeared — the example did not boot/render" >&2
fi
echo ">>> RENDER_FORMAT_QUERY SMOKE: FAIL" >&2
exit 1
