{
    rustPlatform,
    lib,
    sqlite,
    cbc,
    cmake,
    python3,
    fontconfig,
    freetype,
    libglvnd,
    libinput,
    libxkbcommon,
    makeBinaryWrapper,
    mesa,
    pkg-config,
    vulkan-loader,
    wayland,
    xorg
}:
rustPlatform.buildRustPackage rec {
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
        pkg-config
        makeBinaryWrapper
    ];

    buildInputs = [
        sqlite
        cbc
        python3
        fontconfig
        freetype
        libglvnd
        libinput
        libxkbcommon
        vulkan-loader
        wayland
        xorg.libX11
    ];

    # LD_LIBRARY_PATH can be removed once tiny-xlib is bumped above 0.2.2
    postInstall = ''
        wrapProgram "$out/bin/${pname}" \
            --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [
                libxkbcommon
                mesa.drivers
                vulkan-loader
                xorg.libX11
                xorg.libXcursor
                xorg.libXi
                xorg.libXrandr
            ]}
    '';

    meta = {
        description = "Automatic colloscope building program";
        license = lib.licenses.agpl3Plus;
    };
}
