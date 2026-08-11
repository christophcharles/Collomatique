//! What a write hands back
//!
//! Every mutator returns an [OpResult] rather than `None`: a write can do more
//! than it was asked to — the cascade repairs whatever the change broke — and
//! `docs/python/new_api_design.md` §5 makes those repairs part of the answer
//! instead of leaving them silent, which is what the old api did.

use pyo3::prelude::*;

/// What one write returned
///
/// `warnings` are the repairs the cascade applied beyond what the call itself
/// asked for. An empty list is the ordinary case, and a script that does not
/// care about them can ignore the whole object.
#[pyclass(module = "collomatique", frozen)]
pub struct OpResult {
    warnings: Vec<Py<Warning>>,
}

impl OpResult {
    /// Builds the result of a write, from the warnings it has already rendered
    pub(crate) fn new(warnings: Vec<Py<Warning>>) -> OpResult {
        OpResult { warnings }
    }
}

#[pymethods]
impl OpResult {
    /// The id the write created, or `None` when it created nothing
    ///
    /// Always `None` today: the only writes that exist are the two first-week
    /// ones, and neither creates anything. It is here from the start so that
    /// the shape of a result does not change under scripts when the write
    /// surface brings creating ops with it.
    #[getter]
    fn new_id(&self) -> Option<Py<PyAny>> {
        None
    }

    /// The repairs the write had to apply, in the order it applied them
    #[getter]
    fn warnings(&self, py: Python<'_>) -> Vec<Py<Warning>> {
        self.warnings
            .iter()
            .map(|warning| warning.clone_ref(py))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "OpResult(new_id=None, warnings=[{}])",
            self.warnings
                .iter()
                .map(|warning| warning.get().__repr__())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

/// One repair the cascade applied
///
/// `str(w)` is the french sentence the gui shows in its confirmation dialog,
/// rendered against the document as it was *before* the write — that is where
/// the entities it names are still to be found.
///
/// The structured `Fix` payload `docs/python/new_api_design.md` §5 describes
/// lands with the write surface, where the repairs a script can act on
/// actually happen. A rendered sentence is all the first-week ops can produce.
#[pyclass(module = "collomatique", frozen, eq, hash)]
#[derive(PartialEq, Eq, Hash)]
pub struct Warning {
    text: String,
}

impl Warning {
    pub(crate) fn new(text: String) -> Warning {
        Warning { text }
    }
}

#[pymethods]
impl Warning {
    fn __str__(&self) -> &str {
        &self.text
    }

    fn __repr__(&self) -> String {
        format!("Warning({:?})", self.text)
    }
}

/// Adds the result classes to the module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<OpResult>()?;
    m.add_class::<Warning>()?;
    Ok(())
}
