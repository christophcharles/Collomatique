{
    rustPlatform,
    lib,
    cbc,
    pkg-config,
    gettext,
    wrapGAppsHook4,
    gdk-pixbuf,
    glib,
    gtk4,
    wayland,
    libadwaita,
    adwaita-icon-theme,
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
        gettext
        pkg-config
        wrapGAppsHook4
        cbc # We need it for tests
    ];

    buildInputs = [
        cbc
        gdk-pixbuf
        glib
        gtk4
        libadwaita
        wayland
        adwaita-icon-theme
    ];

    preFixup = ''
        gappsWrapperArgs+=(
            --prefix XDG_DATA_DIRS : "${gtk4}/share/gsettings-schemas/${gtk4.name}"
        )
    '';

    shellHook = ''
        export XDG_DATA_DIRS="${gtk4}/share/gsettings-schemas/${gtk4.name}:$XDG_DATA_DIRS"
    '';

    meta = {
        description = "Automatic colloscope building program";
        license = lib.licenses.agpl3Plus;
    };
}
