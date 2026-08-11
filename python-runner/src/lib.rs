//! python runner
//!
//! Interpreter lifecycle and script execution for the collomatique python modules.

use std::sync::Arc;

use pyo3::prelude::*;

pub use collomatique_python_old::SharedFileState;

/// The application a hosted script talks to, re-exported for whoever runs one
///
/// Re-exported rather than reached for directly, so that a caller of
/// [run_python_script] needs this crate and nothing else.
pub use collomatique_python::Host;

pub fn initialize() {
    use collomatique_python::collomatique;
    use collomatique_python_old::collomatique_old;
    pyo3::append_to_inittab!(collomatique);
    pyo3::append_to_inittab!(collomatique_old);
    Python::initialize();
}

/// Runs one script, hosted or not
///
/// `file_state` is what the old module hands its scripts; `host` is what the
/// new one hands its own (`docs/python/new_api_design.md` §9.2). Both are
/// `None` for a script that runs on its own, and both are cleared afterwards,
/// so a second run in the same process starts clean.
pub fn run_python_script(
    script: String,
    file_state: Option<SharedFileState>,
    host: Option<Arc<dyn Host>>,
) -> anyhow::Result<()> {
    // Store shared state for Python to access
    collomatique_python_old::set_current_file_state(file_state);
    collomatique_python::set_host(host);

    let cscript = std::ffi::CString::new(script)?;
    let flush_script = std::ffi::CString::new(
        "import sys
sys.stdout.flush()
sys.stderr.flush()",
    )?;
    let result = Python::attach(|py| {
        py.run(&cscript, None, None)?;
        py.run(&flush_script, None, None)?;
        Ok(())
    });

    // Clear the shared state
    collomatique_python_old::set_current_file_state(None);
    collomatique_python::set_host(None);

    result
}
