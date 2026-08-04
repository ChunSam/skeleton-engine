#!/usr/bin/env bash
# Runs every example's `<NAME>_SELFTEST=1` headless acceptance test.
#
# These tests exist because a headline feature can degrade gracefully into silence: an audio meter
# that reads 0.0, a hot-reload that reloads nothing, a reconcile call that is never reached. Each
# one was proven non-vacuous by sabotage when it was written — and then nothing ran them again,
# because nothing in CI or `verify.sh` ever invoked one. This script is what runs them.
#
# Usage:
#   ./scripts/selftests.sh
#
# ⚠️ A SKIP IS NOT A PASS. Every one of these tests opts out with exit 0 when its environment
# cannot support a check, which is correct — a machine with no sound card should not fail the
# build. But that makes a skip indistinguishable from a pass at the exit code, so this script
# reads the output too:
#
#   - "no audio device" skips are EXPECTED (CI has no sound card) and tolerated.
#   - Every other skip is a FAILURE. The networked tests skip their live checks when the sibling
#     server binary is missing, and those are the checks that cover the most; a run that quietly
#     dropped them is green while testing strictly less than it appears to.
#
# That distinction is the whole reason this is a script and not a one-line `cargo run` in CI.

set -uo pipefail

cd "$(dirname "$0")/.."

# `ENV_VAR:cargo-example-target`. The target is the Cargo name, which is NOT always the file stem —
# the directory-based examples rename (`examples/games/survivor/survivor.rs` is `survivor_game`).
SELFTESTS=(
  "DATA_ANIM_SELFTEST:data_anim_game"
  "DATA_PARTICLES_SELFTEST:data_particles_game"
  "SURVIVOR_SELFTEST:survivor_game"
  "BEAT_CRAWLER_SELFTEST:beat_crawler_game"
  "SALVAGE_RUN_SELFTEST:salvage_run"
  "PREDICT_SHOOTER_SELFTEST:predict_shooter"
  "AUDIO_REACTIVE_SELFTEST:audio_reactive"
)

# Build EVERY example before running any of them. The networked tests spawn a sibling server binary
# (`salvage_run_server`, `predict_shooter_server`) and skip their live checks if it was never built
# — and `cargo run --example salvage_run` builds only `salvage_run`. Getting this wrong does not
# fail; it silently drops the live checks and reports success.
echo "[selftests] cargo build --examples"
cargo build --examples --quiet || exit 1

failed=()

for spec in "${SELFTESTS[@]}"; do
  var="${spec%%:*}"
  target="${spec##*:}"
  bin="target/debug/examples/${target}"

  if [ ! -x "$bin" ]; then
    echo "[selftests] FAIL ${var} — ${bin} was not built"
    failed+=("${var} (missing binary)")
    continue
  fi

  echo "[selftests] ${var}"
  out=$(env "${var}=1" "$bin" 2>&1)
  code=$?

  # A skip the environment should have been able to support: a server that was never built, a port
  # that could not be reserved, a child that never bound. Not having a sound card is the one
  # legitimate reason to opt out here.
  unexpected=$(printf '%s\n' "$out" | grep 'SKIP:' | grep -v 'no audio device' || true)

  if [ "$code" -ne 0 ]; then
    printf '%s\n' "$out" | sed 's/^/    /'
    echo "[selftests] FAIL ${var} — exit ${code}"
    failed+=("${var} (exit ${code})")
  elif [ -n "$unexpected" ]; then
    printf '%s\n' "$unexpected" | sed 's/^/    /'
    echo "[selftests] FAIL ${var} — a check opted out for a reason this environment should not have"
    failed+=("${var} (unexpected skip)")
  else
    # `ok:` / `OK:` both appear — `audio_reactive` capitalizes where the game selftests do not, and
    # a filter that missed it printed nothing at all under its heading, which reads as "did nothing".
    printf '%s\n' "$out" | grep -E '[Oo][Kk]:|SKIP:|PASS' | sed 's/^/    /'
  fi
done

if [ "${#failed[@]}" -gt 0 ]; then
  echo "[selftests] FAILED: ${failed[*]}"
  exit 1
fi

echo "[selftests] all selftests passed ✓"
