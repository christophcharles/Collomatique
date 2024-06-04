{
    rustPlatform,
    lib,
    sqlite,
    cbc,
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
        cbc
    ];

    meta = {
        description = "Automatic colloscope building program";
        license = lib.licenses.agpl3Plus;
    };
}
