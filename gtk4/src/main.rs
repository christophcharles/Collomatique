//! Collomatique GTK4 main executable
//!
//! At this date, the goal of this code is to be a gtk4 GUI
//! for the collomatique-core crate.

use collomatique_gtk4::AppModel;
use relm4::RelmApp;

fn main() {
    let app = RelmApp::new("fr.collomatique.gtk4");
    app.run::<AppModel>(());
}
