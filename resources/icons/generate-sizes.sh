#!/bin/sh
# Regenerate everything derived from the application icon.
#
# collomatique.png is the master, 1024x1024. Icon themes want the usual smaller
# sizes, and those cannot be produced during the flatpak build: gdk-pixbuf now
# decodes through glycin, which re-spawns every loader through flatpak-spawn,
# and a build sandbox has nothing to spawn through. So the scaled copies are
# generated here and committed next to the master. Run this after replacing
# collomatique.png.
#
# collomatique.ico is the same picture in the one format Windows understands,
# and it is generated here for the same reason twice over: the Windows build
# machine has no image tooling on it at all, and colloscopes/gtk4/build.rs embeds this file
# into the executable without looking at what is inside it.
#
# Needs python3 with Pillow. If the python3 in PATH does not have it, the
# script fetches one that does with nix-shell and re-runs itself inside it.
# Just run it.
set -eu

script=$(cd "$(dirname "$0")" && pwd)/$(basename "$0")
cd "$(dirname "$script")"

if python3 -c 'import PIL' 2>/dev/null; then
    python3 - <<'PY'
from PIL import Image

MASTER = "collomatique.png"
SIZES = (128, 256, 512)
ICO = "collomatique.ico"

master = Image.open(MASTER)
if master.size[0] != master.size[1]:
    raise SystemExit(f"{MASTER} is {master.size[0]}x{master.size[1]}, not square")

for size in SIZES:
    path = f"collomatique-{size}.png"
    master.resize((size, size), Image.LANCZOS).save(path, "PNG", optimize=True)
    print("wrote", path)

# One .ico holds every size at once, and Windows picks between them by itself:
# 16 in a title bar, 32 on the taskbar, 48 in a folder, 256 for large icons.
# Pillow's defaults are those seven sizes, downscaled with LANCZOS like the
# copies above, so no size list is written down here.
#
# The entries are stored PNG-compressed rather than as bitmaps, which is also
# Pillow's default. Windows has read that since Vista, and the file is a fifth
# of the size. If some corner of Windows ever draws one of them wrong, the flag
# to try is bitmap_format="bmp".
master.save(ICO)
print("wrote", ICO)
PY
    exit 0
fi

# Second time around Pillow should be there; if it is not, stop rather than
# spawn nix-shell inside nix-shell forever.
if [ "${COLLOMATIQUE_ICON_SIZES_RETRY:-}" = 1 ]; then
    echo "error: python3 still has no Pillow inside nix-shell." >&2
    exit 1
fi

if ! command -v nix-shell >/dev/null 2>&1; then
    echo "error: this python3 has no Pillow, and nix-shell is not available to" >&2
    echo "       fetch one that does. Install Pillow for python3." >&2
    exit 1
fi

echo "python3 has no Pillow: fetching one with nix-shell..." >&2
COLLOMATIQUE_ICON_SIZES_RETRY=1
export COLLOMATIQUE_ICON_SIZES_RETRY
exec nix-shell -p 'python3.withPackages (ps: with ps; [ pillow ])' --run "$script"
