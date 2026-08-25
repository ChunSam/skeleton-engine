#!/usr/bin/env bash
# Builds every example game for wasm32 — except the ones that declare they cannot.
#
# Rebuilt 2026-08-20 as part of phase 4 of plans/2026-08-19-examples-rebuild-plan.md. The original
# went with the examples tree in v0.153.0, and between then and now `cargo build --target
# wasm32-unknown-unknown` covered the **library only**: an example could stop compiling for the web
# and every job stayed green.
#
# ⚠️ Compiling for wasm is still not running on wasm. This script proves a game *builds*; nothing
# here loads it in a browser. That is the `wasm-smokes` job, restored in phase 5b on 2026-08-21
# together with its branch-protection context — the two have to move in one change, or the job
# becomes a check nobody is gated on (the mirror image of the failure v0.153.0 hit). It caught a
# real defect on its first run: a wasm entry point that compiled and exported nothing.
#
# ── Discovery, and why it is shaped this way ────────────────────────────────────────────────────
#
# The list of examples is DERIVED from Cargo.toml's `[[example]]` blocks, never hardcoded — the same
# rule scripts/selftests.sh follows, for the same reason (v0.143.9 shipped a green gate that had
# never run the selftest the change was for, because its list was hand-maintained).
#
# A game that cannot build for wasm declares it in its own source with the token `NATIVE_ONLY`,
# followed by the reason. That declaration is checked BOTH ways:
#
#   * undeclared + fails to build  -> failure. The default is "every game runs on the web", so a
#     game that quietly stops doing so is loud rather than silent.
#   * declared   + builds fine     -> ALSO a failure. A declaration is a claim about the code, and a
#     stale one hides the very regression this script exists to catch. When rapier2d gains a wasm
#     backend, this is what makes somebody notice.

set -uo pipefail

cd "$(dirname "$0")/.."

TARGET=wasm32-unknown-unknown

if ! rustup target list --installed 2>/dev/null | grep -q "^${TARGET}$"; then
    echo "[wasm-examples] FAIL: the ${TARGET} target is not installed." >&2
    echo "[wasm-examples]       rustup target add ${TARGET}" >&2
    exit 1
fi

# `name` + `path` pairs from the `[[example]]` blocks. Same awk shape as scripts/selftests.sh — and
# the same `while read` loop rather than `mapfile`, which is bash 4+ and absent on macOS's 3.2.
ENTRIES=()
while IFS= read -r line; do [ -n "$line" ] && ENTRIES+=("$line"); done < <(awk '
    function val(s) { if (match(s, /"[^"]*"/)) return substr(s, RSTART + 1, RLENGTH - 2); return "" }
    /^[[:space:]]*\[/                  { if (n != "" && p != "") print n "\t" p; n = ""; p = "" }
    /^[[:space:]]*name[[:space:]]*=/   { n = val($0) }
    /^[[:space:]]*path[[:space:]]*=/   { p = val($0) }
    END                                { if (n != "" && p != "") print n "\t" p }
' Cargo.toml)

if [ "${#ENTRIES[@]}" -eq 0 ]; then
    echo "[wasm-examples] NOTICE: no [[example]] blocks in Cargo.toml — nothing to build."
    echo "[wasm-examples]         This is a NO-OP, not a pass. It arms itself with the first game."
    exit 0
fi

echo "[wasm-examples] ${#ENTRIES[@]} example target(s) from Cargo.toml"

failed=()
built=0
declared=0

for entry in "${ENTRIES[@]}"; do
    name="${entry%%$'\t'*}"
    path="${entry##*$'\t'}"

    reason=""
    if [ -f "$path" ]; then
        reason=$(grep -m1 'NATIVE_ONLY' "$path" | sed 's/.*NATIVE_ONLY[: ]*//' | sed 's/[[:space:]]*$//')
    fi

    if [ -n "$reason" ]; then
        declared=$((declared + 1))
        # A declared-native example must still FAIL to build — otherwise the declaration is stale.
        if cargo build --quiet --target "$TARGET" --example "$name" 2>/dev/null; then
            echo "[wasm-examples] FAIL ${name} — declared NATIVE_ONLY (${reason}) but it builds for ${TARGET}." >&2
            echo "[wasm-examples]      Remove the declaration; a stale one hides a real regression." >&2
            failed+=("$name (stale NATIVE_ONLY)")
        else
            echo "[wasm-examples] skip ${name} — NATIVE_ONLY: ${reason}"
        fi
        continue
    fi

    echo "[wasm-examples] ${name}"
    if cargo build --quiet --target "$TARGET" --example "$name"; then
        built=$((built + 1))
    else
        echo "[wasm-examples] FAIL ${name} — does not build for ${TARGET}." >&2
        echo "[wasm-examples]      Fix it, or add a 'NATIVE_ONLY: <reason>' line to ${path}." >&2
        failed+=("$name")
    fi
done

if [ "${#failed[@]}" -gt 0 ]; then
    echo "[wasm-examples] ${#failed[@]} failure(s): ${failed[*]}" >&2
    exit 1
fi

echo "[wasm-examples] ${built} example(s) build for ${TARGET}, ${declared} declared native-only ✓"
