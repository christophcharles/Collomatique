fn main() {
    glib_build_tools::compile_resources(
        &["../resources/icons"],
        "../resources/icons/icon.gresource.xml",
        "icon.gresource",
    );

    embed_windows_icon();
}

/// Put the application icon into the executable itself.
///
/// Windows draws it in three places, and all three read this one resource: on
/// the .exe in Explorer, on the Start-menu shortcut the installer creates
/// pointing at it, and on `.collomatique` files through the `DefaultIcon` the
/// installer registers as `collomatique-gtk4.exe,0`. There is no separate
/// document artwork, which is the same choice the flatpak makes in its
/// `mime.xml`.
///
/// The `.ico` is generated and committed by `resources/icons/generate-sizes.sh`
/// rather than built here: the Windows build machine has no image tooling on it.
///
/// The dependency and this function appear and vanish together — see
/// `Cargo.toml`. A build script is compiled for the machine that runs it, and we
/// never cross-compile, so "the host is Windows" and "we are building for
/// Windows" are the same statement here.
#[cfg(windows)]
fn embed_windows_icon() {
    const ICON: &str = "../resources/icons/collomatique.ico";

    println!("cargo:rerun-if-changed={ICON}");

    let mut res = winresource::WindowsResource::new();
    res.set_icon(ICON);

    // Without these, what Windows shows in the file's properties and beside the
    // process in Task Manager comes from the crate: "collomatique-gtk4", and an
    // empty description because the crate declares none.
    res.set("ProductName", "Collomatique");
    res.set("FileDescription", "Collomatique");

    res.compile()
        .expect("could not compile the icon resource: this needs rc.exe from the Windows SDK");
}

#[cfg(not(windows))]
fn embed_windows_icon() {}
