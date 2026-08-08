//! python module
//!
//! This crate contains the python module glue: the pyclasses and pyfunctions of the
//! `collomatique_old` module. Running an interpreter is `collomatique-python-runner`'s job.

use std::sync::{Arc, Mutex};

use collomatique_ops::Desc;
use collomatique_state::AppState;
use collomatique_state_colloscopes::Data;

mod glue;

/// The `collomatique_old` python module, for registration in an interpreter's inittab.
pub use glue::collomatique_old;

/// Shared file state type used to pass local data to Python scripts
pub type SharedFileState = Arc<Mutex<AppState<Data, Desc>>>;

static CURRENT_FILE_STATE: Mutex<Option<SharedFileState>> = Mutex::new(None);

pub(crate) fn get_current_file_state() -> Option<SharedFileState> {
    CURRENT_FILE_STATE.lock().unwrap().clone()
}

/// Set (or clear) the file state that `collomatique_old.current_session()` hands to scripts.
///
/// Called by the runner around a script run.
pub fn set_current_file_state(file_state: Option<SharedFileState>) {
    *CURRENT_FILE_STATE.lock().unwrap() = file_state;
}
