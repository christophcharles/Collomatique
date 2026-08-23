# The `collomatique` python module, as a member of an interpreter's package set.
#
# A member rather than a flag on `collomatique.nix`, because that is what
# `python3.withPackages` composes with -- see `python-env.nix` next door, and
# the `collomatique-python` output of the flake. It is built from
# `python/pyproject.toml`, so what comes out here is the same wheel a plain
# `maturin build` produces.
{
    lib,
    rustPlatform,
    buildPythonPackage,
    cbc,
    wayland,
    pkg-config,
    collomatique,
}:
buildPythonPackage {
    pname = "collomatique";
    # Read out of the crate rather than written here, and out of *that* crate
    # rather than taken from the application derivation: what buildPythonPackage
    # wants is the version of the wheel, which it checks against the metadata
    # maturin produced. That is the python crate's truncated version, the only
    # one of the two that PEP 440 accepts -- see python/Cargo.toml.
    version = (builtins.fromTOML
        (builtins.readFile ../../python/Cargo.toml)).package.version;
    # The vendor derivation, on the other hand, really is the same one: it is
    # built from Cargo.lock alone, and both builds are of that same workspace.
    # So there is one cargoHash in the tree and nothing here to drift.
    cargoDeps = collomatique.cargoDeps;

    pyproject = true;

    src = lib.cleanSourceWith {
        # The whole repository: the wheel is built from one crate, but the
        # workspace manifest and the lock file are at the root.
        src = ../../.;
        filter = path: type:
            let
                baseName = baseNameOf path;
            in
            # Exclude .git directory and target directory
            !(baseName == ".git" && type == "directory") &&
            !(baseName == "target" && type == "directory");
    };

    # Where the maturin manifest is.
    buildAndTestSubdir = "python";

    nativeBuildInputs = [
        rustPlatform.cargoSetupHook
        rustPlatform.maturinBuildHook
        # `collo-cbc` is in this crate's dependency graph, through the solver.
        rustPlatform.bindgenHook
        pkg-config
    ];

    # Less than the application needs: nothing here draws, so there is no GTK
    # and no icon theme. `wayland` is not a contradiction -- it comes from the
    # native file dialogs of the `dialogs` submodule, which ask the desktop's
    # portal for a window rather than opening one.
    buildInputs = [
        cbc
        wayland
    ];

    # The last rung of `python/src/engine.rs`, baked in at compile time: a
    # solve run from this module re-executes exactly the collomatique it was
    # built against, with nobody having to name a binary. The store path ends
    # up inside the compiled module, so it is a real dependency and stays.
    env.COLLOMATIQUE_DEFAULT_ENGINE = "${collomatique}/bin/collomatique";

    # The test suite is run from the dev shell, not from the package build.
    doCheck = false;

    # Cheap, and exactly the thing this package exists for: the module must
    # import into a plain interpreter with no collomatique around it.
    pythonImportsCheck = [ "collomatique" ];

    meta = {
        description = "Python module of the automatic colloscope building program";
        license = lib.licenses.agpl3Plus;
    };
}
