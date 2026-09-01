#!/usr/bin/env bash
# Browser smoke for the engine's **wasm WebSocket path**.
#
# Runs `netplay_game`'s wasm build in headless Chrome against a real native `netplay_server`, and
# asserts the handshake completed and that entities streamed in over the browser's socket. The page
# writes its verdict into `document.title` (`NETPLAY_CHECK: PASS — …`); this script reads it live
# over Chrome's DevTools endpoint.
#
# ── Why this one ────────────────────────────────────────────────────────────────────────────────
#
# `src/network/wasm_impl.rs` is a wholly separate implementation from the native tungstenite client
# — different queueing, different overflow policy, different open semantics. Since the 2026-08-19
# deletion NOTHING has executed a line of it. The two native↔wasm contracts recorded in
# docs/MODULE_MAP.md (a send before the socket opens; the inbound overflow policy) were both found
# and fixed in v0.150.2 *because* a browser smoke existed to catch them.
#
# It also covers a gap the `wasm` build job structurally cannot: on 2026-08-21 `netplay_game`'s wasm
# entry point was `#[no_mangle] pub extern "C"` rather than `#[wasm_bindgen]`. It compiled,
# `build_wasm_examples.sh` went green, and the generated JS contained ZERO occurrences of the
# function `index.html` imports — the game could not start at all. Only loading the page finds that.
#
# ⚠️ This does NOT assert that anything was drawn. Reading pixels back out of a wgpu canvas needs
# `preserveDrawingBuffer`, which changes how the surface is configured, so such a check would be
# measuring a different configuration than the one the game ships. The browser render path still has
# no pixel-level gate; said plainly rather than implied.
#
# ── Prerequisites ───────────────────────────────────────────────────────────────────────────────
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
#   Google Chrome or Chromium (set $CHROME to override auto-detection)
#
# Usage:
#   scripts/netplay_web_smoke.sh
#   SMOKE_PORT=8091 SMOKE_DBG=9311 scripts/netplay_web_smoke.sh
#
# Exit codes: 0 = pass · 1 = the handshake/streaming assertion failed · 2 = environment
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WEB_DIR="$PROJECT_ROOT/examples/netplay_game/web"

PORT="${SMOKE_PORT:-8091}"
DBG="${SMOKE_DBG:-9311}"
# ⚠️ Not configurable. The wasm build has no environment to read, so `protocol.rs`'s `server_addr()`
# always returns the compiled-in constant on that target — the page can only ever dial this.
SERVER_ADDR="127.0.0.1:9006"

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

PROFILE="$(mktemp -d -t chrome_netplay_smoke.XXXXXX)"
HTTPD_PID=""
CHROME_PID=""
SERVER_PID=""
set +m
cleanup() {
  if [[ -n "$CHROME_PID" ]]; then kill "$CHROME_PID" 2>/dev/null; wait "$CHROME_PID" 2>/dev/null; fi
  if [[ -n "$HTTPD_PID" ]]; then kill "$HTTPD_PID" 2>/dev/null; wait "$HTTPD_PID" 2>/dev/null; fi
  if [[ -n "$SERVER_PID" ]]; then kill "$SERVER_PID" 2>/dev/null; wait "$SERVER_PID" 2>/dev/null; fi
  pkill -f "$PROFILE" 2>/dev/null
  rm -rf "$PROFILE"
  return 0
}
trap cleanup EXIT

echo ">>> [1/5] building netplay_game -> wasm (release)..."
if ! bash "$WEB_DIR/build.sh" >/dev/null; then
  echo "FAIL: the wasm build failed — run $WEB_DIR/build.sh to see why" >&2
  exit 2
fi

echo ">>> [2/5] building and starting the native netplay_server on $SERVER_ADDR..."
if ! cargo build --quiet --example netplay_server; then
  echo "FAIL: could not build netplay_server" >&2
  exit 2
fi
if lsof -nP -iTCP:"${SERVER_ADDR##*:}" -sTCP:LISTEN >/dev/null 2>&1; then
  echo "FAIL: ${SERVER_ADDR} is already in use — stop whatever is on it." >&2
  echo "      The wasm client has no environment to read, so it can only dial this address." >&2
  exit 2
fi
NETPLAY_ADDR="$SERVER_ADDR" "$PROJECT_ROOT/target/debug/examples/netplay_server" >/dev/null 2>&1 &
SERVER_PID=$!
# `NetworkClient::connect` dials once and never retries, so the page must not load before the
# server binds — connecting first is not a slower path, it is a guaranteed failure.
bound=0
for _ in $(seq 1 100); do
  if bash -c "exec 3<>/dev/tcp/${SERVER_ADDR%:*}/${SERVER_ADDR##*:}" 2>/dev/null; then bound=1; break; fi
  kill -0 "$SERVER_PID" 2>/dev/null || break
  sleep 0.1
done
if [[ "$bound" -ne 1 ]]; then
  echo "FAIL: netplay_server never bound $SERVER_ADDR" >&2
  exit 2
fi

echo ">>> [3/5] serving $WEB_DIR on :$PORT..."
if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  echo "FAIL: port $PORT is already in use — stop it (or set SMOKE_PORT) so we serve OUR page" >&2
  exit 2
fi
python3 -m http.server "$PORT" --directory "$WEB_DIR" >/dev/null 2>&1 &
HTTPD_PID=$!
# ⚠️ Wait for the LISTEN, not for a second to pass. `sleep 1` is a guess about how fast python
# starts, and `kill -0` cannot correct it — the process exists long before it binds. On a loaded
# runner Chrome then dials a port nothing is on yet, the page never loads, no verdict is ever
# written, and the poll further down times out with a message that blames the ENGINE for a server
# that was not up. Same loop the sibling servers in these smokes already use.
serving=0
for _ in $(seq 1 100); do
  if bash -c "exec 3<>/dev/tcp/127.0.0.1/$PORT" 2>/dev/null; then serving=1; break; fi
  kill -0 "$HTTPD_PID" 2>/dev/null || break
  sleep 0.1
done
if [[ "$serving" -ne 1 ]]; then
  echo "FAIL: http.server never began serving :$PORT within 10 s — this is the server, not the" >&2
  echo "      engine. It either died on startup or never bound." >&2
  exit 2
fi

echo ">>> [4/5] running the WebSocket check headless..."
# ⚠️ NOT --virtual-time-budget: the handshake and the server's 12 Hz snapshots happen on a wall
# clock, and fast-forwarding virtual time races past them and reads an empty world off a working
# socket. Same trap CLAUDE.md records for ENGINE_CAPTURE, one layer out.
"$CHROME" --headless=new \
  --enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader \
  --remote-debugging-port="$DBG" \
  --user-data-dir="$PROFILE" \
  "http://localhost:$PORT/check.html" >/dev/null 2>&1 &
CHROME_PID=$!

verdict=""
for _ in $(seq 1 90); do          # up to ~45 s; the page's own deadline is 25 s
  titles="$(curl -s "http://localhost:$DBG/json" 2>/dev/null || true)"
  if [[ "$titles" == *"NETPLAY_CHECK:"* ]]; then
    verdict="$(printf '%s' "$titles" | grep -oE 'NETPLAY_CHECK: [^"]*' | head -1)"
    break
  fi
  kill -0 "$CHROME_PID" 2>/dev/null || break
  sleep 0.5
done

echo ">>> [5/5] checking results..."
if [[ "$verdict" == "NETPLAY_CHECK: PASS"* ]]; then
  echo ">>> NETPLAY WEB SMOKE: PASS — $verdict"
  exit 0
fi

if [[ "$verdict" == "NETPLAY_CHECK: FAIL"* ]]; then
  echo "  FAIL: the page reported ${verdict#NETPLAY_CHECK: }" >&2
else
  echo "  FAIL: no NETPLAY_CHECK verdict appeared — the page did not finish." >&2
  echo "        Run the server and serve the page by hand, then watch the console:" >&2
  echo "        cargo run --example netplay_server" >&2
  echo "        python3 -m http.server $PORT --directory $WEB_DIR" >&2
fi
echo ">>> NETPLAY WEB SMOKE: FAIL" >&2
exit 1
