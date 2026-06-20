#!/usr/bin/env bash
# wasm AEAD save/load smoke test (the `localStorage` save backend).
#
# Why this exists: v0.22.0 made save/load/save_versioned/load_migrated cross-platform — on wasm the
# ChaCha20-Poly1305 blob is hex-encoded into `localStorage`. That path was compile-gated +
# native-playtested but never *run* in a browser. This runs the `wasm_save` example headless and
# asserts the round-trip end-to-end:
#   - save() AEAD-encrypts to localStorage; exists() sees it; load() round-trips the struct
#   - the stored value is hex ciphertext, not plaintext
#   - a tampered blob makes load() fail (AEAD authentication)
#   - save_versioned()/load_migrated() round-trips; delete() removes the key
# The example writes the verdict to the document title (`SAVE_CHECK: PASS (n/n)` /
# `...FAIL: <step>`); this script reads it live over Chrome's DevTools endpoint and asserts PASS.
#
# It is an *optional local* check, not a CI gate (CI has no Chrome). Run it after touching
# src/save.rs (the wasm path) or the example.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
#   Google Chrome or Chromium (set $CHROME to override auto-detection)
#
# Usage:
#   scripts/wasm_save_smoke.sh
#   CHROME=/path/to/chrome scripts/wasm_save_smoke.sh
#
# Exit codes: 0 = pass · 1 = round-trip assertion failed · 2 = environment
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WEB_DIR="$PROJECT_ROOT/examples/wasm_save/web"

PORT="${SMOKE_PORT:-8085}"

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

PROFILE="$(mktemp -d -t chrome_save_smoke.XXXXXX)"
HTTPD_PID=""
cleanup() {
  [[ -n "$HTTPD_PID" ]] && kill "$HTTPD_PID" 2>/dev/null || true
  pkill -f "$PROFILE" 2>/dev/null || true
  rm -rf "$PROFILE"
}
trap cleanup EXIT

echo ">>> [1/4] building wasm_save -> wasm (release)..."
bash "$WEB_DIR/build.sh" >/dev/null

echo ">>> [2/4] serving $WEB_DIR on :$PORT..."
# Refuse a stale server on $PORT (an orphaned http.server from a prior run would
# serve a different page → no SAVE_CHECK verdict, a confusing FAIL). centered_text
# defaults to this same port (8085), so leaking python here also breaks that smoke.
if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  echo "FAIL: port $PORT is already in use — stop it (or set SMOKE_PORT) so we serve OUR page" >&2
  exit 2
fi
# `--directory` (not a `( cd && python3 )` subshell) so $! IS python, not a subshell
# whose python *child* survives `kill $HTTPD_PID` and orphans itself onto $PORT.
python3 -m http.server "$PORT" --directory "$WEB_DIR" >/dev/null 2>&1 &
HTTPD_PID=$!
sleep 1
# Confirm OUR server actually came up (a silent bind failure must not mislead).
if ! kill -0 "$HTTPD_PID" 2>/dev/null; then
  echo "FAIL: http.server failed to start on :$PORT" >&2
  exit 2
fi

echo ">>> [3/4] running the round-trip check headless..."
# The example is synchronous (no audio/GPU), but we still read the verdict from the page title over
# the DevTools endpoint (--remote-debugging-port exposes /json with each tab's live title), which is
# robust and matches scripts/wasm_audio_smoke.sh.
DBG="${SMOKE_DBG:-9224}"
"$CHROME" --headless=new \
  --remote-debugging-port="$DBG" \
  --user-data-dir="$PROFILE" \
  "http://localhost:$PORT/" >/dev/null 2>&1 &
CHROME_PID=$!

verdict=""
for _ in $(seq 1 60); do          # up to ~30s
  titles="$(curl -s "http://localhost:$DBG/json" 2>/dev/null || true)"
  if [[ "$titles" == *"SAVE_CHECK:"* ]]; then
    verdict="$(printf '%s' "$titles" | grep -oE 'SAVE_CHECK: [^"]*' | head -1)"
    break
  fi
  kill -0 "$CHROME_PID" 2>/dev/null || break
  sleep 0.5
done
kill "$CHROME_PID" 2>/dev/null || true
pkill -f "$PROFILE" 2>/dev/null || true

echo ">>> [4/4] checking results..."
if [[ "$verdict" == "SAVE_CHECK: PASS"* ]]; then
  echo ">>> WASM SAVE SMOKE: PASS — $verdict"
  echo "    (AEAD save/load + ciphertext-in-localStorage + tamper-detection + versioned round-trip)"
  exit 0
fi

if [[ "$verdict" == "SAVE_CHECK: FAIL"* ]]; then
  echo "  FAIL: the example reported a failing step: ${verdict#SAVE_CHECK: }" >&2
else
  echo "  FAIL: no SAVE_CHECK verdict appeared — the check did not finish" >&2
fi
echo ">>> WASM SAVE SMOKE: FAIL" >&2
exit 1
