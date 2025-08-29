{
    pkgs ? import <nixpkgs> {}
}:
pkgs.callPackage ./collomatique.nix {
    cbc = pkgs.callPackage ./cbc.nix {};
}
