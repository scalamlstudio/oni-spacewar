#!/usr/bin/env bash
# Renders every SVG in this folder to a 1600x1000 PNG with headless Chrome (macOS path).
set -euo pipefail
cd "$(dirname "$0")"
CHROME="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
for f in *.svg; do
  "$CHROME" --headless --disable-gpu --hide-scrollbars --window-size=1600,1000 \
    --screenshot="${f%.svg}.png" "file://$PWD/$f" >/dev/null 2>&1
  echo "rendered ${f%.svg}.png"
done
