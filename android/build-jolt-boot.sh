#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:?usage: build-jolt-boot.sh OUTPUT_DIRECTORY}"
JOLT="${JOLT:-$ROOT/scripts/jolt}"
MODULE="${MODULE:-vidya.examples.control-center}"
CHEZ_ANDROID="${CHEZ_ANDROID:-$HOME/.cache/vidya-chez-android}"
HOST_SCHEME="$CHEZ_ANDROID/ta6le/bin/ta6le/scheme"
TARGET_BOOT="$CHEZ_ANDROID/boot/tarm64le"
XPATCH="$CHEZ_ANDROID/xc-tarm64le/s/xpatch"

for path in "$HOST_SCHEME" "$TARGET_BOOT/petite.boot" \
  "$TARGET_BOOT/scheme.boot" "$TARGET_BOOT/scheme.h" "$XPATCH"; do
  [[ -e "$path" ]] || {
    echo "missing Android Chez artifact: $path" >&2
    echo "Build Chez's tarm64le cross target first; see android/README.md." >&2
    exit 1
  }
done
command -v "$JOLT" >/dev/null || {
  echo "Jolt executable not found: $JOLT" >&2
  exit 1
}

mkdir -p "$OUT/project" "$OUT/cross"
cat > "$OUT/project/deps.edn" <<EOF
{:paths ["$ROOT/jolt/src" "$ROOT/jolt/examples"]}
EOF

(
  cd "$OUT/project"
  rm -rf app app.build
  JOLT_NO_FLAT_SPLIT=1 "$JOLT" build \
    -m "$MODULE" -o app
)

cat > "$OUT/cross/compile.ss" <<EOF
(import (chezscheme))
(load "$XPATCH")
(optimize-level 2)
(generate-inspector-information #f)
(compile-file "$OUT/project/app.build/flat.ss" "$OUT/cross/flat.so")
(make-boot-file "$OUT/jolt.boot" '()
  "$TARGET_BOOT/petite.boot"
  "$TARGET_BOOT/scheme.boot"
  "$OUT/cross/flat.so")
EOF

SCHEMEHEAPDIRS="$CHEZ_ANDROID/ta6le/boot/ta6le" \
  "$HOST_SCHEME" --script "$OUT/cross/compile.ss"

cp "$TARGET_BOOT/scheme.h" "$OUT/scheme.h"
