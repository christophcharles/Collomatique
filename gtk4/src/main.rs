//! Collomatique GTK4 main executable
//!
//! At this date, the goal of this code is to be a gtk4 GUI
//! for the collomatique-state crate.

use clap::Parser;
use collomatique_gtk4::AppModel;
use relm4::RelmApp;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// Collomatique gtk4 UI
struct Args {
    /// Ignore all other parameters and run the python engine
    #[arg(long, default_value_t = false)]
    rpc_engine: bool,

    /// Open Collomatique directly editing a new colloscope
    #[arg(short, long, default_value_t = false)]
    new: bool,

    /// Pass a file as argument to open it with Collomatique
    file: Option<PathBuf>,

    /// Everything after gets passed through to GTK.
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    gtk_options: Vec<String>,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    if args.rpc_engine {
        return collomatique_core::rpc::run_rpc_engine();
    }

    let payload = collomatique_gtk4::AppInit {
        new: args.new,
        file_name: args.file,
    };

    let program_invocation = std::env::args().next().unwrap();
    let mut gtk_args = vec![program_invocation];
    gtk_args.extend(args.gtk_options.clone());

    let app = RelmApp::new("fr.collomatique.gtk4").with_args(gtk_args);
    app.allow_multiple_instances(true);
    app.run::<AppModel>(payload);

    Ok(())
}
