//! Collomatique GTK4 main executable
//!
//! At this date, the goal of this code is to be a gtk4 GUI
//! for the collomatique-state crate.

// Windows decides whether to give a process a console from a flag in the
// executable itself, not from how it is started. Without this, launching the
// GUI opens a console window beside it, and that window is what takes the
// focus.
//
// The `--rpc-engine` child is this same executable, so it loses its console
// too. That costs it nothing: its standard streams are pipes the parent
// creates and hands over, which need no console at either end.
//
// The cost is that nothing has a console to print to any more -- `--help`,
// `--version`, `--debug` output, a clap parse error. `windows_console::attach`
// gives those a destination again. Unix is unaffected throughout.
#![cfg_attr(windows, windows_subsystem = "windows")]

use clap::Parser;
use collomatique_gtk4::AppModel;
use relm4::RelmApp;
use std::path::PathBuf;

mod debug;
#[cfg(windows)]
mod windows_console;

const WORKER_THREAD_COUNT: usize = 4;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// Collomatique gtk4 UI
struct Args {
    /// Ignore all other parameters and run the python engine
    #[arg(long, default_value_t = false)]
    rpc_engine: bool,

    /// Run in debug mode (requires a file argument)
    #[arg(long, value_enum)]
    debug: Option<debug::DebugMode>,

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
    // Before the parse, because a parse error is one of the things this makes
    // visible again.
    #[cfg(windows)]
    windows_console::attach();

    let args = Args::parse();

    if args.rpc_engine {
        return collomatique_rpc_engine::run_rpc_engine();
    }

    if let Some(mode) = args.debug {
        if matches!(mode, debug::DebugMode::Help) {
            return debug::print_help();
        }
        let file = args.file.expect("--debug requires a file argument");
        return debug::run(mode, file);
    }

    let payload = collomatique_gtk4::AppInit {
        new: args.new,
        file_name: args.file,
    };

    let program_invocation = std::env::args().next().unwrap();
    let mut gtk_args = vec![program_invocation];
    gtk_args.extend(args.gtk_options.clone());

    relm4::RELM_THREADS
        .set(WORKER_THREAD_COUNT)
        .expect("RELM_THREADS should not have been set before");

    let app = RelmApp::new("fr.collomatique.Collomatique").with_args(gtk_args);
    app.allow_multiple_instances(true);
    app.run::<AppModel>(payload);

    Ok(())
}
