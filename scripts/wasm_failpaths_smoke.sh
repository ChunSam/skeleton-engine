#!/usr/bin/env bash
# wasm fail-path smoke — the checks that only pass if FAILURE handling works.
#
# Why this exists: every other browser smoke drives a path that is supposed to succeed and passes
# when nothing goes wrong. That is a blind spot, and the engine has been bitten through it twice.
# Both of these shipped COMPILE-VERIFIED ONLY, because no automated check could reach them:
#
#   v0.150.1  a wasm image fetch that 404s set `AssetLoadState::Failed` but never called
#             `record_failure`, so `asset_failures()` stayed empty and `set_strict_assets` never
#             fired — the two documented ways to refuse to start on a missing asset were
#             native-only in practice, and the web just painted magenta in silence.
#   v0.150.2  a `send` issued before the socket opened was handed to a CONNECTING WebSocket,
#             which throws — the message vanished. Native queues it and delivers it on open, so
#             the same game lost its join packet on the web and nowhere else.
#
# The `wasm_failpaths` page takes both paths ON PURPOSE and stamps
# `FAILPATH_CHECK: PASS (n/n)` / `FAILPATH_CHECK: FAIL: <step>` into the document title. This
# script reads that title live over Chrome's DevTools endpoint (so the failing step travels with a
# failure) and asserts PASS. It is self-verdicting: it cannot pass by "nothing happened".
#
# ⚠️ Sabotage-check it if you change either fix: revert the fix, run this, and it MUST go red.
# A fail-path check that stays green when the fix is reverted is worth less than no check at all,
# because it looks like coverage.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
#   Google Chrome or Chromium (set $CHROME to override auto-detection)
#
# Usage:
#   scripts/wasm_failpaths_smoke.sh
#   CHROME=/path/to/chrome scripts/wasm_failpaths_smoke.sh
#   SMOKE_KEEP=1 scripts/wasm_failpaths_smoke.sh     # keep the server log
#
# Exit codes: 0 = pass · 1 = a fail-path assertion failed · 2 = environment
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

WEB_DIR="$PROJECT_ROOT/examples/wasm_failpaths/web"
PORT="${SMOKE_PORT:-8092}"          # static-file server (8083-8091 are taken by other smokes)
DBG="${SMOKE_DBG:-9224}"            # Chrome DevTools endpoint
ECHO_ADDR="${FAILPATHS_ADDR:-127.0.0.1:9004}"
SENTINEL="PREOPEN-SENTINEL"

CHROME="${CHROME:-}"
if [[ -z "$CHROME" ]]; then
  for candidate in \
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    "/Applications/Chromium.app/Contents/MacOS/Chromium" \
    "$(command -v google-chrome || true)" \
    "$(command -v chromium || true)" \
    "$(command -v chromium-browser || true)"; do
    [[ -n "$candidate" && -x "$candidate" ]] && CHROME="$candidate" && break
  done
fi
if [[ -z "$CHROME" ]]; then
  echo "FAIL: no Chrome/Chromium found — set \$CHROME to its path" >&2
  exit 2
fi

PROFILE="$(mktemp -d -t chrome_failpaths_smoke.XXXXXX)"
SRV_LOG="$(mktemp -t failpaths_server.XXXXXX)"
HTTPD_PID=""
SRV_PID=""
CHROME_PID=""
cleanup() {
  [[ -n "$CHROME_PID" ]] && kill "$CHROME_PID" 2>/dev/null || true
  [[ -n "$HTTPD_PID" ]] && kill "$HTTPD_PID" 2>/dev/null || true
  [[ -n "$SRV_PID" ]] && kill "$SRV_PID" 2>/dev/null || true
  pkill -f "$PROFILE" 2>/dev/null || true
  rm -rf "$PROFILE"
  if [[ "${SMOKE_KEEP:-0}" = "1" ]]; then
    echo "    server log kept at $SRV_LOG"
  else
    rm -f "$SRV_LOG"
  fi
}
trap cleanup EXIT

echo ">>> [1/5] building wasm_failpaths -> wasm (release)..."
# Invoke via `bash` so it works regardless of the script's executable bit.
bash "$WEB_DIR/build.sh" >/dev/null

echo ">>> [2/5] starting the echo server on $ECHO_ADDR..."
cargo build --example wasm_failpaths_echo_server >/dev/null 2>&1
FAILPATHS_ADDR="$ECHO_ADDR" "$PROJECT_ROOT/target/debug/examples/wasm_failpaths_echo_server" \
  > "$SRV_LOG" 2>&1 &
SRV_PID=$!
# Wait for the bind rather than sleeping and hoping — the page connects once and does not retry,
# so a browser that starts first is a guaranteed failure, not a slower path.
for _ in $(seq 1 50); do
  grep -q "listening on" "$SRV_LOG" 2>/dev/null && break
  kill -0 "$SRV_PID" 2>/dev/null || break
  sleep 0.2
done
if ! grep -q "listening on" "$SRV_LOG" 2>/dev/null; then
  echo "FAIL: echo server never bound $ECHO_ADDR" >&2
  sed -n '1,20p' "$SRV_LOG" >&2
  exit 2
fi

echo ">>> [3/5] serving $WEB_DIR on :$PORT..."
# Refuse a stale server on $PORT: an orphaned http.server from a prior run would serve a
# different page, producing no verdict and a confusing FAIL.
if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  echo "FAIL: port $PORT is already in use — stop it (or set SMOKE_PORT) so we serve OUR page" >&2
  exit 2
fi
# `--directory` (not a `( cd && python3 )` subshell) so $! IS python, not a subshell whose python
# child survives `kill $HTTPD_PID` and orphans itself onto $PORT.
python3 -m http.server "$PORT" --directory "$WEB_DIR" >/dev/null 2>&1 &
HTTPD_PID=$!
sleep 1
if ! kill -0 "$HTTPD_PID" 2>/dev/null; then
  echo "FAIL: http.server failed to start on :$PORT" >&2
  exit 2
fi

echo ">>> [4/5] running the fail-path checks headless..."
# Real time, NOT --virtual-time-budget: both checks wait on real browser I/O (a fetch that must
# 404, a WebSocket handshake), and a virtual clock that fast-forwards JS timers races ahead of
# them. SwiftShader runs WebGL without a GPU.
"$CHROME" --headless=new \
  --enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader \
  --remote-debugging-port="$DBG" \
  --user-data-dir="$PROFILE" \
  "http://localhost:$PORT/" >/dev/null 2>&1 &
CHROME_PID=$!

verdict=""
for _ in $(seq 1 80); do          # up to ~40s real time; the page's own deadline is 20s
  titles="$(curl -s "http://localhost:$DBG/json" 2>/dev/null || true)"
  if [[ "$titles" == *"FAILPATH_CHECK:"* ]]; then
    verdict="$(printf '%s' "$titles" | grep -oE 'FAILPATH_CHECK: [^"]*' | head -1)"
    break
  fi
  kill -0 "$CHROME_PID" 2>/dev/null || break   # Chrome exited on its own
  sleep 0.5
done
# SwiftShader headless Chrome often hangs on exit, so reap it ourselves rather than waiting.
kill "$CHROME_PID" 2>/dev/null || true
pkill -f "$PROFILE" 2>/dev/null || true

echo ">>> [5/5] checking results..."

# Independent witness: the server's own log. If the page says PASS but the server never logged
# the pre-open payload, something is wrong with the check itself, not the engine.
if grep -q "recv text: $SENTINEL" "$SRV_LOG" 2>/dev/null; then
  echo "  ok  : the echo server received the pre-open payload (server log)"
  server_saw_preopen=1
else
  echo "  MISS: the echo server never logged '$SENTINEL'"
  server_saw_preopen=0
fi

if [[ "$verdict" == "FAILPATH_CHECK: PASS"* ]]; then
  if [[ "$server_saw_preopen" != "1" ]]; then
    echo "  FAIL: the page reported PASS but the server never saw the pre-open payload —" >&2
    echo "        distrust the page, not the server: the check is measuring the wrong thing" >&2
    echo ">>> WASM FAIL-PATH SMOKE: FAIL" >&2
    exit 1
  fi
  echo ">>> WASM FAIL-PATH SMOKE: PASS — $verdict"
  echo "    (a 404 fetch reached asset_failures(); a pre-open send survived and echoed back)"
  exit 0
fi

if [[ "$verdict" == "FAILPATH_CHECK: FAIL"* ]]; then
  echo "  FAIL: the page reported a failing check: ${verdict#FAILPATH_CHECK: }" >&2
else
  echo "  FAIL: no FAILPATH_CHECK verdict appeared — the page did not finish" >&2
  echo "        (wasm may have failed to start; check the page manually)" >&2
fi
echo ">>> WASM FAIL-PATH SMOKE: FAIL" >&2
exit 1
