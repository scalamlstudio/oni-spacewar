#!/bin/sh
# Renders a demo-v2 SVG to a PNG of its own size with headless Chrome:
#   design/art/demo-v2/render-svg.sh <name>   (no extension)
set -e
cd "$(dirname "$0")"
w=$(sed -n 's/.*<svg[^>]* width="\([0-9]*\)".*/\1/p' "$1.svg" | head -1)
h=$(sed -n 's/.*<svg[^>]* height="\([0-9]*\)".*/\1/p' "$1.svg" | head -1)
"${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}" \
  --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=1 \
  --default-background-color=00000000 --window-size="$w,$h" \
  --screenshot="$PWD/$1.png" "file://$PWD/$1.svg" 2>/dev/null
