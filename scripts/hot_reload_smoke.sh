#!/usr/bin/env bash
# Native smoke for hot-reload under an asset root — the one EW-008 clause the engine used to miss.
#
# Why this exists: the `notify` watcher used to register the caller's LOGICAL asset path. In a
# packaged / foreign-cwd layout that path does not exist relative to the working directory, so the
# watch silently failed and an edit never triggered a reload. The fix watches `resolve(logical)`
# (the file that actually exists under the asset root) and translates the watcher event back to the
# logical dispatch key. This proves that end to end with a REAL filesystem edit — the deterministic
# unit tests in `src/asset/tests.rs` pin the pure mapping; this exercises the live `notify` loop.
#
# The example itself is the assertion: it pins a temp asset root (so resolution is not cwd-based),
# loads a data table by a relative logical path, edits the file on disk, polls `poll_reloads`, and
# exits non-zero unless the edited value is picked up. Needs no GPU, window, or display.
#
# Usage:
#   scripts/hot_reload_smoke.sh
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo ">>> [1/3] building the example..."
cargo build --quiet --example hot_reload_asset_root

BIN="$PROJECT_ROOT/target/debug/examples/hot_reload_asset_root"
if [[ ! -x "$BIN" ]]; then
  echo "FAIL: no binary at $BIN" >&2
  exit 1
fi

echo ">>> [2/3] running it (edits a table under a pinned asset root, from a foreign cwd)..."
# Run from / to underline that the working directory is irrelevant — the asset root, not cwd,
# resolves the table. (The example pins the root itself, so cwd truly does not matter.)
OUT="$(cd / && "$BIN" 2>&1)" || {
  echo "$OUT"
  echo "FAIL: hot-reload did not fire for a resolved-not-cwd asset" >&2
  exit 1
}
echo "$OUT"

echo ">>> [3/3] verifying the output..."
if ! grep -q "^OK: hot-reload fired under a foreign asset root" <<<"$OUT"; then
  echo "FAIL: example did not report OK" >&2
  exit 1
fi

echo "PASS: an edit to a resolved-not-cwd data table hot-reloaded under its logical key"
