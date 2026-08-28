//! python runner
//!
//! Interpreter lifecycle and script execution for the collomatique python module.

use std::sync::Arc;

use pyo3::prelude::*;

/// The application a hosted script talks to, re-exported for whoever runs one
///
/// Re-exported rather than reached for directly, so that a caller of
/// [run_python_script] needs this crate and nothing else.
pub use collomatique_python::{Host, SendError, TakenDocument};

/// What a script's solves re-execute as their engine, re-exported for the same reason
///
/// Which binary that is, is the caller's to decide — a hosted process is one
/// itself, a bare interpreter is not — so [run_python_script] takes it rather
/// than working it out.
pub use collomatique_python::EngineExe;

/// The version of the Python library this binary is linked against, as
/// `major.minor.micro`
///
/// Read from the library rather than from the headers pyo3 compiled against,
/// so a system that upgrades its Python under an installed Collomatique
/// reports what is really loaded.
///
/// No interpreter is started, and none is needed: `Py_GetVersion` fills a
/// static buffer from constants of its own translation unit and reads no
/// interpreter state (CPython's `Python/getversion.c`). That is what lets the
/// graphical interface show the version without paying for an interpreter it
/// may never use -- scripts run in another process entirely.
pub fn version() -> String {
    // SAFETY: the pointer is to a static buffer that lives as long as the
    // process, filled by the call itself. Valid before [initialize], per the
    // note above.
    let raw = unsafe { std::ffi::CStr::from_ptr(pyo3::ffi::Py_GetVersion()) };
    let full = raw.to_string_lossy();

    // "3.12.13 (main, Jun  3 2026, 09:12:41) [GCC 13.2.0]": the version is
    // everything up to the first space, the rest describes the build.
    full.split_whitespace().next().unwrap_or(&full).to_string()
}

pub fn initialize() {
    use collomatique_python::collomatique;
    pyo3::append_to_inittab!(collomatique);
    Python::initialize();
}

/// Runs one script, hosted or not
///
/// `host` is the application a hosted script talks to. `engine` is the
/// collomatique binary the script's solves re-execute, when the caller is in a
/// position to know one — a script may still name its own, or the environment
/// may. Both are `None` for a script that runs on its own, and both are cleared
/// afterwards, so a second run in the same process starts clean.
pub fn run_python_script(
    script: String,
    host: Option<Arc<dyn Host>>,
    engine: Option<EngineExe>,
) -> anyhow::Result<()> {
    // Store shared state for Python to access
    collomatique_python::set_host(host);
    collomatique_python::set_engine(engine);

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
    collomatique_python::set_host(None);
    collomatique_python::set_engine(None);

    result
}
