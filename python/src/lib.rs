//! python module
//!
//! This crate contains the code to run python code

use std::sync::{Arc, Mutex};

use collomatique_ops::Desc;
use collomatique_state::AppState;
use collomatique_state_colloscopes::Data;
use pyo3::prelude::*;

mod glue;

/// Shared file state type used to pass local data to Python scripts
pub type SharedFileState = Arc<Mutex<AppState<Data, Desc>>>;

static CURRENT_FILE_STATE: Mutex<Option<SharedFileState>> = Mutex::new(None);

pub(crate) fn get_current_file_state() -> Option<SharedFileState> {
    CURRENT_FILE_STATE.lock().unwrap().clone()
}

pub fn initialize() {
    use glue::collomatique;
    pyo3::append_to_inittab!(collomatique);
    Python::initialize();
}

pub fn run_python_script(
    script: String,
    file_state: Option<SharedFileState>,
) -> anyhow::Result<()> {
    // Store shared state for Python to access
    {
        let mut guard = CURRENT_FILE_STATE.lock().unwrap();
        *guard = file_state;
    }

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
    {
        let mut guard = CURRENT_FILE_STATE.lock().unwrap();
        *guard = None;
    }

    result
}
