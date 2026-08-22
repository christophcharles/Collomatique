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
/// asked for, and they are the whole of it: a write that creates nothing hands
/// back nothing else. An empty list is the ordinary case, and a script that
/// does not care about them can ignore the whole object.
///
/// A write that *does* create something answers the `AddResult` subclass
/// instead, which carries the handle of what it made beside the same warnings
/// (`docs/python/ops_migration.md`). It lands with the first creating op.
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
            "OpResult(warnings=[{}])",
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
/// the entities it names are still to be found. Beside it, `kind` and `details`
/// are the same repair as structured data: the model's own name for it, and its
/// coordinates, walked off `Fix` the way a refusal's are walked off
/// `UpdateError` ([crate::payload]). The coordinates are **ids**, never handles:
/// a warning names material the write may just have removed, and a handle to
/// that would be born dead.
///
/// `parent` is the repair that needed this one, `None` for one the write asked
/// for directly. A repair lands before the one that needed it, so a parent is
/// always further down the list [OpResult] hands back — the list is a tree, and
/// this is the link that says so.
///
/// A warning is not hashable: what it carries is a dict, and python does not
/// hash those either.
#[pyclass(module = "collomatique", frozen)]
pub struct Warning {
    kind: Option<String>,
    details: Py<PyAny>,
    text: String,
    parent: Option<Py<Warning>>,
}

#[pymethods]
impl Warning {
    /// The model's own name for this repair — `"DeleteSlot"`
    ///
    /// `None` only for a repair shaped in a way the structural walk cannot
    /// follow, which no variant is today.
    #[getter]
    fn kind(&self) -> Option<&str> {
        self.kind.as_deref()
    }

    /// What the repair names, keyed by the model's own field names
    #[getter]
    fn details(&self, py: Python<'_>) -> Py<PyAny> {
        self.details.clone_ref(py)
    }

    /// The repair that needed this one, or `None` when the write asked for it
    #[getter]
    fn parent(&self, py: Python<'_>) -> Option<Py<Warning>> {
        self.parent.as_ref().map(|warning| warning.clone_ref(py))
    }

    fn __str__(&self) -> &str {
        &self.text
    }

    fn __repr__(&self) -> String {
        format!("Warning({:?}, {:?})", self.kind, self.text)
    }

    /// Two warnings are equal when they say the same thing about the same
    /// repair, under the same parent
    fn __eq__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        let Ok(other) = other.cast::<Warning>() else {
            return Ok(false);
        };
        let other = other.get();

        let same_parent = match (&self.parent, &other.parent) {
            (None, None) => true,
            // Terminates: a parent is strictly further down a finite list.
            (Some(mine), Some(theirs)) => mine.get().__eq__(py, theirs.bind(py).as_any())?,
            _ => false,
        };

        Ok(same_parent
            && self.kind == other.kind
            && self.text == other.text
            && self.details.bind(py).eq(other.details.bind(py))?)
    }
}

/// The warnings one cascade produced, rendered and linked
///
/// `data` must be the **pre-state**: a warning names material the update may be
/// about to remove (`ops/src/cascade.rs`).
///
/// Built back to front. A repair's parent is always a *later* entry of the list
/// — children land before the repair that needed them — so walking in reverse
/// means the parent object exists by the time its child asks for it, and no
/// second pass has to mutate a frozen class.
pub(crate) fn from_cascade(
    py: Python<'_>,
    warnings: &[collomatique_ops::CascadeWarning],
    data: &collomatique_state_colloscopes::Data,
) -> PyResult<Vec<Py<Warning>>> {
    let mut built: Vec<Option<Py<Warning>>> = (0..warnings.len()).map(|_| None).collect();

    for (index, warning) in warnings.iter().enumerate().rev() {
        let text = warning.text(data).map_err(|e| {
            bug(format!(
                "a repair named something the document does not hold ({e})"
            ))
        })?;
        let (kind, details) = crate::payload::repair(py, warning.fix())
            .map_err(|e| bug(format!("a repair could not be read ({e})")))?;

        let parent = match warning.parent() {
            None => None,
            Some(parent) => Some(
                built
                    .get(parent)
                    .and_then(|warning| warning.as_ref())
                    .ok_or_else(|| {
                        bug(format!(
                            "repair {index} claims parent {parent}, which is not a later \
                             entry of the same list"
                        ))
                    })?
                    .clone_ref(py),
            ),
        };

        built[index] = Some(Py::new(
            py,
            Warning {
                kind,
                details: details.unbind(),
                text,
                parent,
            },
        )?);
    }

    Ok(built
        .into_iter()
        .map(|warning| warning.expect("the walk fills every index"))
        .collect())
}

/// A failure that is this module's own fault, and not the script's
fn bug(what: String) -> PyErr {
    crate::errors::Error::new_err(format!(
        "{what}; this is a bug in collomatique, not something the script did"
    ))
}

/// Adds the result classes to the module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<OpResult>()?;
    m.add_class::<Warning>()?;
    Ok(())
}
