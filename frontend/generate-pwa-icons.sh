#!/usr/bin/env bash
# Generate the PWA icon PNG set from icons/icon.svg.
#
# Used locally by developers who want to preview the full icon set
# without a Docker build. The production Dockerfile runs the exact
# same rasterisation inline during the frontend-builder stage, so the
# generated files are identical.
#
# Requires `rsvg-convert` (librsvg). Installation:
#   macOS : brew install librsvg
#   Linux : apt-get install librsvg2-bin  (Debian/Ubuntu)
#           dnf install librsvg2-tools    (Fedora)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ICONS_DIR="$REPO_ROOT/frontend/public/icons"
SOURCE="$ICONS_DIR/icon.svg"
SIZES=(72 96 128 144 152 192 384 512)

if [[ ! -f "$SOURCE" ]]; then
  echo "error: $SOURCE not found" >&2
  exit 1
fi

if ! command -v rsvg-convert >/dev/null 2>&1; then
  echo "error: rsvg-convert not installed." >&2
  echo "  macOS : brew install librsvg" >&2
  echo "  Linux : apt-get install librsvg2-bin  or  dnf install librsvg2-tools" >&2
  exit 2
fi

echo "[pwa-icons] Generating PNG icons from $SOURCE"
cd "$ICONS_DIR"

for size in "${SIZES[@]}"; do
  out="icon-${size}x${size}.png"
  rsvg-convert -w "$size" -h "$size" "$SOURCE" -o "$out"
  printf "  %-28s %s\n" "$out" "$(wc -c < "$out" | tr -d ' ') bytes"
done

echo "[pwa-icons] Done. Generated ${#SIZES[@]} icons in $ICONS_DIR"
