#!/usr/bin/env bash
# Regenerate the bundled Korean editor font subset.
#
# The editor / debug overlay (egui) needs a CJK fallback to render Hangul, but the
# full Noto Sans KR Regular is ~5.9 MB — which pushes the crate package over the
# crates.io 10 MB publish limit. This script subsets the full font down to ~2.3 MB
# by keeping only the glyphs the editor actually needs and dropping the Hanja (CJK
# ideographs) plus the OpenType layout / hinting tables that egui's ab_glyph
# rasterizer never consults.
#
# The output (assets/fonts/NotoSansKR-Regular-subset.ttf) is what src/debug_ui.rs
# embeds via include_bytes!. It is a "Modified Version" under the SIL OFL 1.1 — see
# assets/fonts/NotoSansKR-OFL.txt (the primary font name stays "Noto Sans KR" and
# does NOT use the Reserved Font Name 'Source', per OFL clause 3).
#
# Coverage kept (any modern Korean text renders; archaic Hangul + Hanja do not):
#   U+0000-00FF  Basic Latin + Latin-1 Supplement
#   U+1100-11FF  Hangul Jamo
#   U+2000-206F  General Punctuation (curly quotes, dashes, ellipsis, …)
#   U+20A9       ₩ Won sign
#   U+3000-303F  CJK Symbols and Punctuation
#   U+3130-318F  Hangul Compatibility Jamo
#   U+AC00-D7A3  Hangul Syllables (all 11172 modern syllables)
#   U+FF00-FFEF  Halfwidth and Fullwidth Forms
#
# Usage:
#   1. Install fonttools:  python3 -m venv .venv && .venv/bin/pip install fonttools
#   2. Fetch the full Noto Sans KR Regular (SIL OFL 1.1), e.g. from
#      https://fonts.google.com/noto/specimen/Noto+Sans+KR , and save it as
#      NotoSansKR-Regular.ttf (or pass its path as $1).
#   3. ./scripts/subset_korean_font.sh [path/to/NotoSansKR-Regular.ttf]
set -euo pipefail

SRC="${1:-NotoSansKR-Regular.ttf}"
OUT="assets/fonts/NotoSansKR-Regular-subset.ttf"
RANGES='U+0000-00FF,U+1100-11FF,U+2000-206F,U+20A9,U+3000-303F,U+3130-318F,U+AC00-D7A3,U+FF00-FFEF'

if ! command -v pyftsubset >/dev/null 2>&1; then
  echo "error: pyftsubset not found — install fonttools (pip install fonttools)" >&2
  exit 1
fi
if [ ! -f "$SRC" ]; then
  echo "error: source font '$SRC' not found — fetch the full Noto Sans KR Regular first" >&2
  exit 1
fi

pyftsubset "$SRC" \
  --unicodes="$RANGES" \
  --no-hinting \
  --layout-features='' \
  --drop-tables+=vhea,vmtx,BASE,STAT,GSUB,GPOS,GDEF \
  --output-file="$OUT"

echo "wrote $OUT ($(wc -c < "$OUT") bytes)"
