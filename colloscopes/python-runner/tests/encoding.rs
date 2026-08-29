//! The hosted interpreter speaks UTF-8, whatever the locale says
//!
//! Everything that reads a hosted interpreter's output decodes UTF-8, so the
//! interpreter has to produce it -- on windows, where the locale's encoding is
//! a code page, that only happens because `initialize` asks for UTF-8 mode.

use std::sync::Once;

static INIT: Once = Once::new();

/// Registers the module and starts the interpreter, at most once per process
fn interpreter() {
    INIT.call_once(collomatique_python_runner::initialize);
}

/// The assertions are Python's: one that fails raises, and `run_python_script`
/// hands the error back here.
#[test]
fn the_interpreter_is_in_utf8_mode() {
    interpreter();

    let script = "\
import sys
assert sys.flags.utf8_mode == 1, sys.flags.utf8_mode
assert sys.stdout.encoding.lower().replace('-', '') == 'utf8', sys.stdout.encoding
assert sys.stderr.encoding.lower().replace('-', '') == 'utf8', sys.stderr.encoding
"
    .to_string();

    collomatique_python_runner::run_python_script(script, None, None)
        .expect("the interpreter should be in UTF-8 mode");
}
