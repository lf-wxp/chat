#!/bin/bash
# Strip the CSS hot-reload <script> block from dist/index.html in release builds.
#
# Runs as a Trunk post_build hook. The block to remove is delimited by the
# HTML sentinel comments:
#
#   <!-- CSS_HOT_RELOAD_BEGIN -->
#   ...
#   <!-- CSS_HOT_RELOAD_END -->
#
# In debug builds (trunk serve / trunk build) the block is kept intact so the
# hot-reload works locally. In release builds (trunk build --release) the block
# is physically removed, so the shipped index.html contains zero dev-only JS.
#
# Trunk exposes the profile via TRUNK_PROFILE ("debug" | "release"); the staging
# directory via TRUNK_STAGING_DIR. Both have sensible fallbacks.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROFILE="${TRUNK_PROFILE:-debug}"
STAGING_DIR="${TRUNK_STAGING_DIR:-$SCRIPT_DIR/dist}"
TARGET="$STAGING_DIR/index.html"

if [ "$PROFILE" != "release" ]; then
  echo "[strip-dev-hot-reload] Profile=$PROFILE; keeping hot-reload block."
  exit 0
fi

if [ ! -f "$TARGET" ]; then
  echo "[strip-dev-hot-reload] $TARGET not found; nothing to strip."
  exit 0
fi

if ! grep -q 'CSS_HOT_RELOAD_BEGIN' "$TARGET"; then
  echo "[strip-dev-hot-reload] No CSS_HOT_RELOAD_BEGIN sentinel in $TARGET; already stripped?"
  exit 0
fi

TMP=$(mktemp)
# Portable (BSD/GNU sed): delete from BEGIN to END sentinel, inclusive.
sed '/<!-- CSS_HOT_RELOAD_BEGIN -->/,/<!-- CSS_HOT_RELOAD_END -->/d' "$TARGET" > "$TMP"
mv "$TMP" "$TARGET"

echo "[strip-dev-hot-reload] Removed dev hot-reload block from $TARGET (profile=release)."
