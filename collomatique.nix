{
    rustPlatform,
    lib,
    sqlite,
}:
rustPlatform.buildRustPackage {
    pname = "collomatique";
    version = "0.1.0";

    src = [ ./. ];

    cargoLock = {
        lockFile = ./Cargo.lock;
    };

    buildInputs = [
        sqlite
    ];

    meta = {
        description = "Automatic colloscope building program";
        license = lib.licenses.agpl3Plus;
    };
}
