#!/usr/bin/env bash
# Rebuild assets/vidya-symbols.ttf from DejaVu Sans (pyftsubset).
#
# Keeps the font small while covering HIG-style UI punctuation that Ubuntu
# Light lacks on Android (arrows, bullets, disclosure triangles, quotes, …).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/assets/vidya-symbols.ttf"

find_dejavu() {
  if [[ -n "${DEJAVU_SANS:-}" && -f "$DEJAVU_SANS" ]]; then
    echo "$DEJAVU_SANS"
    return
  fi
  local candidates=(
    /nix/store/*-dejavu-fonts-*/share/fonts/truetype/DejaVuSans.ttf
    /run/current-system/sw/share/fonts/truetype/DejaVuSans.ttf
    /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf
    /usr/share/fonts/TTF/DejaVuSans.ttf
  )
  # shellcheck disable=SC2068
  for f in ${candidates[@]}; do
    if [[ -f "$f" ]]; then
      echo "$f"
      return
    fi
  done
  # fc-list fallback
  if command -v fc-list >/dev/null 2>&1; then
    fc-list "DejaVu Sans:style=Book" file 2>/dev/null | head -1 | cut -d: -f1
    return
  fi
  return 1
}

DEJAVU="$(find_dejavu || true)"
if [[ -z "$DEJAVU" || ! -f "$DEJAVU" ]]; then
  echo "error: DejaVu Sans not found; set DEJAVU_SANS=/path/to/DejaVuSans.ttf" >&2
  exit 1
fi

if ! command -v pyftsubset >/dev/null 2>&1; then
  echo "error: pyftsubset not on PATH (nix-shell -p python3Packages.fonttools)" >&2
  exit 1
fi

# Keep in sync with assets/NOTICE.
UNICODES=$(
  cat <<'EOF' | tr '\n' ',' | sed 's/,$//'
U+00B0
U+00B7
U+00D7
U+2013
U+2014
U+2018
U+2019
U+201C
U+201D
U+2022
U+2023
U+2026
U+2039
U+203A
U+2190
U+2191
U+2192
U+2193
U+21D2
U+25A0
U+25A1
U+25B2
U+25B4
U+25B6
U+25B8
U+25BA
U+25BC
U+25BE
U+25C0
U+25C2
U+25C4
U+25CB
U+25CF
U+2713
U+2714
U+2715
U+2717
EOF
)

echo "source: $DEJAVU"
echo "output: $OUT"
pyftsubset "$DEJAVU" \
  --unicodes="$UNICODES" \
  --layout-features='' \
  --glyph-names \
  --symbol-cmap \
  --legacy-cmap \
  --notdef-glyph \
  --notdef-outline \
  --recommended-glyphs \
  --name-IDs='*' \
  --name-legacy \
  --name-languages='*' \
  --output-file="$OUT"

echo "wrote $(wc -c <"$OUT") bytes"
