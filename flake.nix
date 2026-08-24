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
        workspace = pkgs.callPackage ./pkgs/nix/collomatique.nix {};
      in rec {
        packages = rec {
          collomatique = workspace;
          # The python module on its own, for adding to an interpreter of your
          # own making, and a ready interpreter with it in -- `nix build
          # .#python-env` then `./result/bin/python3`. Same two things as
          # pkgs/nix/python-env.nix, which is the flake-less way in.
          collomatique-python = pkgs.python3Packages.callPackage ./pkgs/nix/collomatique-python.nix {
            collomatique = workspace;
          };
          python-env = pkgs.python3.withPackages (_: [ collomatique-python ]);
          default = collomatique;
        };
        apps = rec {
          default = collomatique;
          collomatique = {
            type = "app";
            # The name cargo gives the binary is the gtk4 package's own, since
            # no crate declares a [[bin]] of its own. `command:` in the flatpak
            # manifest and the `rr` alias in .cargo/config.toml name the same
            # one.
            program = "${workspace}/bin/collomatique-gtk4";
          };
        };
      }
    );
}
