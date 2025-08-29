{
  description = "Collomatique - A tool to help build colloscopes in the CPGE French higher education system";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = inputs: with inputs;
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        workspace = pkgs.callPackage ./collomatique.nix {};
      in rec {
        packages = rec {
          collomatique = workspace;
          default = collomatique;
        };
        apps = rec {
          default = collomatique;
          collomatique = {
            type = "app";
            program = "${workspace}/bin/collomatique";
          };
        };
      }
    );
}