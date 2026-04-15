#!/bin/bash
# CSS Preprocessor Hook for Trunk
# Runs css-processor to expand `composes` declarations before Trunk copies CSS files.
#
# Usage:
#   css-hook.sh              # Build css-processor if needed, then process CSS
#   css-hook.sh --skip-build # Skip building css-processor (assumes binary exists)
#   css-hook.sh --dev        # Dev mode: skip processing if watcher is handling it
#
# This script:
# 1. Optionally builds the css-processor binary (skipped with --skip-build)
# 2. Processes styles/ -> styles-dist/ (expanding composes)
# 3. Trunk then copies styles-dist/ to dist/styles/

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CSS_PROCESSOR="$PROJECT_ROOT/target/release/css-processor"
INPUT_DIR="$SCRIPT_DIR/styles"
OUTPUT_DIR="$SCRIPT_DIR/styles-dist"

SKIP_BUILD=false
DEV_MODE=false
for arg in "$@"; do
  case "$arg" in
    --skip-build) SKIP_BUILD=true ;;
    --dev) DEV_MODE=true ;;
  esac
done

# In dev mode, skip processing if styles-dist already exists
# (css-processor --watch maintains it independently)
if [ "$DEV_MODE" = true ] && [ -d "$OUTPUT_DIR" ]; then
  echo "[css-hook] Dev mode: styles-dist/ exists, skipping (watcher handles updates)."
  exit 0
fi

# Build css-processor if not skipping and (binary missing or source newer)
if [ "$SKIP_BUILD" = false ]; then
  if [ ! -f "$CSS_PROCESSOR" ] || [ "$PROJECT_ROOT/css-processor/src/processor/mod.rs" -nt "$CSS_PROCESSOR" ] || [ "$PROJECT_ROOT/css-processor/src/main.rs" -nt "$CSS_PROCESSOR" ]; then
    echo "[css-hook] Building css-processor..."
    cargo build --release -p css-processor --manifest-path "$PROJECT_ROOT/Cargo.toml" 2>&1
    echo "[css-hook] css-processor built successfully."
  fi
else
  if [ ! -f "$CSS_PROCESSOR" ]; then
    echo "[css-hook] ERROR: css-processor binary not found at $CSS_PROCESSOR"
    echo "[css-hook] Run 'cargo make css-process' first, or remove --skip-build flag."
    exit 1
  fi
fi

# Clean output directory
rm -rf "$OUTPUT_DIR"

# Run css-processor
echo "[css-hook] Processing CSS files: $INPUT_DIR -> $OUTPUT_DIR"
"$CSS_PROCESSOR" "$INPUT_DIR" "$OUTPUT_DIR"

echo "[css-hook] CSS preprocessing complete."
