//! The `collomatique` python module
//!
//! This crate is the new scripting API described in
//! `docs/python/new_api_design.md`. Running an interpreter is
//! `collomatique-python-runner`'s job; this crate only defines what the module
//! contains.

use pyo3::prelude::*;

pub mod caveats;
pub mod collections;
pub mod document;
pub mod errors;
pub mod results;

pub use document::Document;

/// The `collomatique` python module, for registration in an interpreter's inittab.
#[pymodule]
pub fn collomatique(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();

    m.add(
        "__version__",
        collomatique_settings::current_version().to_string(),
    )?;

    m.add("Error", py.get_type::<errors::Error>())?;
    m.add("LoadError", py.get_type::<errors::LoadError>())?;
    m.add("SaveError", py.get_type::<errors::SaveError>())?;
    m.add("NoOrigin", py.get_type::<errors::NoOrigin>())?;
    m.add("UpdateError", py.get_type::<errors::UpdateError>())?;
    m.add("NothingToUndo", py.get_type::<errors::NothingToUndo>())?;
    m.add(
        "IdCeilingExceeded",
        py.get_type::<errors::IdCeilingExceeded>(),
    )?;
    m.add(
        "CaveatedOverwrite",
        py.get_type::<errors::CaveatedOverwrite>(),
    )?;

    caveats::register(m)?;
    collections::register(m)?;
    results::register(m)?;

    m.add_class::<Document>()?;
    m.add_function(wrap_pyfunction!(document::load, m)?)?;
    m.add_function(wrap_pyfunction!(document::new_document, m)?)?;

    Ok(())
}
