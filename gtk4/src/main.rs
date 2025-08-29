//! Collomatique GTK4 main executable
//!
//! At this date, the goal of this code is to be a gtk4 GUI
//! for the collomatique-core crate.

use clap::Parser;
use collomatique_gtk4::AppModel;
use relm4::RelmApp;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// Collomatique gtk4 UI
struct Opts {
    /// Open Collomatique directly editing a new colloscope
    #[arg(short, long, default_value_t = false)]
    new: bool,

    /// Pass a file as argument to open it with Collomatique
    file: Option<PathBuf>,
}

fn main() {
    let opts = Opts::parse();

    let payload = collomatique_gtk4::AppInit {
        new: opts.new,
        file_name: opts.file,
    };

    let app = RelmApp::new("fr.collomatique.gtk4").with_args(vec![]);
    app.run::<AppModel>(payload);
}
