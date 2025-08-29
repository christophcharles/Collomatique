{
    rustPlatform,
    lib,
    cbc,
    pkg-config,
}:
rustPlatform.buildRustPackage rec {
    pname = "collomatique";
    version = "0.1.0";

    src = [ ./. ];

    cargoLock = {
        lockFile = ./Cargo.lock;
    };

    nativeBuildInputs = [
        rustPlatform.bindgenHook
        cbc # We need it for tests
    ];

    buildInputs = [
        cbc
    ];

    meta = {
        description = "Automatic colloscope building program";
        license = lib.licenses.agpl3Plus;
    };
}
