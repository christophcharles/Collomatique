#!/bin/sh
# Build Collomatique as a flatpak and pop out a single installable file.
#
# Nothing is installed on the machine running this: the build ends with a
# .flatpak bundle, the file you would attach to a release. Install it with
#
#     flatpak install --user ./collomatique-<version>.flatpak
#
# The bundle does not carry the runtime it needs (org.gnome.Platform, see the
# manifest); flatpak fetches that from flathub when installing.
#
# Everything is written under an output directory outside the repository, both
# to keep the working tree clean and because the manifest copies that working
# tree into the build sandbox. Pass a different directory as first argument.
#
# Needs flatpak-builder and appstreamcli. If they are not in PATH, the script
# fetches them with nix-shell and re-runs itself inside it. Just run it.
set -eu

script=$(cd "$(dirname "$0")" && pwd)/$(basename "$0")
cd "$(dirname "$script")/.."

out=${COLLOMATIQUE_FLATPAK_OUT:-${1:-${TMPDIR:-/tmp}/collomatique-flatpak}}
export COLLOMATIQUE_FLATPAK_OUT=$out

if ! command -v flatpak-builder >/dev/null 2>&1 || ! command -v appstreamcli >/dev/null 2>&1; then
    # Second time around they should be there; if they are not, stop rather
    # than spawn nix-shell inside nix-shell forever.
    if [ "${COLLOMATIQUE_FLATPAK_BUILD_RETRY:-}" = 1 ]; then
        echo "error: flatpak-builder or appstreamcli still missing inside nix-shell." >&2
        exit 1
    fi

    if ! command -v nix-shell >/dev/null 2>&1; then
        echo "error: flatpak-builder or appstreamcli is missing, and nix-shell is not" >&2
        echo "       available to fetch them. Install the flatpak-builder and appstream" >&2
        echo "       packages." >&2
        exit 1
    fi

    echo "flatpak-builder or appstreamcli missing: fetching them with nix-shell..." >&2
    COLLOMATIQUE_FLATPAK_BUILD_RETRY=1
    export COLLOMATIQUE_FLATPAK_BUILD_RETRY
    exec nix-shell -p flatpak-builder appstream --run "'$script'"
fi

# appstreamcli is called by flatpak-builder itself, at the end of the build, to
# compose the catalogue data software centres read. It also validates
# flatpak/fr.collomatique.Collomatique.metainfo.xml on the way.
if ! command -v flatpak >/dev/null 2>&1; then
    echo "error: flatpak itself is not in PATH; it is needed to make the bundle." >&2
    exit 1
fi

app_id=fr.collomatique.Collomatique
version=$(awk '/^\[workspace\.package\]/ { in_section = 1 }
               in_section && /^version = / { gsub(/"/, "", $3); print $3; exit }' Cargo.toml)
: "${version:=unknown}"

bundle=$out/collomatique-$version.flatpak

# --force-clean below erases $out/build whenever it is not empty, without
# looking at what is in it. That is what we want for a build directory we own,
# but not for a directory that happens to sit at that path: flatpak-builder
# leaves a `metadata` file in every build directory it makes, so use it to tell
# ours apart from someone else's.
if [ -d "$out/build" ] && [ -n "$(ls -A "$out/build" 2>/dev/null)" ] &&
   [ ! -f "$out/build/metadata" ]; then
    echo "error: $out/build is not empty and was not made by flatpak-builder." >&2
    echo "       Building would erase it. Move it away, or pass another output" >&2
    echo "       directory as first argument." >&2
    exit 1
fi

echo "building $app_id $version into $out"
flatpak-builder \
    --force-clean \
    --state-dir="$out/state" \
    --repo="$out/repo" \
    "$out/build" \
    "flatpak/$app_id.yml"

# The bundle is one app taken out of the repository just written. `master` is
# the branch flatpak-builder exports to when the manifest names none.
rm -f "$bundle"
flatpak build-bundle "$out/repo" "$bundle" "$app_id" master

echo
echo "wrote $bundle"
echo "install it with: flatpak install --user \"$bundle\""
