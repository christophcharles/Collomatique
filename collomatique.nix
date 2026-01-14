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
    python3,
}:
rustPlatform.buildRustPackage rec {
    pname = "collomatique";
    version = "0.1.0";

    src = lib.cleanSourceWith {
        src = ./.;
        filter = path: type:
            let
                baseName = baseNameOf path;
            in
            # Exclude .git directory and target directory
            !(baseName == ".git" && type == "directory") &&
            !(baseName == "target" && type == "directory");
    };

    cargoHash = "sha256-LbAt66gVeIUZJAk4X9mt44JUDd9VR5uldCCs1m8a0FQ=";

    nativeBuildInputs = [
        rustPlatform.bindgenHook
        gettext
        pkg-config
        wrapGAppsHook4
        cbc # We need it for tests
        python3
    ];

    buildInputs = [
        cbc
        gdk-pixbuf
        glib
        gtk4
        libadwaita
        wayland
        adwaita-icon-theme
        python3
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
