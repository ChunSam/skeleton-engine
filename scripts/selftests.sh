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
#   - "no audio device" skips are tolerated by default, so a developer box or a runner without a
#     sound card does not fail the build.
#   - Every other skip is a FAILURE. The networked tests skip their live checks when the sibling
#     server binary is missing, and those are the checks that cover the most; a run that quietly
#     dropped them is green while testing strictly less than it appears to.
#
# `SKELETON_REQUIRE_AUDIO=1` makes the audio skips fatal too. Set it wherever a device is supposed
# to exist — CI provisions a PulseAudio null sink, and without this flag a null sink that failed to
# come up would simply skip the audio checks and report success. That is the same trap this script
# exists to close, one level up. Mirrors the render job's `SKELETON_REQUIRE_GPU=1`.
#
# That distinction is the whole reason this is a script and not a one-line `cargo run` in CI.

set -uo pipefail

cd "$(dirname "$0")/.."

# `ENV_VAR:cargo-example-target`, DERIVED rather than hardcoded: an example is a selftest iff it
# reads a `<NAME>_SELFTEST` environment variable.
#
# The first version of this script hardcoded the list, and the very next selftest to land
# (`ORBITAL_DODGER_SELFTEST`, v0.143.9) was not in it — the gate went green having never run the
# test that was the whole point of that change. A list you must remember to edit is a list that
# silently shrinks, which is the same failure this script exists to prevent.
SELFTESTS=()
while IFS= read -r file; do
  var=$(grep -oE '"[A-Z_]+_SELFTEST"' "$file" | head -1 | tr -d '"')
  [ -z "$var" ] && continue
  # The Cargo target is NOT always the file stem — the directory-based examples rename
  # (`examples/games/survivor/survivor.rs` builds as `survivor_game`). Explicit `[[example]]` blocks
  # carry the name; auto-discovered `examples/*.rs` have no block and use the stem.
  name=$(awk -v want="$file" '
    /^\[\[example\]\]/   { name = "" }
    /^name[[:space:]]*=/ { gsub(/[" ]/, "", $3); name = $3 }
    /^path[[:space:]]*=/ { gsub(/[" ]/, "", $3); if ($3 == want) print name }
  ' Cargo.toml)
  [ -z "$name" ] && name=$(basename "$file" .rs)
  SELFTESTS+=("${var}:${name}")
done < <(grep -rlE 'env::var\("[A-Z_]+_SELFTEST"\)' examples/ | sort)

if [ "${#SELFTESTS[@]}" -eq 0 ]; then
  echo "[selftests] found no selftests — the detection above is broken" >&2
  exit 1
fi

echo "[selftests] ${#SELFTESTS[@]} selftests discovered"
if [ "${SKELETON_REQUIRE_AUDIO:-0}" = "1" ]; then
  echo "[selftests] SKELETON_REQUIRE_AUDIO=1 — an audio-device skip counts as a failure"
fi

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
  # legitimate reason to opt out — unless the caller says a device is supposed to be there.
  if [ "${SKELETON_REQUIRE_AUDIO:-0}" = "1" ]; then
    unexpected=$(printf '%s\n' "$out" | grep 'SKIP:' || true)
  else
    unexpected=$(printf '%s\n' "$out" | grep 'SKIP:' | grep -v 'no audio device' || true)
  fi

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
