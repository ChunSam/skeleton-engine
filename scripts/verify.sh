#!/usr/bin/env bash
# CI-equivalent local verification — run before declaring a change "done".
#
# Mirrors the gates in .github/workflows/ci.yml (native fmt + clippy + test,
# the wasm build, and the rustdoc -D warnings doc build) so a refactor can't
# pass locally yet break CI.
#
# Usage:
#   ./scripts/verify.sh
#
# Note on the wasm gate: this builds the lib + bins for wasm32, NOT --all-targets.
# The native-only tests depend on rapier2d / tungstenite / GpuParticleEmitter, or on
# native-only engine APIs (hot-reload, headless screenshots), and do not compile to wasm,
# so --all-targets would report false failures. --lib would also work.

set -euo pipefail

echo "[verify] cargo fmt --check"
cargo fmt --check

echo "[verify] cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets -- -D warnings

echo "[verify] cargo build --target wasm32-unknown-unknown"
cargo build --target wasm32-unknown-unknown

# CI's Build (WASM) job ALSO runs wasm clippy with -D warnings. The wasm build above
# only WARNS on issues like unused imports in wasm-gated code; this step turns those
# into errors locally, matching CI (a wasm-only unused import once passed local build
# but failed CI clippy — see git history).
echo "[verify] cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings"
cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings

echo "[verify] cargo test --all-targets"
cargo test --all-targets

# --all-targets does NOT run doctests, so run them explicitly — doc examples are
# fork-facing copy-paste sources and must compile/pass.
echo "[verify] cargo test --doc"
cargo test --doc

echo "[verify] RUSTDOCFLAGS=-D warnings cargo doc --no-deps"
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# The `<NAME>_SELFTEST` acceptance tests. Locally these run MORE than they do in CI — a machine with
# a sound card exercises the audio checks CI can only skip, and `SKELETON_REQUIRE_AUDIO=1` turns a
# skipped one into a failure. A no-op until the first rebuilt game lands; self-arming after that.
echo "[verify] ./scripts/selftests.sh"
./scripts/selftests.sh

echo "[verify] all checks passed ✓"
