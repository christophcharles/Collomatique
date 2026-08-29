//! Answering command-line arguments in a dialog, on Windows.
//!
//! There is no terminal there — see `windows_stdio` for why, and why that is
//! settled rather than pending. Everything the program would have printed about
//! its own arguments goes nowhere, which for `--help` and a mistyped option
//! means the application appears to start normally.
//!
//! A message box is the only channel left before GTK is up, so that is what
//! these arguments get. The text is clap's own wherever clap produced one: a
//! rewritten help is a help that drifts.

use clap::Parser;
use rfd::{MessageButtons, MessageDialog, MessageLevel};

use crate::Args;

/// What every one of these dialogs is really saying.
const NO_TERMINAL: &str = "Collomatique n'écrit pas dans un terminal sous Windows.";

/// Parse the command line, or explain in a dialog and stop.
///
/// The unix build calls `Args::parse()` directly and keeps its terminal
/// behaviour: `--help` on stdout, an error on stderr, clap's exit codes.
///
/// The exit codes are kept here too. Nothing on Windows is likely to read them,
/// but a wrong one is a lie for free.
pub fn parse() -> Args {
    match Args::try_parse() {
        Ok(args) => args,
        Err(error) => {
            // Help, version and usage errors all arrive as an `Err`. They are
            // told apart by their exit code -- 0 for the two that answered the
            // question, non-zero for the one that reports a mistake.
            let code = error.exit_code();
            let level = if code == 0 {
                MessageLevel::Info
            } else {
                MessageLevel::Error
            };
            show(level, &error.to_string());
            std::process::exit(code);
        }
    }
}

fn show(level: MessageLevel, text: &str) {
    // Blocking, and deliberately so: this runs before GTK, before the main
    // loop, before anything the user could interact with instead.
    let _ = MessageDialog::new()
        .set_title("Collomatique")
        .set_level(level)
        .set_buttons(MessageButtons::Ok)
        .set_description(format!("{NO_TERMINAL}\n\n{text}"))
        .show();
}
