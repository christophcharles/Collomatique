//! Collomatique-python crate
//!
//! This crate contains the code to run python code

use pyo3::prelude::*;

pub fn run_python_script(script: String) -> anyhow::Result<()> {
    pyo3::prepare_freethreaded_python();
    let cscript = std::ffi::CString::new(script)?;
    let flush_script = std::ffi::CString::new(
        "import sys
sys.stdout.flush()
sys.stderr.flush()",
    )?;
    Python::with_gil(|py| {
        py.run(&cscript, None, None)?;
        py.run(&flush_script, None, None)?;
        Ok(())
    })
}
