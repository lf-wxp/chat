#!/bin/bash
# Strip environment-specific blocks from dist/index.html after Trunk builds.
#
# Runs as a Trunk post_build hook. Blocks to remove are delimited by HTML
# sentinel comments:
#
#   <!-- PWA_SW_BEGIN --> ... <!-- PWA_SW_END -->
#     Service Worker registration — stripped in DEBUG builds to avoid SW
#     caching interference during development; kept in RELEASE for PWA.
#
#   <!-- CSS_HOT_RELOAD_BEGIN --> ... <!-- CSS_HOT_RELOAD_END -->
#     CSS hot-reload script — kept in DEBUG for live CSS reload; stripped
#     in RELEASE so zero dev-only JS ships to users.
#
# Trunk exposes the profile via TRUNK_PROFILE ("debug" | "release"); the staging
# directory via TRUNK_STAGING_DIR. Both have sensible fallbacks.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROFILE="${TRUNK_PROFILE:-debug}"
STAGING_DIR="${TRUNK_STAGING_DIR:-$SCRIPT_DIR/dist}"
TARGET="$STAGING_DIR/index.html"

if [ ! -f "$TARGET" ]; then
  echo "[strip-dev-hot-reload] $TARGET not found; nothing to strip."
  exit 0
fi

strip_block() {
  local begin_sentinel="$1"
  local end_sentinel="$2"
  local label="$3"

  if ! grep -q "$begin_sentinel" "$TARGET"; then
    echo "[strip-dev-hot-reload] No $begin_sentinel sentinel in $TARGET; already stripped?"
    return 0
  fi

  local tmp
  tmp=$(mktemp)
  # Portable (BSD/GNU sed): delete from BEGIN to END sentinel, inclusive.
  sed "/$begin_sentinel/,/$end_sentinel/d" "$TARGET" > "$tmp"
  mv "$tmp" "$TARGET"
  echo "[strip-dev-hot-reload] Removed $label block from $TARGET (profile=$PROFILE)."
}

if [ "$PROFILE" = "release" ]; then
  # Release: strip CSS hot-reload, keep PWA Service Worker.
  strip_block 'CSS_HOT_RELOAD_BEGIN' 'CSS_HOT_RELOAD_END' 'CSS hot-reload'
else
  # Debug: strip PWA Service Worker, keep CSS hot-reload.
  strip_block 'PWA_SW_BEGIN' 'PWA_SW_END' 'PWA Service Worker'
fi
