#!/usr/bin/env bash
# wasm render + network smoke test.
#
# Why this exists: CI builds wasm but never *runs* it, so three latent wasm
# render bugs (HiDPI viewport halving, no default font, canvas stretch) stayed
# invisible for months — nothing ever rendered a wasm frame and looked at it.
# This script does exactly that: it builds the `coin_race` example to wasm,
# stands up the authoritative server + a static file server, renders one frame
# headless on a simulated Retina (DPR=2) display, and saves the screenshot.
#
# What it AUTOMATICALLY catches (the catastrophic class — the most likely
# regressions and the cheapest to detect reliably):
#   - wasm fails to load / panics on start          -> never connects, blank shot
#   - the WebSocket / network path breaks            -> server logs no connection
#   - rendering produces nothing                     -> blank/degenerate frame
# What it does NOT catch automatically: the subtle 3-bug class (off-screen
# sprites, missing text, shifted/clipped text). Each yields a wrong-but-NOT-blank
# frame — empirically the HiDPI-reverted frame is actually *larger* than a good
# one — so no cheap scalar (byte size, pixel count) separates them. For those,
# EYEBALL the saved screenshot (the script always writes + prints its path);
# a reference-image diff would be needed to automate it, at the cost of
# determinism + cross-Chrome fragility (deliberately out of scope).
#
# It is an *optional local* check, not a CI gate: CI has no Chrome and no GPU.
# Run it before shipping wasm-affecting changes, then glance at the screenshot.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen
#                                                    # crate version in Cargo.lock
#   Google Chrome or Chromium (set $CHROME to override auto-detection)
#
# Usage:
#   scripts/wasm_smoke.sh                 # build + render + assert
#   CHROME=/path/to/chrome scripts/wasm_smoke.sh
#   SMOKE_KEEP=1 scripts/wasm_smoke.sh    # keep the screenshot + server log
#
# Exit codes: 0 = pass · 1 = render/network assertion failed · 2 = environment
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WEB_DIR="$PROJECT_ROOT/examples/games/coin_race/web"

PORT="${SMOKE_PORT:-8083}"             # static-file server port (configurable)
SERVER_PORT=9002                       # hardcoded in server.rs + the client URL
SHOT="${SMOKE_SHOT:-/tmp/wasm_smoke.png}"
SRV_LOG="$(mktemp -t coin_race_server.XXXXXX).log"
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

# ── refuse to clobber an already-running server on the fixed port ─────────────
if lsof -nP -iTCP:"$SERVER_PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  echo "FAIL: port $SERVER_PORT is already in use — stop the running coin_race_server first" >&2
  exit 2
fi

PROFILE="$(mktemp -d -t chrome_smoke.XXXXXX)"
SERVER_PID=""; HTTPD_PID=""
cleanup() {
  [[ -n "$SERVER_PID" ]] && kill "$SERVER_PID" 2>/dev/null || true
  [[ -n "$HTTPD_PID"  ]] && kill "$HTTPD_PID"  2>/dev/null || true
  pkill -f "$PROFILE" 2>/dev/null || true
  rm -rf "$PROFILE"
  if [[ "${SMOKE_KEEP:-0}" != "1" ]]; then rm -f "$SRV_LOG"; fi
}
trap cleanup EXIT

echo ">>> [1/5] building coin_race_game -> wasm (release)..."
"$WEB_DIR/build.sh" >/dev/null

echo ">>> [2/5] starting coin_race_server on :$SERVER_PORT..."
# Build first, then exec the binary directly: `cargo run` would fork the server
# as a *child*, and killing the cargo wrapper at cleanup would orphan it (it
# keeps holding port $SERVER_PORT). Running the binary makes $SERVER_PID the
# server itself, so cleanup actually frees the port.
( cd "$PROJECT_ROOT" && cargo build --quiet --example coin_race_server )
"$PROJECT_ROOT/target/debug/examples/coin_race_server" >"$SRV_LOG" 2>&1 &
SERVER_PID=$!
for _ in $(seq 1 40); do
  grep -q "authoritative server" "$SRV_LOG" && break
  kill -0 "$SERVER_PID" 2>/dev/null || { echo "FAIL: server exited early" >&2; cat "$SRV_LOG" >&2; exit 1; }
  sleep 0.5
done

echo ">>> [3/5] serving $WEB_DIR on :$PORT..."
( cd "$WEB_DIR" && python3 -m http.server "$PORT" ) >/dev/null 2>&1 &
HTTPD_PID=$!
sleep 1

echo ">>> [4/5] rendering one frame headless (DPR=2)..."
rm -f "$SHOT"
# --force-device-scale-factor=2 is load-bearing: it reproduces the Retina/HiDPI
# path. SwiftShader lets headless Chrome do WebGL2 without a real GPU.
# Run Chrome in the background: under SwiftShader it frequently hangs on *exit*
# long after the screenshot is on disk, so we poll for the PNG to appear and
# stop growing, then reap the instance ourselves instead of waiting on it.
"$CHROME" --headless=new \
  --enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader \
  --force-device-scale-factor=2 --window-size=800,600 \
  --hide-scrollbars --virtual-time-budget=5000 \
  --user-data-dir="$PROFILE" \
  --screenshot="$SHOT" \
  "http://localhost:$PORT/" >/dev/null 2>&1 &
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

echo ">>> [5/5] checking results..."
fail=0

# (a) network path (primary signal — Chrome-version-independent): the browser
#     tab must have completed a WebSocket handshake with the server.
if grep -q "connected" "$SRV_LOG"; then
  echo "  ok  : server logged a client connection (wasm WebSocket path works)"
else
  echo "  FAIL: server never logged a connection — wasm WebSocket path broke" >&2
  fail=1
fi

# (b) render (secondary signal): the screenshot must be non-trivial. A blank
#     canvas is ~4KB; any real frame (sprites/text) is far larger.
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
  echo ">>> WASM SMOKE: FAIL" >&2
  echo "    server log : $SRV_LOG  (SMOKE_KEEP=1 to retain)" >&2
  echo "    screenshot : $SHOT" >&2
  exit 1
fi
echo ">>> WASM SMOKE: PASS (run + connect + non-blank render)"
echo "    eyeball $SHOT for subtle geometry/text regressions (not auto-checked)"
