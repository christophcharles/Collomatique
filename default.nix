{
    pkgs ? import <nixpkgs> {}
}:
pkgs.callPackage ./pkgs/nix/collomatique.nix {}
