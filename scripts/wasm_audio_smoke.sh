#!/usr/bin/env bash
# WebAudio lifecycle smoke test (the wasm `engine::WebAudio` path).
#
# Why this exists: 3B (v0.23.0) grew `WebAudio` from one-shot SFX into a small mixer (master
# volume, looping music channel, suspend/resume) but nothing ever *ran* it in a browser — the
# wasm build only proves it compiles. This script runs the `web_audio` example headless and
# asserts the audio-graph LIFECYCLE actually works at runtime:
#   - AudioContext is created and resume()s to "running"
#   - master volume set/clamp is applied (read back through the gain node)
#   - play_music() decodes real bytes and starts a looping source
#   - crossfade_music() fades the music channel from one track to another
#   - play_sfx() controllable SFX: pan/volume/stop
#   - named mixer buses: set_bus_volume/clamp, bus_volume, bus_names, play_sfx_on_bus/play_on_bus
#   - bus ducking: duck_bus/release_bus ramp the duck multiplier, bus_duck reads it, gain clamp
#   - 2D positional: play_at/play_at_on_bus/update_position set distance-falloff volume + x-offset pan
#   - suspend()/resume() toggle the context state
# The example drives every WebAudio method and stamps the verdict into the document title as
# "AUDIO_CHECK: PASS (n/n)" / "...FAIL: <step>"; this script reads that title live over Chrome's
# DevTools endpoint and asserts PASS (so the failing step name travels with a failure).
#
# What it does NOT check: whether you actually HEAR the tone. There is no audio capture in this
# flow, so acoustic output stays a human step — open the page (see below) and listen. Everything
# up to "the right nodes are wired and playing" is automated here.
#
# It is a **CI gate** as of v0.143.17 (the `wasm-smokes` job) — Web Audio renders in software, so
# unlike the native rodio path this needs no hardware device. Run it after touching
# src/audio_wasm.rs or the example.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <ver>   # MUST match the wasm-bindgen crate in Cargo.lock
#   Google Chrome or Chromium (set $CHROME to override auto-detection)
#
# Usage:
#   scripts/wasm_audio_smoke.sh
#   CHROME=/path/to/chrome scripts/wasm_audio_smoke.sh
#
# Manual listen (the human half):
#   examples/web_audio/web/build.sh
#   python3 -m http.server 8080 --directory examples/web_audio/web
#   open http://localhost:8080   # click "Start audio", listen for a 440 Hz tone
#
# Exit codes: 0 = pass · 1 = lifecycle assertion failed · 2 = environment
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WEB_DIR="$PROJECT_ROOT/examples/web_audio/web"

PORT="${SMOKE_PORT:-8084}"

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

PROFILE="$(mktemp -d -t chrome_audio_smoke.XXXXXX)"
HTTPD_PID=""
cleanup() {
  [[ -n "$HTTPD_PID" ]] && kill "$HTTPD_PID" 2>/dev/null || true
  pkill -f "$PROFILE" 2>/dev/null || true
  rm -rf "$PROFILE"
}
trap cleanup EXIT

echo ">>> [1/4] building web_audio -> wasm (release)..."
# Invoke via `bash` so it works regardless of the script's executable bit (git stores it 0644).
bash "$WEB_DIR/build.sh" >/dev/null

echo ">>> [2/4] serving $WEB_DIR on :$PORT..."
# Refuse a stale server on $PORT (an orphaned http.server from a prior run would
# serve a different page → no AUDIO_CHECK verdict, a confusing FAIL). Guard it like
# centered_text_smoke.sh so a leaked python can't silently take over the port.
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

echo ">>> [3/4] running the lifecycle check headless..."
# Run Chrome headless in REAL time (NOT --virtual-time-budget): the AudioContext suspend/resume
# state transitions happen on the browser's real-time audio thread, so a virtual clock that
# fast-forwards the JS timers races ahead of them and the last resume() flakes. The example signals
# its result by setting document.title to "AUDIO_CHECK: PASS (n/n)" / "...FAIL: <step>"; we read
# that LIVE over the DevTools endpoint (--remote-debugging-port exposes /json with each tab's
# title, including the failing step on FAIL) and poll until the verdict appears.
# --autoplay-policy=no-user-gesture-required lets resume() unlock the context without a click;
# SwiftShader runs WebGL/Audio without a GPU.
DBG="${SMOKE_DBG:-9223}"
# `SKELETON_MUTE=1` silences the speakers without weakening the check: the verdict is computed
# inside the page's Web Audio graph, not from the output device. CI proves that directly — it has
# no audio device at all and still reports 38/38 (v0.143.17). Same switch as the native engine.
MUTE=()
[ "${SKELETON_MUTE:-0}" = "1" ] && MUTE=(--mute-audio)
"$CHROME" --headless=new \
  --enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader \
  --autoplay-policy=no-user-gesture-required "${MUTE[@]}" \
  --remote-debugging-port="$DBG" \
  --user-data-dir="$PROFILE" \
  "http://localhost:$PORT/" >/dev/null 2>&1 &
CHROME_PID=$!

verdict=""
for _ in $(seq 1 60); do          # up to ~30s real time
  # /json lists open tabs with their current (live) title.
  titles="$(curl -s "http://localhost:$DBG/json" 2>/dev/null || true)"
  if [[ "$titles" == *"AUDIO_CHECK:"* ]]; then
    verdict="$(printf '%s' "$titles" | grep -oE 'AUDIO_CHECK: [^"]*' | head -1)"
    break
  fi
  kill -0 "$CHROME_PID" 2>/dev/null || break   # Chrome exited on its own
  sleep 0.5
done
# SwiftShader headless Chrome often hangs on exit, so reap it ourselves rather than waiting.
kill "$CHROME_PID" 2>/dev/null || true
pkill -f "$PROFILE" 2>/dev/null || true

echo ">>> [4/4] checking results..."
if [[ "$verdict" == "AUDIO_CHECK: PASS"* ]]; then
  echo ">>> WASM AUDIO SMOKE: PASS — $verdict"
  echo "    (AudioContext + volume set/clamp + looping-music decode + suspend/resume all work)"
  echo "    NOTE: acoustic output is NOT auto-checked — to hear the tone:"
  echo "      python3 -m http.server 8080 --directory examples/web_audio/web && open http://localhost:8080"
  exit 0
fi

if [[ "$verdict" == "AUDIO_CHECK: FAIL"* ]]; then
  echo "  FAIL: the example reported a failing lifecycle step: ${verdict#AUDIO_CHECK: }" >&2
else
  echo "  FAIL: no AUDIO_CHECK verdict appeared — the check did not finish" >&2
  echo "        (Chrome may lack Web Audio support headless; try the manual listen path)" >&2
fi
echo ">>> WASM AUDIO SMOKE: FAIL" >&2
exit 1
