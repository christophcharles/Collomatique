//! Running a solve, as a script drives it
//!
//! §13 of `docs/python/new_api_design.md` is the design. What lives here is
//! everything about *running* a solve rather than about the document a solve
//! recomputes: for now, the two presets the strategy's classmethods answer.
//!
//! The strategy value itself is in `data.rs`, with the other value classes —
//! it is written in python, like all of them, and this module only says what
//! the application's own two look like.

use pyo3::prelude::*;

use collomatique_strategies::ConductorStrategy as RawConductorStrategy;

/// The « Recherche simple » preset, as the application builds it
///
/// The door `ConductorStrategy.search()` goes through, private to the module
/// because a script has the classmethod. A preset written out in `data.py`
/// would be a copy of the application's, and a copy drifts; this hands back
/// the very structure the application's dialog opens with, converted.
#[pyfunction]
fn _conductor_search(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    crate::data::ConductorStrategy::to_py(py, &RawConductorStrategy::default())
}

/// The « Optimisation complète » preset, sized to this machine
///
/// The same door, for `ConductorStrategy.optimize()`. This one could not be
/// written out in python at all: its worker count is read from the cores the
/// machine reports, so the number is not known until it is asked for.
#[pyfunction]
fn _conductor_optimize(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    crate::data::ConductorStrategy::to_py(py, &RawConductorStrategy::with_parallelism_defaults())
}

/// Puts what a script drives a solve with in the module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(_conductor_search, m)?)?;
    m.add_function(wrap_pyfunction!(_conductor_optimize, m)?)?;

    Ok(())
}
