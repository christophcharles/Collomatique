//! The Collomatique application icon, embedded in the binary.
//!
//! The PNG is compiled into a GResource at build time (see `build.rs`), and
//! the GResource itself is embedded here. Registering it makes the icon
//! available to GTK's icon theme, so it can be used by icon name anywhere
//! (the about dialog, the main window, etc.).

use relm4::gtk;

pub const ICON_NAME: &str = "collomatique";

const ICON_RESOURCE_PATH: &str = "/fr/collomatique/gtk4/icons";

pub fn register() -> Result<(), gtk::glib::Error> {
    gtk::gio::resources_register_include!("icon.gresource")?;
    gtk::IconTheme::for_display(&gtk::gdk::Display::default().expect("no display"))
        .add_resource_path(ICON_RESOURCE_PATH);
    Ok(())
}
