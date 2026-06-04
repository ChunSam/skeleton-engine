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
# The native-only examples (platformer_game / mp_server / gpu_particles) depend on
# rapier2d / tungstenite / GpuParticleEmitter and do not compile to wasm, so
# --all-targets would report false failures. --lib would also work.

set -euo pipefail

echo "[verify] cargo fmt --check"
cargo fmt --check

echo "[verify] cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets -- -D warnings

echo "[verify] cargo build --target wasm32-unknown-unknown"
cargo build --target wasm32-unknown-unknown

echo "[verify] cargo test --all-targets"
cargo test --all-targets

echo "[verify] RUSTDOCFLAGS=-D warnings cargo doc --no-deps"
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

echo "[verify] all checks passed ✓"
