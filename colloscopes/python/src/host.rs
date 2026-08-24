//! The application a script may be running inside
//!
//! A script started from the GUI's script editor is *hosted*: the application
//! hands it the document it has open, and takes one back if the script asks it
//! to. `docs/python/new_api_design.md` §9.2 is the design.
//!
//! This crate knows nothing about how the two sides talk. The host is a trait
//! whoever runs the interpreter implements — in the application that is the rpc
//! engine, over the pipe to the GUI — so the protocol stays in the crate that
//! owns it, and the tests here can run a script against a fake.

use std::sync::{Arc, Mutex};

use pyo3::prelude::*;

use collomatique_state_colloscopes::Data;

use crate::Document;
use crate::errors::{NotHosted, SaveError};

/// The application a hosted script is running inside
///
/// Both methods carry the whole document: the handoff is a document, not a
/// stream of edits, in either direction.
pub trait Host: Send + Sync {
    /// The document the application is offering
    ///
    /// Answered from what the runner already fetched, so this does not go back
    /// to the application on every call.
    fn data(&self) -> Data;

    /// Replaces what the application holds. Not a merge.
    ///
    /// The `Err` is a sentence for the script to read: whatever went wrong is
    /// the transport's business, and there is nothing a script can do about it
    /// but report it.
    fn send(&self, data: &Data) -> Result<(), String>;
}

/// The application the current run is talking to, if there is one
static HOST: Mutex<Option<Arc<dyn Host>>> = Mutex::new(None);

/// The document [current_document] built, so a second call answers the first
///
/// Built on demand rather than up front: a hosted script that never asks for
/// the document should not pay for one.
static HOSTED_DOCUMENT: Mutex<Option<Py<Document>>> = Mutex::new(None);

/// Installs, or clears, the host for the coming run
///
/// The runner calls this on both sides of a script. Clearing also drops the
/// document [current_document] built, so a second run in the same process
/// starts from the application's document rather than from the first script's.
pub fn set_host(host: Option<Arc<dyn Host>>) {
    // Taken first, and dropped after both locks are released: dropping a
    // `Py<Document>` from a thread that is not attached to the interpreter is
    // allowed — pyo3 defers the reference-count decrement — and this is called
    // from outside `Python::attach`, on the way in and out of a run.
    let previous = HOSTED_DOCUMENT.lock().unwrap().take();
    *HOST.lock().unwrap() = host;
    drop(previous);
}

/// The document the application is hosting, or `None` when standalone
///
/// The script works on a copy. Editing it changes nothing in the application,
/// `undo()` is invisible there, and only `send_to_host` — or `doc.save()`,
/// which is the same thing for this document — crosses back.
///
/// Calling it twice gives the same object, so an edit made through one call is
/// there through the other:
///
/// ```python
/// doc = clm.current_document()
/// if doc is None:
///     doc = clm.load(sys.argv[1])
/// ```
#[pyfunction]
pub fn current_document(py: Python<'_>) -> PyResult<Option<Py<Document>>> {
    if let Some(doc) = HOSTED_DOCUMENT.lock().unwrap().as_ref() {
        return Ok(Some(doc.clone_ref(py)));
    }

    let Some(host) = HOST.lock().unwrap().clone() else {
        return Ok(None);
    };

    // Built with no lock held: `Host::data` belongs to the runner and this
    // crate does not get to assume it is quick.
    let doc = Py::new(py, Document::hosted(host.data()))?;

    let mut cached = HOSTED_DOCUMENT.lock().unwrap();
    // Another thread may have got here first while this one was building; the
    // document it cached is then the one every caller has to see, since the
    // promise is that two calls give the same object.
    Ok(Some(cached.get_or_insert(doc).clone_ref(py)))
}

/// Hands a document to the application, replacing what it holds
///
/// It takes any document, not only the hosted one, because its subject is the
/// application's slot rather than the document: building next year's file from
/// a template and a csv and dropping it into the open application, or loading
/// a backup to repair a broken document, are ordinary scripts.
///
/// Two things it does not do. **It is not a merge** — it replaces the
/// application's whole document, so sending a different file wipes what the
/// user had, and the application's validation step is the only safety net. And
/// **it does not happen on its own**: a hosted script that edits its document
/// and never calls this changes nothing, which the application says plainly
/// rather than leaving the script to guess.
///
/// Sending twice is allowed, and the last one wins. That is what makes sending
/// a document other than the hosted one composable.
///
/// Raises `NotHosted` when the script is standalone, since there is then
/// nothing to send to and doing nothing quietly would hide it.
#[pyfunction]
pub fn send_to_host(py: Python<'_>, doc: &Document) -> PyResult<()> {
    let Some(host) = HOST.lock().unwrap().clone() else {
        return Err(NotHosted::new_err(
            "this script is not running inside collomatique, so there is nothing to send \
             the document to; save() it to a file instead",
        ));
    };

    let data = doc.data().clone();

    // The GIL goes back for the trip: the real host blocks on a round trip to
    // the application, and holding it through that would freeze every other
    // thread the script started.
    py.detach(|| host.send(&data))
        .map_err(|e| SaveError::new_err(format!("collomatique refused the document: {e}")))
}

/// Adds the host functions to the module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(current_document, m)?)?;
    m.add_function(wrap_pyfunction!(send_to_host, m)?)?;
    Ok(())
}
