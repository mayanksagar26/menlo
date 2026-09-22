#!/usr/bin/env bash
# Render the brand artwork that the macOS bundle ships.
#
#   ./design/brand/build.sh
#
# Two outputs, both committed so a normal `npm run tauri build` needs no browser:
#
#   src-tauri/dmg-background.tiff  the DMG window background
#   src-tauri/icons/*              the app icon set, via `tauri icon`
#
# The background ships as a two-representation TIFF — 660×488 and 1320×976 in one
# file — which is what Finder wants for a HiDPI background picture. It picks the rep
# that matches the display, so the sheet is sharp on retina and correctly sized on
# everything else.
#
# 488 is the artwork height, not the window height. Finder counts its own 28pt title
# bar in `windowSize`, so tauri.conf.json says 516. Match the artwork to the window
# and the bottom 28pt gets clipped.
#
# `--default-background-color=00000000` is load-bearing for the icon. Without it the
# browser composites the page onto opaque white and the icon ships with a white
# square baked in behind the squircle, which macOS then draws as an unintended border
# in the Dock, in Finder, and on this very disk image. Chrome is driven directly
# rather than through a screenshot tool because the tools generally do not expose it.
set -euo pipefail

cd "$(dirname "$0")/../.."

chrome=""
for candidate in \
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
  "/Applications/Chromium.app/Contents/MacOS/Chromium" \
  "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"
do
  [[ -x "$candidate" ]] && { chrome="$candidate"; break; }
done

if [[ -z "$chrome" ]]; then
  echo "No Chrome-family browser found. It is only needed to rasterise the artwork," >&2
  echo "and the results are committed, so this is not a build dependency. Install" >&2
  echo "Chrome, or open the files in design/brand/ and export the .dmg element at" >&2
  echo "1320x976 and the icon at 1024x1024, both on a transparent background." >&2
  exit 1
fi

# shot <html> <out.png> <width> <height> <device-scale>
shot() {
  "$chrome" \
    --headless \
    --disable-gpu \
    --hide-scrollbars \
    --default-background-color=00000000 \
    --force-device-scale-factor="$5" \
    --window-size="$3,$4" \
    --screenshot="$2" \
    "file://$PWD/$1" >/dev/null 2>&1
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "→ dmg background"
shot design/brand/dmg-background.html "$tmp/bg@2x.png" 660 488 2
sips -z 488 660 "$tmp/bg@2x.png" --out "$tmp/bg.png" >/dev/null
tiffutil -cathidpicheck "$tmp/bg.png" "$tmp/bg@2x.png" \
  -out src-tauri/dmg-background.tiff >/dev/null

echo "→ app icon"
shot design/brand/icon.html "$tmp/icon-1024.png" 1024 1024 1
cp "$tmp/icon-1024.png" design/brand/icon-1024.png
npm run --silent tauri icon -- design/brand/icon-1024.png >/dev/null
# This project is macOS-only; `tauri icon` also emits phone icon sets.
rm -rf src-tauri/icons/android src-tauri/icons/ios

# The white-square bug is silent and ships all the way to the Dock, so assert it.
python3 - <<'PY'
from PIL import Image

icon = Image.open("src-tauri/icons/128x128@2x.png").convert("RGBA")
if icon.getpixel((1, 1))[3] != 0:
    raise SystemExit("   icon corner is opaque — the transparent background was lost")
print("   icon corner is transparent")
PY

echo "done"
