{
    rustPlatform,
    lib,
    sqlite,
    cbc,
    cmake,
    python3,
}:
rustPlatform.buildRustPackage {
    pname = "collomatique";
    version = "0.1.0";

    src = [ ./. ];

    cargoLock = {
        lockFile = ./Cargo.lock;
    };

    nativeBuildInputs = [
        cmake
        rustPlatform.bindgenHook
        python3
    ];

    buildInputs = [
        sqlite
        cbc
        python3
    ];

    meta = {
        description = "Automatic colloscope building program";
        license = lib.licenses.agpl3Plus;
    };
}
