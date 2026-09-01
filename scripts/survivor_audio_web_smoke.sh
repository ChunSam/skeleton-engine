#!/usr/bin/env bash
# Web Audio browser smoke — the one measurement the 2026-08-19 deletion actually lost.
#
# Runs `survivor_game`'s wasm build in headless Chrome, plays a 110 Hz metered tone, and asserts the
# engine's analysis reports BOTH:
#   * a live level  — `Audio::levels(meter).rms > 0`
#   * a low-biased spectrum — `Audio::bands` leaning toward the low end
# The page writes its verdict into `document.title` (`AUDIO_CHECK: PASS — …`); this script reads it
# live over Chrome's DevTools endpoint.
#
# ── Why this one is first ───────────────────────────────────────────────────────────────────────
#
# Web Audio genuinely gated from v0.143.17 to v0.153.0: the deleted `wasm_audio` smoke reported
# 38/38 and `audio_reactive` measured rms=0.643 with low=9.41 / high=0.00 on a 110 Hz tone — real
# spectral discrimination. Native rodio/ALSA cannot be tested in CI (five runs, v0.143.10; the table
# is in docs/VERIFICATION.md), so the browser half was the ONLY automated audio evidence this repo
# ever had. Since the deletion there has been none of any kind.
#
# ⚠️ Both halves are asserted because either alone is forgeable. A level with no spectrum passes on
# a backend that reports a plausible number without analysing anything; a spectrum with no level
# passes on one that fills a fixed curve. Together they say the analyser is looking at *this* signal.
#
# ── Prerequisites ───────────────────────────────────────────────────────────────────────────────
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
#   Google Chrome or Chromium (set $CHROME to override auto-detection)
#
# Usage:
#   scripts/survivor_audio_web_smoke.sh
#   SMOKE_PORT=8090 SMOKE_DBG=9301 scripts/survivor_audio_web_smoke.sh
#   SKELETON_MUTE=1 scripts/survivor_audio_web_smoke.sh    # silent speakers, same measurement
#
# Exit codes: 0 = pass · 1 = the audio assertion failed · 2 = environment
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WEB_DIR="$PROJECT_ROOT/examples/survivor_game/web"

PORT="${SMOKE_PORT:-8090}"
DBG="${SMOKE_DBG:-9310}"

# ── locate a Chrome/Chromium binary ─────────────────────────────────────────────────────────────
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

PROFILE="$(mktemp -d -t chrome_audio_smoke.XXXXXX)"
HTTPD_PID=""
CHROME_PID=""
# `wait` after each kill, and job control off, so a PASSING run does not end with two
# "Terminated: 15" lines from the shell reporting its own background jobs — that reads as a crash
# in a green log and is exactly the kind of noise that trains people to skim CI output.
set +m
cleanup() {
  if [[ -n "$CHROME_PID" ]]; then kill "$CHROME_PID" 2>/dev/null; wait "$CHROME_PID" 2>/dev/null; fi
  if [[ -n "$HTTPD_PID" ]]; then kill "$HTTPD_PID" 2>/dev/null; wait "$HTTPD_PID" 2>/dev/null; fi
  pkill -f "$PROFILE" 2>/dev/null
  rm -rf "$PROFILE"
  return 0
}
trap cleanup EXIT

echo ">>> [1/4] building survivor_game -> wasm (release)..."
if ! bash "$WEB_DIR/build.sh" >/dev/null; then
  echo "FAIL: the wasm build failed — run $WEB_DIR/build.sh to see why" >&2
  exit 2
fi

echo ">>> [2/4] serving $WEB_DIR on :$PORT..."
# Refuse a stale server on $PORT. An orphaned http.server from a previous run would serve a
# DIFFERENT page, which produces no AUDIO_CHECK verdict and reads as a broken engine.
if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  echo "FAIL: port $PORT is already in use — stop it (or set SMOKE_PORT) so we serve OUR page" >&2
  exit 2
fi
# `--directory` (not a `( cd && python3 )` subshell) so $! IS python, not a subshell whose python
# *child* survives `kill $HTTPD_PID` and orphans itself onto $PORT.
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

echo ">>> [3/4] running the Web Audio check headless..."
# Flags, and why each is load-bearing:
#   --headless=new                      real Chrome, not the old headless shell
#   --enable-unsafe-swiftshader
#   --use-gl=angle --use-angle=swiftshader   software GPU — CI runners have none
#   --autoplay-policy=no-user-gesture-required
#                                       lets the AudioContext unlock without a click. WITHOUT this
#                                       the context stays suspended, the meter reads 0.0, and the
#                                       check fails on a perfectly good engine.
#   --mute-audio (only under SKELETON_MUTE=1)
#                                       silences the speakers. The level taps are pre-volume, so
#                                       this does NOT weaken the measurement — same numbers.
#
# ⚠️ NOT --virtual-time-budget. The AudioContext unlock and the analysis windows happen on a wall
# clock; fast-forwarding virtual time would race past them and read silence off a working device.
# This is the same trap CLAUDE.md records for ENGINE_CAPTURE, one layer out.
MUTE=()
[ "${SKELETON_MUTE:-0}" = "1" ] && MUTE=(--mute-audio)
"$CHROME" --headless=new \
  --enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader \
  --autoplay-policy=no-user-gesture-required "${MUTE[@]}" \
  --remote-debugging-port="$DBG" \
  --user-data-dir="$PROFILE" \
  "http://localhost:$PORT/check.html" >/dev/null 2>&1 &
CHROME_PID=$!

verdict=""
for _ in $(seq 1 90); do          # up to ~45 s; the page's own deadline is 20 s
  titles="$(curl -s "http://localhost:$DBG/json" 2>/dev/null || true)"
  if [[ "$titles" == *"AUDIO_CHECK:"* ]]; then
    verdict="$(printf '%s' "$titles" | grep -oE 'AUDIO_CHECK: [^"]*' | head -1)"
    break
  fi
  kill -0 "$CHROME_PID" 2>/dev/null || break   # Chrome exited on its own
  sleep 0.5
done

echo ">>> [4/4] checking results..."
if [[ "$verdict" == "AUDIO_CHECK: PASS"* ]]; then
  echo ">>> SURVIVOR WEB AUDIO SMOKE: PASS — $verdict"
  exit 0
fi

if [[ "$verdict" == "AUDIO_CHECK: FAIL"* ]]; then
  echo "  FAIL: the page reported ${verdict#AUDIO_CHECK: }" >&2
else
  echo "  FAIL: no AUDIO_CHECK verdict appeared — the page did not finish." >&2
  echo "        Serve it by hand and watch the console:" >&2
  echo "        python3 -m http.server $PORT --directory $WEB_DIR" >&2
  echo "        then open http://localhost:$PORT/check.html" >&2
fi
echo ">>> SURVIVOR WEB AUDIO SMOKE: FAIL" >&2
exit 1
