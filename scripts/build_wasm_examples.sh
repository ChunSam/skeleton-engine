#!/usr/bin/env bash
# Builds every example that has a wasm entry point for `wasm32-unknown-unknown`.
#
# CI's Build (WASM) job compiles the library and binaries only, so an example's wasm path was
# invisible to it — `CLAUDE.md` had to tell contributors to remember this by hand, which is exactly
# the kind of thing a human forgets and a machine does not.
#
# Usage:
#   ./scripts/build_wasm_examples.sh
#
# A blanket `cargo build --examples --target wasm32-unknown-unknown` cannot replace this: several
# examples are native-only *by construction* and fail to compile for wasm on purpose — hot-reload
# (`hot_reload_asset_root`, `tile_anim_stagger`), headless screenshots (`headless_screenshot`),
# the servers (`tungstenite`), physics (`rapier2d`), GPU particles. Their wasm failure is correct
# behavior, so the build has to name the examples that are actually meant to ship to the web.
#
# The set is DERIVED, not hardcoded: an example is wasm-targeted iff it declares a `#[wasm_bindgen]`
# entry point, which is the thing an `index.html` calls. A hardcoded list here would go stale the
# first time someone adds a web example and forget to say so.

set -euo pipefail

cd "$(dirname "$0")/.."

targets=()
while IFS= read -r file; do
  # Examples declared with an explicit `[[example]]` block carry a name that need not match the
  # file stem (`examples/games/survivor/survivor.rs` builds as `survivor_game`); auto-discovered
  # `examples/*.rs` have no block at all and use the stem.
  name=$(awk -v want="$file" '
    /^\[\[example\]\]/            { name = "" }
    /^name[[:space:]]*=/          { gsub(/[" ]/, "", $3); name = $3 }
    /^path[[:space:]]*=/          { gsub(/[" ]/, "", $3); if ($3 == want) print name }
  ' Cargo.toml)

  if [ -z "$name" ]; then
    name=$(basename "$file" .rs)
  fi

  targets+=("$name")
done < <(grep -rl 'wasm_bindgen::prelude::wasm_bindgen\]' examples/ | sort)

if [ "${#targets[@]}" -eq 0 ]; then
  echo "[wasm-examples] found no examples with a wasm entry point — the detection above is broken" >&2
  exit 1
fi

echo "[wasm-examples] ${#targets[@]} examples: ${targets[*]}"

args=()
for t in "${targets[@]}"; do
  args+=(--example "$t")
done

cargo build "${args[@]}" --target wasm32-unknown-unknown

echo "[wasm-examples] all ${#targets[@]} wasm examples built ✓"
