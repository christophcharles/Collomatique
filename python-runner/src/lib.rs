//! python runner
//!
//! Interpreter lifecycle and script execution for the collomatique python modules.

use pyo3::prelude::*;

pub use collomatique_python_old::SharedFileState;

pub fn initialize() {
    use collomatique_python_old::collomatique_old;
    pyo3::append_to_inittab!(collomatique_old);
    Python::initialize();
}

pub fn run_python_script(
    script: String,
    file_state: Option<SharedFileState>,
) -> anyhow::Result<()> {
    // Store shared state for Python to access
    collomatique_python_old::set_current_file_state(file_state);

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

    result
}
