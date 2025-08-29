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
        cbc # We need it for tests
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

    # Force linking to libEGL, which is always dlopen()ed, and to
    # libwayland-client, which is always dlopen()ed except by the
    # obscure winit backend.
    RUSTFLAGS = map (a: "-C link-arg=${a}") [
        "-Wl,--push-state,--no-as-needed"
        "-lEGL"
        "-lwayland-client"
        "-Wl,--pop-state"
    ];

    shellHook = ''
        export LD_LIBRARY_PATH="${lib.makeLibraryPath [
            xorg.libX11 xorg.libXcursor xorg.libXi xorg.libXrandr vulkan-loader libxkbcommon mesa.drivers wayland
            ]}:$LD_LIBRARY_PATH"
    '';

    # LD_LIBRARY_PATH can be removed once tiny-xlib is bumped above 0.2.2
    postInstall = ''
        wrapProgram "$out/bin/${pname}" \
            --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [
            xorg.libX11 xorg.libXcursor xorg.libXi xorg.libXrandr vulkan-loader libxkbcommon mesa.drivers wayland
            ]}
    '';

    meta = {
        description = "Automatic colloscope building program";
        license = lib.licenses.agpl3Plus;
    };
}
