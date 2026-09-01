#!/usr/bin/env bash
# Reruns `<NAME>_SELFTEST` acceptance tests many times and reports how often they fail.
#
# Usage:
#   ./scripts/soak.sh                        # every selftest, 20 runs each
#   ./scripts/soak.sh -n 60 NETPLAY          # 60 runs of NETPLAY_SELFTEST
#   ./scripts/soak.sh -n 40 netplay_game rpg_quest_game
#
# A name matches either the env var (`NETPLAY_SELFTEST`, or just `NETPLAY`) or the Cargo target
# (`netplay_game`), case-insensitively.
#
# ── ⚠️ What a soak proves, and the much smaller thing it actually proves ────────────────────────
#
# It is a DETECTOR, not a proof. This repo already paid to learn the difference, and the lesson is
# in docs/VERIFICATION.md § "A probe that changes timing can hide the bug you added it to find":
# while hunting the v0.155.1 flake, an `eprintln!` in the server's claim path produced **65
# consecutive clean runs** with the bug fully present. Reverting the probe reproduced the failure
# on run 17 of 25. Clean runs are evidence about the runs that happened, and nothing more.
#
# So: a non-zero rate here is a real finding, and a zero rate is NOT a clean bill of health. What
# closes a timing bug is forcing the interleaving — the `sabotage-check` discipline — and this
# script exists to tell you a flake is THERE and roughly how often, which is the question a one-off
# red in CI cannot answer.
#
# ── The detection floor, since a run count is a claim about sensitivity ─────────────────────────
#
# A flake of rate p survives N independent runs with probability (1-p)^N. At the default N=20:
#
#   p = 15%  (the measured v0.155.1 rate)  ->  4% chance of being missed
#   p = 5%                                 ->  36% chance of being missed
#   p = 1%                                 ->  82% chance of being missed
#
# So 20 runs is sized for the flake this repo has actually seen, not for a rare one. Raise `-n`
# when you are chasing something quieter — and read a green 20-run soak as "not the 15% kind",
# never as "not flaky".
#
# ── Cost ────────────────────────────────────────────────────────────────────────────────────────
#
# Measured 2026-09-01, per single run: NETPLAY_SELFTEST ~9 s (it spawns a real server and paces
# several checks off a wall clock); the other four are well under a second each. So a 20-run soak
# of everything is a few minutes and a 60-run netplay soak is about nine. Deliberately no total
# here — the number of selftests is derived, and a total written down would go stale the moment a
# sixth game lands. Time one and multiply.
#
# ── Discovery is NOT reimplemented here ─────────────────────────────────────────────────────────
#
# `scripts/selftests.sh --list` is the single derivation of which selftests exist, which Cargo
# target each is, and which sibling servers they spawn. Copying that logic into this file is what
# rule 1 over there is written against.

set -uo pipefail

cd "$(dirname "$0")/.."

RUNS=20
FILTERS=()

while [ $# -gt 0 ]; do
  case "$1" in
    -n|--runs)
      RUNS="${2:-}"
      if ! printf '%s' "$RUNS" | grep -qE '^[1-9][0-9]*$'; then
        echo "[soak] FAIL: -n needs a positive integer, got '${RUNS}'" >&2
        exit 2
      fi
      shift 2
      ;;
    -h|--help)
      sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    -*)
      echo "[soak] FAIL: unknown option '$1'" >&2
      exit 2
      ;;
    *)
      FILTERS+=("$1")
      shift
      ;;
  esac
done

# ── Inventory, from the one place that derives it ───────────────────────────────────────────────

INVENTORY=$(./scripts/selftests.sh --list)
if [ $? -ne 0 ]; then
  echo "[soak] FAIL: ./scripts/selftests.sh --list failed — fix the gate before soaking it" >&2
  printf '%s\n' "$INVENTORY" >&2
  exit 2
fi

VARS=()
TARGETS=()
while IFS=' ' read -r kind a b; do
  [ "$kind" = "selftest" ] || continue
  VARS+=("$a")
  TARGETS+=("$b")
done < <(printf '%s\n' "$INVENTORY" | grep '^selftest ')

SERVERS=()
while IFS=' ' read -r kind a; do
  [ "$kind" = "server" ] || continue
  SERVERS+=("$a")
done < <(printf '%s\n' "$INVENTORY" | grep '^server ' || true)

if [ "${#VARS[@]}" -eq 0 ]; then
  echo "[soak] NOTICE: no selftests exist to soak. Nothing to do — this proves nothing."
  exit 0
fi

# ── Selection ───────────────────────────────────────────────────────────────────────────────────

SEL_VARS=()
SEL_TARGETS=()
i=0
while [ "$i" -lt "${#VARS[@]}" ]; do
  var="${VARS[$i]}"
  target="${TARGETS[$i]}"
  keep=0
  if [ "${#FILTERS[@]}" -eq 0 ]; then
    keep=1
  else
    for f in "${FILTERS[@]}"; do
      lf=$(printf '%s' "$f" | tr '[:upper:]' '[:lower:]')
      lv=$(printf '%s' "$var" | tr '[:upper:]' '[:lower:]')
      lt=$(printf '%s' "$target" | tr '[:upper:]' '[:lower:]')
      # `NETPLAY`, `NETPLAY_SELFTEST` and `netplay_game` all select the same thing.
      if [ "$lf" = "$lv" ] || [ "$lf" = "$lt" ] || [ "${lv%_selftest}" = "$lf" ]; then
        keep=1
      fi
    done
  fi
  if [ "$keep" -eq 1 ]; then
    SEL_VARS+=("$var")
    SEL_TARGETS+=("$target")
  fi
  i=$((i + 1))
done

if [ "${#SEL_VARS[@]}" -eq 0 ]; then
  echo "[soak] FAIL: no selftest matched: ${FILTERS[*]}" >&2
  echo "[soak]       available: ${VARS[*]}" >&2
  exit 2
fi

# ── Build ───────────────────────────────────────────────────────────────────────────────────────
#
# The servers come along unconditionally: a networked selftest resolves its sibling from its own
# `current_exe()` directory, and a missing one is a SKIP — which this script treats as a failure,
# exactly as the gate does. Building them is cheaper than diagnosing that.

BUILD_ARGS=()
for t in "${SEL_TARGETS[@]}"; do BUILD_ARGS+=(--example "$t"); done
if [ "${#SERVERS[@]}" -gt 0 ]; then
  for s in "${SERVERS[@]}"; do BUILD_ARGS+=(--example "$s"); done
fi

echo "[soak] cargo build — ${#SEL_TARGETS[@]} target(s) + ${#SERVERS[@]} sibling server(s)"
cargo build --quiet "${BUILD_ARGS[@]}" || exit 1

LOGDIR="target/soak"
rm -rf "$LOGDIR"
mkdir -p "$LOGDIR"

echo "[soak] ${RUNS} run(s) each of: ${SEL_VARS[*]}"
echo "[soak] logs: ${LOGDIR}/<VAR>/run-NNN.log"

# ── Run ─────────────────────────────────────────────────────────────────────────────────────────

overall_failures=0
summary=()

s=0
while [ "$s" -lt "${#SEL_VARS[@]}" ]; do
  var="${SEL_VARS[$s]}"
  target="${SEL_TARGETS[$s]}"
  bin="target/debug/examples/${target}"
  s=$((s + 1))

  if [ ! -x "$bin" ]; then
    echo "[soak] FAIL ${var} — ${bin} was not built" >&2
    summary+=("${var}: BUILD MISSING")
    overall_failures=$((overall_failures + 1))
    continue
  fi

  mkdir -p "${LOGDIR}/${var}"
  echo ""
  echo "[soak] ── ${var} ── ${RUNS} run(s)"

  failures=0
  reasons=""
  run=1
  started=$(date +%s)
  while [ "$run" -le "$RUNS" ]; do
    log=$(printf '%s/%s/run-%03d.log' "$LOGDIR" "$var" "$run")
    out=$(env "${var}=1" "$bin" 2>&1)
    code=$?
    printf '%s\n' "$out" > "$log"

    # ⚠️ The three verdict rules below MIRROR scripts/selftests.sh, deliberately rather than by
    # sharing code: that script stops the world at the first failure, which is the one behaviour a
    # soak must not have. Keep them in step — a soak that is more lenient than the gate reports a
    # clean rate for runs the gate would have rejected, which is worse than not running it.
    reason=""
    if [ "$code" -ne 0 ]; then
      reason="exit ${code}"
    elif [ "${SKELETON_REQUIRE_AUDIO:-0}" = "1" ] && printf '%s\n' "$out" | grep -q 'SKIP:'; then
      reason="unexpected skip"
    elif printf '%s\n' "$out" | grep 'SKIP:' | grep -qv 'no audio device'; then
      reason="unexpected skip"
    elif ! printf '%s\n' "$out" | grep -qE '[Oo][Kk]:|SKIP:|PASS'; then
      reason="vacuous (exit 0, no verdict)"
    fi

    if [ -n "$reason" ]; then
      failures=$((failures + 1))
      printf 'X'
      first=$(printf '%s\n' "$out" | grep -m1 'FAIL' || true)
      reasons="${reasons}
    run ${run}: ${reason}${first:+ — ${first}}"
    else
      printf '.'
    fi
    if [ $((run % 50)) -eq 0 ]; then printf '\n'; fi
    run=$((run + 1))
  done
  elapsed=$(($(date +%s) - started))
  printf '\n'

  rate=$(awk -v f="$failures" -v n="$RUNS" 'BEGIN { printf "%.1f", (n ? 100 * f / n : 0) }')
  echo "[soak] ${var}: ${failures}/${RUNS} failed (${rate}%) in ${elapsed}s"
  if [ "$failures" -gt 0 ]; then
    printf '%s\n' "$reasons"
    echo "[soak]   full output of every run is in ${LOGDIR}/${var}/"
    overall_failures=$((overall_failures + failures))
    summary+=("${var}: ${failures}/${RUNS}")
  else
    summary+=("${var}: 0/${RUNS}")
  fi

  # ── Margins, when the selftest reports them ───────────────────────────────────────────────────
  #
  # ⚠️ Format-coupled on purpose, and it degrades to silence rather than to a wrong number: only
  # `NETPLAY_SELFTEST` prints a `Margins:` line today (v0.156.4). Those margins are the whole
  # reason a soak is more informative than a pass/fail count — a margin drifting toward zero
  # across runs is the warning that a red would otherwise deliver with no notice. See `APPROACH`
  # in examples/netplay_game/netplay_game.rs for what squeezes each one.
  margins=$(grep -h 'Margins:' "${LOGDIR}/${var}"/*.log 2>/dev/null || true)
  if [ -n "$margins" ]; then
    echo "[soak]   margins across ${RUNS} run(s) — worst of each, which is the one that decides:"
    printf '%s\n' "$margins" \
      | awk '
        match($0, /copy [0-9.]+ px/)          { g = substr($0, RSTART+5, RLENGTH-8) + 0; if (g > mg) mg = g }
        match($0, /slowest frame [0-9.]+ ms/) { f = substr($0, RSTART+14, RLENGTH-17) + 0; if (f > mf) mf = f }
        match($0, /flight [0-9.]+ s/)         { t = substr($0, RSTART+7, RLENGTH-9) + 0; if (t > mt) mt = t }
        END {
          printf "[soak]     server gap %.1f px · slowest frame %.0f ms · flight %.1f s\n", mg, mf, mt
        }'
    echo "[soak]     (every line: grep -h Margins: ${LOGDIR}/${var}/*.log)"
  fi
done

# ── Verdict ─────────────────────────────────────────────────────────────────────────────────────

echo ""
echo "[soak] ${summary[*]}"
if [ "$overall_failures" -gt 0 ]; then
  echo "[soak] FLAKY: ${overall_failures} failing run(s). A rate here is a real finding — but the"
  echo "[soak]        fix is not 'run it again until green'. Force the interleaving and prove it;"
  echo "[soak]        docs/VERIFICATION.md records why counting clean runs settles nothing."
  exit 1
fi
echo "[soak] no failures in ${RUNS} run(s) each ✓"
echo "[soak] ⚠️ That is NOT 'not flaky' — see the detection floor at the top of this file."
