#!/usr/bin/env bash
# The only check in the tree that takes a FAILURE path on purpose.
#
# Runs the `wasm_failpaths` harness in headless Chrome against a native echo server and asserts two
# things that are supposed to go wrong go wrong *visibly*:
#   1. a 404 asset fetch reaches `asset_failures()` — not just the handle's own load state
#   2. a `send_text` issued while the socket is still CONNECTING survives and is echoed back
# The page writes its verdict into `document.title` (`FAILPATH_CHECK: PASS — …`); this script reads
# it live over Chrome's DevTools endpoint.
#
# ── Why this one is different from every other smoke ────────────────────────────────────────────
#
# Every other check in this repo passes when nothing goes wrong. That is a blind spot the engine has
# been bitten by twice, and both times the broken handler shipped GREEN:
#
#   * v0.150.1 — a web 404 set `AssetLoadState::Failed` but never called `record_failure`, so
#     `asset_failures()` stayed empty and `set_strict_assets` never fired. Both are documented as
#     the way to refuse to start on a missing asset; both were native-only in practice.
#   * v0.150.2 — a `send_text` before the socket opened was handed to a CONNECTING socket, which
#     throws. The message vanished on the web and nowhere else, so a game silently lost its join
#     packet in browsers only.
#
# Both fixes shipped compile-verified only: no automated check could reach either. This is what
# closes that, and it was missing from the rebuild plan's own list of four browser smokes — see
# docs/NEXT_WORK.md.
#
# ⚠️ A 404 in the network log and a magenta square on the canvas are EXPECTED. They are the check
# working. Do not "fix" them.
#
# ── Prerequisites ───────────────────────────────────────────────────────────────────────────────
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
#   Google Chrome or Chromium (set $CHROME to override auto-detection)
#
# Usage:
#   scripts/wasm_failpaths_web_smoke.sh
#   SMOKE_PORT=8092 SMOKE_DBG=9312 scripts/wasm_failpaths_web_smoke.sh
#
# Exit codes: 0 = pass · 1 = a failure path is not reported · 2 = environment
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WEB_DIR="$PROJECT_ROOT/examples/wasm_failpaths/web"

PORT="${SMOKE_PORT:-8092}"
DBG="${SMOKE_DBG:-9312}"
# ⚠️ Not configurable. A wasm build has no environment to read, so the page can only ever dial the
# address compiled into `main.rs`. Change it in both files or in neither.
ECHO_ADDR="127.0.0.1:9007"

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

PROFILE="$(mktemp -d -t chrome_failpaths_smoke.XXXXXX)"
ECHO_LOG="$(mktemp -t failpaths_echo.XXXXXX)"
HTTPD_PID=""
CHROME_PID=""
ECHO_PID=""
set +m
cleanup() {
  if [[ -n "$CHROME_PID" ]]; then kill "$CHROME_PID" 2>/dev/null; wait "$CHROME_PID" 2>/dev/null; fi
  if [[ -n "$HTTPD_PID" ]]; then kill "$HTTPD_PID" 2>/dev/null; wait "$HTTPD_PID" 2>/dev/null; fi
  if [[ -n "$ECHO_PID" ]]; then kill "$ECHO_PID" 2>/dev/null; wait "$ECHO_PID" 2>/dev/null; fi
  pkill -f "$PROFILE" 2>/dev/null
  rm -rf "$PROFILE" "$ECHO_LOG"
  return 0
}
trap cleanup EXIT

echo ">>> [1/5] building wasm_failpaths -> wasm (release)..."
if ! bash "$WEB_DIR/build.sh" >/dev/null; then
  echo "FAIL: the wasm build failed — run $WEB_DIR/build.sh to see why" >&2
  exit 2
fi

echo ">>> [2/5] building and starting the echo server on $ECHO_ADDR..."
if ! cargo build --quiet --example wasm_failpaths_echo_server; then
  echo "FAIL: could not build wasm_failpaths_echo_server" >&2
  exit 2
fi
if lsof -nP -iTCP:"${ECHO_ADDR##*:}" -sTCP:LISTEN >/dev/null 2>&1; then
  echo "FAIL: ${ECHO_ADDR} is already in use — stop whatever is on it." >&2
  echo "      The wasm page has no environment to read, so it can only dial this address." >&2
  exit 2
fi
FAILPATHS_ADDR="$ECHO_ADDR" "$PROJECT_ROOT/target/debug/examples/wasm_failpaths_echo_server" \
  >"$ECHO_LOG" 2>&1 &
ECHO_PID=$!
# `NetworkClient::connect` dials once and never retries, so the page must not load before the
# server binds — connecting first is a guaranteed failure, not a slower path.
bound=0
for _ in $(seq 1 100); do
  if bash -c "exec 3<>/dev/tcp/${ECHO_ADDR%:*}/${ECHO_ADDR##*:}" 2>/dev/null; then bound=1; break; fi
  kill -0 "$ECHO_PID" 2>/dev/null || break
  sleep 0.1
done
if [[ "$bound" -ne 1 ]]; then
  echo "FAIL: the echo server never bound $ECHO_ADDR" >&2
  sed 's/^/    /' "$ECHO_LOG" >&2
  exit 2
fi

echo ">>> [3/5] serving $WEB_DIR on :$PORT..."
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

echo ">>> [4/5] taking both failure paths headless..."
# ⚠️ NOT --virtual-time-budget: the fetch that 404s and the WebSocket handshake both resolve on a
# wall clock, and fast-forwarding virtual time races past them.
"$CHROME" --headless=new \
  --enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader \
  --remote-debugging-port="$DBG" \
  --user-data-dir="$PROFILE" \
  "http://localhost:$PORT/" >/dev/null 2>&1 &
CHROME_PID=$!

verdict=""
for _ in $(seq 1 90); do          # up to ~45 s; the page's own deadline is 25 s
  titles="$(curl -s "http://localhost:$DBG/json" 2>/dev/null || true)"
  if [[ "$titles" == *"FAILPATH_CHECK:"* ]]; then
    verdict="$(printf '%s' "$titles" | grep -oE 'FAILPATH_CHECK: [^"]*' | head -1)"
    break
  fi
  kill -0 "$CHROME_PID" 2>/dev/null || break
  sleep 0.5
done

echo ">>> [5/5] checking results..."
if [[ "$verdict" == "FAILPATH_CHECK: PASS"* ]]; then
  echo ">>> WASM FAILPATHS SMOKE: PASS — $verdict"
  exit 0
fi

if [[ "$verdict" == "FAILPATH_CHECK: FAIL"* ]]; then
  echo "  FAIL: the page reported ${verdict#FAILPATH_CHECK: }" >&2
else
  echo "  FAIL: no FAILPATH_CHECK verdict appeared — the page did not finish." >&2
fi
# The echo server is an INDEPENDENT witness: if it logged the sentinel, the pre-open send worked and
# the other half is what broke. Worth printing either way — it tells you which half to look at
# without re-running anything.
echo "  --- echo server log (independent witness) ---" >&2
sed 's/^/    /' "$ECHO_LOG" >&2
echo ">>> WASM FAILPATHS SMOKE: FAIL" >&2
exit 1
