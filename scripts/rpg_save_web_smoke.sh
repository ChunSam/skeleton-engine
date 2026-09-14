#!/usr/bin/env bash
# RPG's real save schema/functions in Chrome: v1 restore and v0 migration across full page reloads,
# then AEAD tamper rejection and deletion. The page self-verdicts in document.title.
# Prerequisites: wasm32 target, matching wasm-bindgen-cli (Cargo.lock), Chrome, python3, curl, lsof.
# Usage: SMOKE_PORT=8093 SMOKE_DBG=9313 ./scripts/rpg_save_web_smoke.sh
# Exit codes: 0 pass; 1 page assertion/timeout; 2 environment/build failure.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WEB_DIR="$PROJECT_ROOT/examples/rpg_quest_game/web"
PORT="${SMOKE_PORT:-8093}"
DBG="${SMOKE_DBG:-9313}"
URL="http://127.0.0.1:$PORT/check.html"

CHROME="${CHROME:-}"
if [[ -z "$CHROME" ]]; then
  for candidate in \
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    "/Applications/Chromium.app/Contents/MacOS/Chromium" \
    "$(command -v google-chrome 2>/dev/null || true)" \
    "$(command -v chromium 2>/dev/null || true)" \
    "$(command -v chromium-browser 2>/dev/null || true)"; do
    if [[ -n "$candidate" && -x "$candidate" ]]; then CHROME="$candidate"; break; fi
  done
fi
if [[ -z "$CHROME" || ! -x "$CHROME" ]]; then
  echo 'FAIL: no Chrome/Chromium found — set $CHROME to its path' >&2
  exit 2
fi
for dependency in python3 curl lsof; do
  if ! command -v "$dependency" >/dev/null; then
    echo "FAIL: missing dependency: $dependency" >&2
    exit 2
  fi
done
if [[ "$PORT" == "$DBG" ]]; then
  echo "FAIL: SMOKE_PORT and SMOKE_DBG must differ" >&2
  exit 2
fi
for port in "$PORT" "$DBG"; do
  if lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "FAIL: port $port is in use — cannot trust another server or Chrome's verdict" >&2
    exit 2
  fi
done

echo '>>> [1/4] building the RPG save check...'
if ! bash "$WEB_DIR/build.sh"; then
  echo 'FAIL: RPG wasm build/bindings failed' >&2
  exit 2
fi

PROFILE="$(mktemp -d "${TMPDIR:-/tmp}/rpg_save_smoke.XXXXXX")"
HTTPD_PID=""
CHROME_PID=""
cleanup() {
  if [[ -n "$CHROME_PID" ]]; then
    kill "$CHROME_PID" 2>/dev/null || true
    wait "$CHROME_PID" 2>/dev/null || true
  fi
  if [[ -n "$HTTPD_PID" ]]; then
    kill "$HTTPD_PID" 2>/dev/null || true
    wait "$HTTPD_PID" 2>/dev/null || true
  fi
  pkill -f "$PROFILE" 2>/dev/null || true
  rm -rf "$PROFILE"
}
trap cleanup EXIT
trap 'exit 130' INT TERM

echo ">>> [2/4] serving the check on :$PORT..."
python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$WEB_DIR" \
  >"$PROFILE/http.log" 2>&1 &
HTTPD_PID=$!
serving=0
for _ in $(seq 1 100); do
  if curl --fail --silent --max-time 1 "$URL" >/dev/null; then serving=1; break; fi
  kill -0 "$HTTPD_PID" 2>/dev/null || break
  sleep 0.1
done
if [[ "$serving" -ne 1 ]]; then
  echo "FAIL: http.server never served $URL within its startup budget" >&2
  cat "$PROFILE/http.log" >&2
  exit 2
fi

echo '>>> [3/4] checking saves across two page reloads in a fresh Chrome profile...'
# No virtual-time budget. No renderer or audio device is started by this storage-only check.
"$CHROME" --headless=new --no-first-run --no-default-browser-check \
  --remote-debugging-port="$DBG" --user-data-dir="$PROFILE" "$URL" \
  >"$PROFILE/chrome.log" 2>&1 &
CHROME_PID=$!

last_title='(no matching page)'
deadline=$((SECONDS + 45))
while [[ "$SECONDS" -lt "$deadline" ]]; do
  pages="$(curl --silent --max-time 2 "http://127.0.0.1:$DBG/json" || true)"
  # Parse JSON rather than grepping it: failure details may contain quotes or Unicode.
  last_title="$(printf '%s' "$pages" | python3 -c '
import json, sys
try:
    pages = json.load(sys.stdin)
except ValueError:
    pages = []
print(next((page.get("title", "") for page in pages
            if page.get("type") == "page"
            and page.get("url", "").split("?", 1)[0] == sys.argv[1]),
           "(no matching page)"))
' "$URL")"
  case "$last_title" in
    'RPG_SAVE_CHECK: PASS'*|'RPG_SAVE_CHECK: FAIL'*) break ;;
  esac
  kill -0 "$CHROME_PID" 2>/dev/null || break
  sleep 0.5
done

echo '>>> [4/4] checking the page verdict...'
if [[ "$last_title" == 'RPG_SAVE_CHECK: PASS'* ]]; then
  echo ">>> RPG SAVE WEB SMOKE: PASS — $last_title"
  exit 0
fi
echo "FAIL: last page title: $last_title" >&2
if [[ "$last_title" != 'RPG_SAVE_CHECK: FAIL'* ]]; then
  echo 'FAIL: no final verdict before the deadline or Chrome exit' >&2
  tail -20 "$PROFILE/chrome.log" >&2
  tail -10 "$PROFILE/http.log" >&2
fi
exit 1
