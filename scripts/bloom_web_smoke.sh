#!/usr/bin/env bash
# wasm bloom render smoke for the `bloom` example.
#
# Why this exists: the mip-chain "dual filter" bloom (v0.67.0) renders the scene into an Rgba16Float
# HDR intermediate and blurs the highlights through a pyramid of Rgba16Float mip render targets.
# Those float render targets are usable on WebGL2 only with the EXT_color_buffer_float extension, so
# whether the whole HDR + mip-chain bloom pipeline actually runs in a browser differs from the
# desktop (Metal/Vulkan/DX12) case. CI builds wasm but never *runs* it, and the live wgpu pipeline
# needs a real GPU/backend. This boots the example headless under SwiftShader (the lowest-common-
# denominator WebGL2 backend) and confirms the pipeline renders without panicking.
#
# The example self-checks by surviving ~30 frames of HDR + mip-chain bloom and then writing the
# verdict to the page title (`BLOOM_WEB_CHECK: PASS (1/1)`). A boot panic (e.g. an unrenderable
# Rgba16Float target) fires console_error_panic_hook and no verdict ever appears. This reads the
# title live over Chrome's DevTools endpoint and asserts PASS.
#
# A **CI gate** as of v0.143.17 (the `wasm-smokes` job; swiftshader supplies the GPU). Run it after touching the bloom
# pass (src/renderer/bloom.rs, bloom.wgsl), the HDR post intermediate, or the example.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
#   Google Chrome or Chromium (set $CHROME to override auto-detection)
#
# Usage:
#   scripts/bloom_web_smoke.sh
#   CHROME=/path/to/chrome scripts/bloom_web_smoke.sh
#
# Exit codes: 0 = pass · 1 = no PASS verdict (pipeline did not render) · 2 = environment
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WEB_DIR="$PROJECT_ROOT/examples/bloom/web"

PORT="${SMOKE_PORT:-8090}"
DBG="${SMOKE_DBG:-9226}"

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

PROFILE="$(mktemp -d -t chrome_bloom_smoke.XXXXXX)"
HTTPD_PID=""
cleanup() {
  [[ -n "$HTTPD_PID" ]] && kill "$HTTPD_PID" 2>/dev/null || true
  pkill -f "$PROFILE" 2>/dev/null || true
  rm -rf "$PROFILE"
}
trap cleanup EXIT

echo ">>> [1/4] building bloom -> wasm (release)..."
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

echo ">>> [3/4] rendering the bloom pipeline headless (SwiftShader WebGL2)..."
# ?autostart=1 boots the example without the (human-only) Start click. SwiftShader is the
# lowest-common-denominator WebGL2 backend; the DevTools endpoint exposes each tab's live title.
"$CHROME" --headless=new \
  --enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader \
  --window-size=880,460 --hide-scrollbars \
  --remote-debugging-port="$DBG" \
  --user-data-dir="$PROFILE" \
  "http://localhost:$PORT/?autostart=1" >/dev/null 2>&1 &
CHROME_PID=$!

verdict=""
for _ in $(seq 1 60); do          # up to ~30s
  titles="$(curl -s "http://localhost:$DBG/json" 2>/dev/null || true)"
  if [[ "$titles" == *"BLOOM_WEB_CHECK:"* ]]; then
    verdict="$(printf '%s' "$titles" | grep -oE 'BLOOM_WEB_CHECK: [^"]*' | head -1)"
    break
  fi
  kill -0 "$CHROME_PID" 2>/dev/null || break
  sleep 0.5
done
kill "$CHROME_PID" 2>/dev/null || true
pkill -f "$PROFILE" 2>/dev/null || true

echo ">>> [4/4] checking results..."
if [[ "$verdict" == "BLOOM_WEB_CHECK: PASS"* ]]; then
  echo ">>> BLOOM WEB SMOKE: PASS — $verdict"
  echo "    (HDR + mip-chain bloom renders on WebGL2: Rgba16Float intermediate + mip targets OK)"
  exit 0
fi

echo "  FAIL: no BLOOM_WEB_CHECK: PASS verdict appeared — the HDR + bloom pipeline did not render" >&2
echo ">>> BLOOM WEB SMOKE: FAIL" >&2
exit 1
