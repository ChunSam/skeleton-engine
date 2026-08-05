#!/usr/bin/env bash
# wasm audio smoke for the `audio_reactive` example (`Audio::levels`).
#
# Why this exists: the verify gate builds wasm but never *runs* it, so "works on the
# web" can only ever be a compile-time claim there. `Audio::levels` is backed by two
# completely different mechanisms — a rodio `Source` tap on native, a Web Audio
# `AnalyserNode` on wasm — so the wasm half shares almost no code with the half the
# native tests cover. Type-checking it proves very little. This builds the example to
# wasm, serves it, runs it in headless Chrome, and asserts the meter ACTUALLY MOVES
# there.
#
# It checks BOTH halves of the feature: that `Audio::levels` moves, and that
# `Audio::bands` returns a spectrum whose energy sits in the LOW bands for the 110 Hz
# kick — the shape assertion, which a mirrored or mis-scaled fold of the browser's FFT
# would fail even though a "spectrum is non-zero" check passed.
#
# How the verdict travels: the example's wasm-only `WebSelfCheck` system watches
# `Audio::levels(BEAT_CHANNEL)` and stamps `AR_CHECK: PASS rms=<n> bands low=<n> high=<n>`
# (or a FAIL with the reason) into the document title. Chrome runs with --remote-debugging-port, whose
# /json endpoint lists each tab's LIVE title, and we poll it. Same mechanism as
# scripts/wasm_audio_smoke.sh.
#
# --autoplay-policy=no-user-gesture-required lets `Audio::resume()` unlock the
# AudioContext without a click; SwiftShader gives WebGL/Audio without a GPU.
#
# What this does NOT check: the acoustic output. A PASS means the analyser read a
# non-zero level from the playing tone, which is exactly the feature's claim. To
# actually hear it:
#   examples/audio_reactive/web/build.sh
#   python3 -m http.server 8087 --directory examples/audio_reactive/web
#
# A **CI gate** as of v0.143.17 (the `wasm-smokes` job). Chrome supplies the GPU via swiftshader and
# renders Web Audio in software, so no hardware device is involved — the "audio is outside CI" rule
# is about the *native* rodio/ALSA path and does not apply here.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the Cargo.lock crate
#   Google Chrome or Chromium (set $CHROME to override auto-detection)
#
# Usage:
#   scripts/audio_reactive_smoke.sh
#   CHROME=/path/to/chrome scripts/audio_reactive_smoke.sh
#
# Exit codes: 0 = pass · 1 = assertion failed · 2 = environment
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WEB_DIR="$PROJECT_ROOT/examples/audio_reactive/web"

PORT="${SMOKE_PORT:-8087}"
DBG="${SMOKE_DBG:-9224}"

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

PROFILE="$(mktemp -d -t chrome_ar_smoke.XXXXXX)"
HTTPD_PID=""
cleanup() {
  [[ -n "$HTTPD_PID" ]] && kill "$HTTPD_PID" 2>/dev/null || true
  pkill -f "$PROFILE" 2>/dev/null || true
  rm -rf "$PROFILE"
}
trap cleanup EXIT

echo ">>> [1/4] building audio_reactive -> wasm (release)..."
"$WEB_DIR/build.sh" >/dev/null

# Refuse to run against a *stale* server on $PORT (e.g. an orphan from a previous run):
# it would serve a different page and the verdict poll could pick up a stale title.
if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  echo "FAIL: port $PORT is already in use — stop it (or set SMOKE_PORT) so we serve OUR page" >&2
  exit 2
fi

echo ">>> [2/4] serving $WEB_DIR on :$PORT..."
# `--directory` (not a `( cd && python3 )` subshell) so $! IS the python process, not a
# subshell whose python child would survive `kill $HTTPD_PID` and orphan itself onto $PORT.
python3 -m http.server "$PORT" --directory "$WEB_DIR" >/dev/null 2>&1 &
HTTPD_PID=$!
sleep 1
if ! kill -0 "$HTTPD_PID" 2>/dev/null; then
  echo "FAIL: http.server failed to start on :$PORT" >&2
  exit 2
fi

# `SKELETON_MUTE=1` silences the speakers without weakening the check: the verdict is computed
# inside the page's Web Audio graph, not from the output device. CI proves that directly — it has
# no audio device at all and still measures rms/bands (v0.143.17). Same switch as the native engine.
MUTE=()
[ "${SKELETON_MUTE:-0}" = "1" ] && MUTE=(--mute-audio)

echo ">>> [3/4] running headless with audio unlocked..."
"$CHROME" --headless=new \
  --enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader \
  --autoplay-policy=no-user-gesture-required "${MUTE[@]}" \
  --remote-debugging-port="$DBG" \
  --user-data-dir="$PROFILE" \
  "http://localhost:$PORT/?autostart=1" >/dev/null 2>&1 &
CHROME_PID=$!

verdict=""
for _ in $(seq 1 60); do          # up to ~30s real time
  titles="$(curl -s "http://localhost:$DBG/json" 2>/dev/null || true)"
  if [[ "$titles" == *"AR_CHECK:"* ]]; then
    verdict="$(printf '%s' "$titles" | grep -oE 'AR_CHECK: [^"]*' | head -1)"
    break
  fi
  kill -0 "$CHROME_PID" 2>/dev/null || break   # Chrome exited on its own
  sleep 0.5
done
# SwiftShader headless Chrome often hangs on exit, so reap it ourselves.
kill "$CHROME_PID" 2>/dev/null || true
pkill -f "$PROFILE" 2>/dev/null || true

echo ">>> [4/4] checking the verdict..."
if [[ "$verdict" == "AR_CHECK: PASS"* ]]; then
  echo ">>> AUDIO REACTIVE SMOKE: PASS — $verdict"
  echo "    (a Web Audio AnalyserNode reported a live non-zero level AND a low-biased spectrum"
  echo "     for the 110 Hz tone, through the same Audio::levels / Audio::bands calls the native"
  echo "     rodio tap + hand-written FFT serve)"
  exit 0
fi

if [[ "$verdict" == "AR_CHECK: FAIL"* ]]; then
  echo "  FAIL: the example reported: ${verdict#AR_CHECK: }" >&2
else
  echo "  FAIL: no AR_CHECK verdict appeared — the check did not finish" >&2
  echo "        (Chrome may lack Web Audio support headless; try the manual listen path)" >&2
fi
echo ">>> AUDIO REACTIVE SMOKE: FAIL" >&2
exit 1
