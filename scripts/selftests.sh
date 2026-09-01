#!/usr/bin/env bash
# Runs every example game's `<NAME>_SELFTEST=1` headless acceptance test.
#
# Rebuilt 2026-08-19 as **phase 0** of plans/2026-08-19-examples-rebuild-plan.md. The previous
# runner went with the examples tree in v0.153.0; this one is deliberately back BEFORE the first
# rebuilt game. The old tree shipped 22 games and only 11 selftests because the runner arrived
# after the games, and nothing ever forced the other half to catch up.
#
# Usage:
#   ./scripts/selftests.sh
#
# ── The contract a selftest must honour ─────────────────────────────────────────────────────────
#
# Each `<NAME>_SELFTEST=1` run must, on stdout/stderr:
#   * print `ok: <what it proved>` for every check that passed,
#   * print `SKIP: <reason>` for every check it opted out of,
#   * exit non-zero if a check failed.
# A run that exits 0 having printed neither is VACUOUS and fails here — a check nobody has seen
# fire is not a check. This contract is written down before the first game exists precisely so it
# does not have to be retrofitted onto five of them.
#
# ── What this enforces, and what each rule already cost ─────────────────────────────────────────
#
# 1. THE LIST IS DERIVED, NEVER HARDCODED. An example is a selftest iff it reads a
#    `<NAME>_SELFTEST` environment variable. v0.143.9 shipped a green gate that had never run the
#    selftest which was the entire point of that change, because the list was hand-maintained.
#    A list you must remember to edit is a list that silently shrinks.
#
# 2. EVERY GAME MUST CARRY ONE. `examples/<name>/<name>.rs` is a game; a game with no selftest
#    variable fails this script. There is no "the failure is visible in a screenshot" exemption —
#    that exemption is what produced 11 unverified games out of 22.
#
# 3. A SKIP IS NEVER A PASS. Each test opts out with exit 0 when its environment cannot support a
#    check, which is correct — a box with no sound card should not fail the build. But that makes
#    a skip indistinguishable from a pass at the exit code, so this script reads the output too:
#    "no audio device" is tolerated, EVERY other skip is a failure. The networked selftest skips
#    its live checks when the sibling server binary is missing, and those are the checks that cover
#    the most: measured on the old tree, with the server hidden the raw exit code was 0.
#
#    `SKELETON_REQUIRE_AUDIO=1` makes the audio skips fatal too. Set it wherever a device is
#    supposed to exist, because without it a device that failed to come up simply skips its checks
#    and reports success — the same trap one level up. Mirrors `SKELETON_REQUIRE_GPU=1`.
#
#    ⚠️ CI does NOT set it: the runner has no usable sound card. A PulseAudio null sink and ALSA
#    `snd-dummy` were both measured over five runs in v0.143.10 — the table is in
#    docs/VERIFICATION.md — and neither works. The audio halves skip in CI by design, so every
#    NATIVE audio claim still rests on a local device. The browser half was the only automated
#    audio measurement the repo ever had, and it is BACK as of 2026-08-21: the `wasm-smokes` CI job
#    runs scripts/survivor_audio_web_smoke.sh, which asserts a live level AND a low-biased spectrum
#    in headless Chrome. So audio is checked somewhere again — just never here, and never natively.
#
# 4. AN EMPTY TREE IS A NO-OP, NOT A PASS. Until phase 1 lands there are no games, and a runner
#    that hard-failed on that would sit red for weeks while telling nobody anything new. It says so
#    loudly and exits 0 — but the moment a game directory appears, rule 2 arms itself, and zero
#    selftests WITH games present is a hard failure rather than a quiet green.
#
# Deliberately no counts anywhere in this file's prose. The previous runner's comments carried
# "~21 example binaries" (real answer: 142) and "9 selftests" (real answer: 11), both because the
# numbers were derived in code and nothing made the prose follow. Measure, don't remember:
#   grep -rhoE '[A-Z_]+_SELFTEST' examples --include='*.rs' | sort -u | wc -l

set -uo pipefail

cd "$(dirname "$0")/.."

# ── Discovery ───────────────────────────────────────────────────────────────────────────────────

# A game is a directory that builds a same-named source file, per the layout settled on 2026-08-19:
# `examples/<name>/<name>.rs`. Sibling files in that directory (a server, a shared protocol module)
# are parts of the game, not games.
GAMES=()
if [ -d examples ]; then
  for dir in examples/*/; do
    g=$(basename "$dir")
    [ -f "${dir}${g}.rs" ] && GAMES+=("$g")
  done
fi

# `ENV_VAR:cargo-example-target`, derived — see rule 1.
SELFTESTS=()
while IFS= read -r file; do
  var=$(grep -oE '"[A-Z_]+_SELFTEST"' "$file" | head -1 | tr -d '"')
  [ -z "$var" ] && continue
  # The Cargo target is NOT the file stem under the current layout: `examples/<name>/<name>.rs` is
  # not auto-discovered by Cargo at all (it finds `examples/*.rs` and `examples/*/main.rs`), so
  # every game carries an explicit `[[example]]` block and the block is authoritative. The stem
  # fallback covers a plain `examples/<name>.rs`, which still auto-discovers.
  name=$(awk -v want="$file" '
    function val(s) { if (match(s, /"[^"]*"/)) return substr(s, RSTART + 1, RLENGTH - 2); return "" }
    /^[[:space:]]*\[/                  { if (p == want && n != "") print n; n = ""; p = "" }
    /^[[:space:]]*name[[:space:]]*=/   { n = val($0) }
    /^[[:space:]]*path[[:space:]]*=/   { p = val($0) }
    END                                { if (p == want && n != "") print n }
  ' Cargo.toml)
  [ -z "$name" ] && name=$(basename "$file" .rs)
  SELFTESTS+=("${var}:${name}")
done < <(grep -rlE 'env::var\("[A-Z_]+_SELFTEST"\)' examples/ 2>/dev/null | sort)

# ── Rule 4: an empty tree is a no-op; a tree with games is armed ────────────────────────────────

if [ "${#SELFTESTS[@]}" -eq 0 ]; then
  if [ "${#GAMES[@]}" -eq 0 ]; then
    echo "[selftests] NOTICE: no example games exist yet — the tree was deleted in v0.153.0 and is"
    echo "[selftests]         being rebuilt (plans/2026-08-19-examples-rebuild-plan.md, phase 1+)."
    echo "[selftests]         Nothing to run. This is a NO-OP, not a pass: it proves nothing about"
    echo "[selftests]         the engine, and it arms itself the moment the first game lands."
    exit 0
  fi
  echo "[selftests] FAIL: ${#GAMES[@]} game(s) present and not one reads a <NAME>_SELFTEST var." >&2
  echo "[selftests]       Either the games skipped their acceptance test (rule 2) or the discovery" >&2
  echo "[selftests]       grep above no longer matches how they read it (rule 1)." >&2
  exit 1
fi

# ── Rule 2: every game carries a selftest ──────────────────────────────────────────────────────

if [ "${#GAMES[@]}" -gt 0 ]; then
  unverified=()
  for g in "${GAMES[@]}"; do
    grep -qE 'env::var\("[A-Z_]+_SELFTEST"\)' "examples/${g}/${g}.rs" || unverified+=("$g")
  done
  if [ "${#unverified[@]}" -gt 0 ]; then
    echo "[selftests] FAIL: game(s) with no <NAME>_SELFTEST: ${unverified[*]}" >&2
    echo "[selftests]       Every game carries one. 'the failure is visible in a screenshot' is the" >&2
    echo "[selftests]       exemption that left 11 of 22 games unverified in the deleted tree." >&2
    exit 1
  fi
fi

echo "[selftests] ${#SELFTESTS[@]} selftest(s) discovered across ${#GAMES[@]} game(s)"
if [ "${SKELETON_REQUIRE_AUDIO:-0}" = "1" ]; then
  echo "[selftests] SKELETON_REQUIRE_AUDIO=1 — an audio-device skip counts as a failure"
fi

# ── Build: the selftest targets and the sibling servers they spawn — NOT every example ─────────
#
# `--examples` builds every example target just to run the handful that carry a selftest, and it
# was the native CI job's single largest cost. The rest stay compile-checked twice over:
# `cargo test --all-targets` covers them natively and the `wasm` job builds the wasm-capable ones.
#
# The servers must be in this list because a networked selftest spawns a sibling `<game>_server`
# binary resolved from its own `current_exe()` directory, and `cargo build --example netplay_game`
# alone would not produce it. Derived from the `[[example]]` block names, for the same reason as
# everything else here. If a future selftest spawns a sibling this pattern misses, it fails LOUDLY
# rather than skipping: "server has not been built" is a skip, and every skip but a missing sound
# card is a failure below.
SERVERS=()
while IFS= read -r n; do [ -n "$n" ] && SERVERS+=("$n"); done < <(
  { grep -oE '^[[:space:]]*name[[:space:]]*=[[:space:]]*"[a-z0-9_]+_server"' Cargo.toml \
      | grep -oE '"[a-z0-9_]+_server"' | tr -d '"'
    for f in examples/*_server.rs; do [ -f "$f" ] && basename "$f" .rs; done
  } | sort -u
)

# ── `--list`: the discovered inventory, for other tools ─────────────────────────────────────────
#
# `scripts/soak.sh` reruns one of these many times and needs the same answers this file computes:
# which env var, which Cargo target, which sibling servers. Deriving that a SECOND time over there
# is precisely the failure rule 1 is written against — two derivations drift, and the one that
# drifts unnoticed is the one that does NOT run on every commit. So there is one derivation and
# this flag exposes it.
#
# Emitted after rules 2-4, so a tree that would fail the gate fails this query the same way rather
# than quietly listing a broken inventory. ⚠️ The `[selftests] ...` progress lines above are on
# stdout too, so a consumer must select the `selftest ` / `server ` prefixes rather than read every
# line — soak.sh does.
if [ "${1:-}" = "--list" ]; then
  for spec in "${SELFTESTS[@]}"; do printf 'selftest %s %s\n' "${spec%%:*}" "${spec##*:}"; done
  if [ "${#SERVERS[@]}" -gt 0 ]; then
    for s in "${SERVERS[@]}"; do printf 'server %s\n' "$s"; done
  fi
  exit 0
fi

BUILD_ARGS=()
for spec in "${SELFTESTS[@]}"; do BUILD_ARGS+=(--example "${spec##*:}"); done
if [ "${#SERVERS[@]}" -gt 0 ]; then
  for s in "${SERVERS[@]}"; do BUILD_ARGS+=(--example "$s"); done
fi

echo "[selftests] cargo build — ${#SELFTESTS[@]} selftest target(s) + ${#SERVERS[@]} sibling server(s)"
cargo build --quiet "${BUILD_ARGS[@]}" || exit 1

# ── Run ────────────────────────────────────────────────────────────────────────────────────────

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

  # `ok:` / `OK:` both appear — the deleted browser harnesses capitalized where the game selftests
  # did not, and a filter that missed one printed nothing at all under its heading, which reads as
  # "did nothing".
  checks=$(printf '%s\n' "$out" | grep -E '[Oo][Kk]:|SKIP:|PASS' || true)

  if [ "$code" -ne 0 ]; then
    printf '%s\n' "$out" | sed 's/^/    /'
    echo "[selftests] FAIL ${var} — exit ${code}"
    failed+=("${var} (exit ${code})")
  elif [ -n "$unexpected" ]; then
    printf '%s\n' "$unexpected" | sed 's/^/    /'
    echo "[selftests] FAIL ${var} — a check opted out for a reason this environment should not have"
    failed+=("${var} (unexpected skip)")
  elif [ -z "$checks" ]; then
    # Exit 0 having asserted nothing. The whole point of an acceptance test is that it can fail;
    # one that prints no verdict is indistinguishable from one that was never wired up.
    printf '%s\n' "$out" | sed 's/^/    /'
    echo "[selftests] FAIL ${var} — exited 0 but printed no 'ok:' or 'SKIP:' verdict (vacuous)"
    failed+=("${var} (no verdict)")
  else
    printf '%s\n' "$checks" | sed 's/^/    /'
  fi
done

if [ "${#failed[@]}" -gt 0 ]; then
  echo "[selftests] FAILED: ${failed[*]}"
  exit 1
fi

echo "[selftests] all selftests passed ✓"
