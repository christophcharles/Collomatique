use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use collomatique::frontend::shell::CliCommand;

mod cli;
mod gui;

#[derive(Debug, Parser)]
#[command(
    version,
    about,
    after_help = "If no command is provided, a GUI is opened."
)]
/// Collomatique - automatic colloscope creation tool
struct Cli {
    /// Create new database - won't override an existing one
    #[arg(short, long, default_value_t = false)]
    create: bool,
    /// Sqlite file (to open or create) that contains the database.
    /// A file must be given if using cli rather than gui
    db: Option<std::path::PathBuf>,
    /// Command to run on the file.
    /// If no command is provided, the GUI will be opened.
    /// You can open an interactive shell command line via the special shell command
    #[command(subcommand)]
    command: Option<CliCommandOrShell>,
}

#[derive(Debug, Subcommand)]
enum CliCommandOrShell {
    #[command(flatten)]
    Global(CliCommand),
    /// Open a shell command line rather than opening a GUI
    Shell,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Some(cmd) => {
            let Some(db) = args.db else {
                return Err(anyhow!(
                    "You must specify a database file when using cli commands."
                ));
            };
            cli::run_cli(args.create, db, cmd)?;
        }
        None => {
            gui::run_gui(args.create, args.db)?;
        }
    }

    Ok(())
}
