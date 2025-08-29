{
    pkgs ? import <nixpkgs> {}
}:
pkgs.callPackage ./collomatique.nix {}
