# A python interpreter with the collomatique module already in it.
#
#     nix-build pkgs/nix/python-env.nix
#     ./result/bin/python3 -c 'import collomatique'
#     ./result/bin/python3 my_script.py
#
# or, to get that interpreter on the PATH of an interactive shell:
#
#     nix-shell pkgs/nix/python-env.nix -A env
#
# The `-A env` matters. A `withPackages` environment is a plain buildEnv, so a
# bare `nix-shell` on it drops you in the environment that *builds* the
# wrapper, which has no interpreter of interest in it; `.env` is the shell
# attribute nixpkgs puts on these for exactly this.
#
# Deliberately one derivation and not a set: `nix-build` above should produce
# the interpreter, not pick something out of a menu. The flake exposes the same
# two things as `.#collomatique-python` and `.#python-env`.
{
    pkgs ? import <nixpkgs> {}
}:
let
    collomatique = pkgs.callPackage ./collomatique.nix {};
    # Through the interpreter's own `callPackage`, so `buildPythonPackage` and
    # the set it is going into are the same one.
    collomatique-python = pkgs.python3Packages.callPackage ./collomatique-python.nix {
        inherit collomatique;
    };
in
pkgs.python3.withPackages (_: [ collomatique-python ])
