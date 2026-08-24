#!/bin/sh
# Regenerate pkgs/flatpak/cargo-sources.json from Cargo.lock.
#
# The flatpak build sandbox has no network, so every crate cargo will need must
# be declared in the manifest beforehand. cargo-sources.json is that list, and
# it has to be regenerated every time Cargo.lock changes.
#
# flatpak-cargo-generator.py next to this script is a verbatim copy of
# cargo/flatpak-cargo-generator.py from the flatpak/flatpak-builder-tools
# repository, at commit f03a673abe6ce189cea1c2857e2b44af2dd79d1f. It needs
# python3 (3.9 or later) with the aiohttp and tomlkit modules, and it reads the
# checksum of every crate from crates.io, so this script — unlike the flatpak
# build itself — does need network access.
#
# If the python3 in PATH does not have those two modules, the script fetches
# one that does with nix-shell and re-runs itself inside it. Just run it.
set -eu

script=$(cd "$(dirname "$0")" && pwd)/$(basename "$0")
cd "$(dirname "$script")/../.."

if python3 -c 'import aiohttp, tomlkit' 2>/dev/null; then
    python3 pkgs/flatpak/flatpak-cargo-generator.py Cargo.lock -o pkgs/flatpak/cargo-sources.json
    echo "wrote pkgs/flatpak/cargo-sources.json"
    exit 0
fi

# Second time around the modules should be there; if they are not, stop rather
# than spawn nix-shell inside nix-shell forever.
if [ "${COLLOMATIQUE_CARGO_SOURCES_RETRY:-}" = 1 ]; then
    echo "error: python3 still has no aiohttp/tomlkit inside nix-shell." >&2
    exit 1
fi

if ! command -v nix-shell >/dev/null 2>&1; then
    echo "error: this python3 has no aiohttp/tomlkit, and nix-shell is not available" >&2
    echo "       to fetch one that does. Install both modules for python3." >&2
    exit 1
fi

echo "python3 has no aiohttp/tomlkit: fetching one with nix-shell..." >&2
COLLOMATIQUE_CARGO_SOURCES_RETRY=1
export COLLOMATIQUE_CARGO_SOURCES_RETRY
exec nix-shell -p 'python3.withPackages (ps: with ps; [ aiohttp tomlkit ])' --run "$script"
