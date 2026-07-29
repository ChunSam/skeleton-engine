#!/usr/bin/env bash
# WASM build script
# Dependency: wasm-pack (cargo install wasm-pack)
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo ">>> Starting WASM build..."
cd "$PROJECT_ROOT"
wasm-pack build --target web --out-dir examples/wasm/pkg

echo ">>> Done. Open in browser:"
echo "    python3 -m http.server 8080 --directory examples/wasm"
echo "    open http://localhost:8080"
