//! The `collomatique` python module
//!
//! This crate is the new scripting API described in
//! `docs/python/new_api_design.md`. Running an interpreter is
//! `collomatique-python-runner`'s job; this crate only defines what the module
//! contains.

use pyo3::prelude::*;

/// The `collomatique` python module, for registration in an interpreter's inittab.
#[pymodule]
pub fn collomatique(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add(
        "__version__",
        collomatique_settings::current_version().to_string(),
    )?;
    Ok(())
}
