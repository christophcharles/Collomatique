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
// The cost is the command line: with no console, `--help`, `--version` and
// `--debug` have nowhere to print. `windows_stdio` stops that from being fatal,
// and `windows_cli` answers in a dialog instead of in silence. Unix is
// unaffected throughout, command line included.
#![cfg_attr(windows, windows_subsystem = "windows")]

use anyhow::Context as _;
use clap::{ArgGroup, Parser};
use collomatique_gtk4::AppModel;
use relm4::RelmApp;
use std::path::PathBuf;

mod debug;
#[cfg(windows)]
mod windows_cli;
#[cfg(windows)]
mod windows_stdio;

const WORKER_THREAD_COUNT: usize = 4;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
// The two python arguments name one script between them, so they exclude each
// other; and a script runs instead of the application, so they exclude
// everything that says what the application should open or do. The group is
// also what `--python-no-engine` hangs off: that flag only means anything
// about a script, so without one it is a mistake rather than a no-op.
#[command(group(
    ArgGroup::new("script")
        .args(["python", "python_file"])
        .conflicts_with_all(["rpc_engine", "debug", "new"]),
))]
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

    /// Run the given python code with the collomatique module, no UI
    #[arg(long)]
    python: Option<String>,

    /// Run the given python script with the collomatique module, no UI
    #[arg(long)]
    python_file: Option<PathBuf>,

    /// With --python/--python-file: do not offer this executable as the
    /// solve engine (the script then needs engine= or COLLOMATIQUE_ENGINE)
    #[arg(long, default_value_t = false, requires = "script")]
    python_no_engine: bool,

    /// Pass a file as argument to open it with Collomatique
    file: Option<PathBuf>,

    /// Everything after gets passed through to GTK.
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    gtk_options: Vec<String>,
}

fn main() -> Result<(), anyhow::Error> {
    // Before the parse, because clap reports a bad argument by printing, and
    // printing is what has to be made harmless.
    #[cfg(windows)]
    windows_stdio::discard_output();

    // On windows the parse can end in a dialog rather than in a return: the
    // arguments that only make sense at a terminal are refused there, `--debug`
    // among them, so the block below is unix-only in practice.
    #[cfg(windows)]
    let args = windows_cli::parse();
    #[cfg(not(windows))]
    let args = Args::parse();

    if args.rpc_engine {
        return collomatique_rpc_engine::run_rpc_engine();
    }

    if let Some(code) = python_code(&args)? {
        // The same door the graphical interface's script runner uses, minus
        // the host: nothing is open, so `current_document()` is None and the
        // script works on files it names itself.
        //
        // This process is a collomatique binary, so it is an engine a solve
        // can re-execute, and it says so — unless `--python-no-engine`
        // withholds it, which is what lets a script (or a test) exercise the
        // other rungs of `python`'s engine resolution.
        let engine =
            (!args.python_no_engine).then_some(collomatique_python_runner::EngineExe::Current);
        collomatique_python_runner::initialize();
        return collomatique_python_runner::run_python_script(code, None, None, engine);
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

/// The script the two python arguments name between them, if either was given
///
/// `--python` is the code itself; `--python-file` is a path to read it from.
/// The clap group above has already made sure at most one of them is here, so
/// the order of the two tests is not a precedence.
///
/// A file that cannot be read is an ordinary error naming the path: the script
/// never starts, and nothing has been done yet that would need undoing.
fn python_code(args: &Args) -> Result<Option<String>, anyhow::Error> {
    if let Some(code) = &args.python {
        return Ok(Some(code.clone()));
    }

    let Some(path) = &args.python_file else {
        return Ok(None);
    };

    let code = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read the python script {}", path.display()))?;

    Ok(Some(code))
}
