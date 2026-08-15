fn main() {
    glib_build_tools::compile_resources(
        &["../resources/icons"],
        "../resources/icons/icon.gresource.xml",
        "icon.gresource",
    );
}
