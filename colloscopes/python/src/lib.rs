//! The `collomatique` python module
//!
//! Running an interpreter is `collomatique-python-runner`'s job; this crate
//! only defines what the module contains.

use pyo3::prelude::*;

pub mod blame;
pub mod caveats;
pub mod collections;
pub mod data;
pub mod dialogs;
pub mod document;
pub mod engine;
pub mod errors;
pub mod generation;
pub mod handles;
pub mod host;
pub mod ids;
pub mod model;
mod payload;
pub mod refs;
pub mod results;
pub mod solve;
pub mod transaction;
pub mod values;

pub use dialogs::{Dialogs, FileRequest, set_dialogs};
pub use document::Document;
pub use engine::{EngineExe, set_engine};
pub use host::{Host, SendError, TakenDocument, set_host};
pub use model::ColloscopeModel;
pub use transaction::Transaction;

/// The `collomatique` python module, for registration in an interpreter's inittab.
#[pymodule]
pub fn collomatique(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add(
        "__version__",
        collomatique_settings::current_version().to_string(),
    )?;

    blame::register(m)?;
    caveats::register(m)?;
    collections::register(m)?;
    data::register(m)?;
    dialogs::register(m)?;
    errors::register(m)?;
    generation::register(m)?;
    host::register(m)?;
    ids::register(m)?;
    model::register(m)?;
    refs::register(m)?;
    results::register(m)?;
    solve::register(m)?;
    values::register(m)?;

    m.add_class::<Document>()?;
    // Registered so `isinstance` and `repr` say something useful, like the
    // collection views are; `doc.transaction(...)` is the only way to build one.
    m.add_class::<Transaction>()?;
    m.add_function(wrap_pyfunction!(document::load, m)?)?;
    m.add_function(wrap_pyfunction!(document::new_document, m)?)?;
    m.add_function(wrap_pyfunction!(document::default_document, m)?)?;

    Ok(())
}
