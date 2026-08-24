//! What a write hands back
//!
//! Every mutator returns an [OpResult] rather than `None`: a write can do more
//! than it was asked to — the cascade repairs whatever the change broke — and
//! `docs/python/new_api_design.md` §5 makes those repairs part of the answer
//! instead of leaving them silent, which is what the old api did.
//!
//! A write that creates something answers the [AddResult] subclass, which
//! carries the handle of what it made beside the same warnings. [created] is
//! the tail every creating mutator ends with.

use pyo3::PyClass;
use pyo3::prelude::*;
use pyo3::types::PyAny;

use collomatique_ops::UpdateOp;
use collomatique_state_colloscopes::NewId;

use crate::Document;
use crate::handles::{Handle, RawId};

/// What one write returned
///
/// `warnings` are the repairs the cascade applied beyond what the call itself
/// asked for, and they are the whole of it: a write that creates nothing hands
/// back nothing else. An empty list is the ordinary case, and a script that
/// does not care about them can ignore the whole object.
///
/// A write that *does* create something answers the [AddResult] subclass
/// instead, which carries the handle of what it made beside the same warnings.
/// Different answers are different types rather than one type with an id field
/// holding `None` — and `isinstance(r, OpResult)` holds for both, so a script
/// that only reads warnings treats them alike.
#[pyclass(module = "collomatique", frozen, subclass)]
pub struct OpResult {
    warnings: Vec<Py<Warning>>,
}

impl OpResult {
    /// Builds the result of a write, from the warnings it has already rendered
    pub(crate) fn new(warnings: Vec<Py<Warning>>) -> OpResult {
        OpResult { warnings }
    }

    /// The warnings as a repr writes them, for both classes' `__repr__`
    fn warnings_repr(&self) -> String {
        self.warnings
            .iter()
            .map(|warning| warning.get().__repr__())
            .collect::<Vec<_>>()
            .join(", ")
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
        format!("OpResult(warnings=[{}])", self.warnings_repr())
    }
}

/// What a write that created something returned
///
/// The same [OpResult] — `warnings` is there and means the same thing — with
/// the one thing a creating write has to say beside them: `created`, what it
/// made. A **handle**, not an id, for the reason §4 of the design gives for
/// every read that names an entity: a handle is strictly more useful, and the
/// id is one attribute away as `r.created.id`.
///
/// The handle is typed — a `doc.incompats.add(...)` answers an `Incompat` —
/// because it is the handle the collection would have handed out anyway. There
/// is no `AddResult` for a write that creates nothing: such a write answers a
/// plain [OpResult], which has no `created` at all rather than one holding
/// `None`.
#[pyclass(module = "collomatique", extends = OpResult, frozen)]
pub struct AddResult {
    created: Py<PyAny>,
}

#[pymethods]
impl AddResult {
    /// The handle of what the write created
    #[getter]
    fn created(&self, py: Python<'_>) -> Py<PyAny> {
        self.created.clone_ref(py)
    }

    fn __repr__(slf: &Bound<'_, AddResult>) -> String {
        let created = crate::handles::shown(slf.get().created.bind(slf.py()), "what it created");

        format!(
            "AddResult(created={created}, warnings=[{}])",
            slf.as_super().get().warnings_repr()
        )
    }
}

/// The answer of a write that created one entity of kind `H`
///
/// The whole of a creating mutator's tail: it takes the mutable borrow, applies
/// the op, mints the handle for the id the model issued, and puts the two
/// halves together. `op` is built by the caller, and so is `pick` — [NewId]
/// spans every creating op of the model, and the mutator is the one place that
/// knows which variant its own op answers.
///
/// An op that answers the wrong kind of id, or none, is this module's fault and
/// not the script's: the op and the handle class are both written down at the
/// call site, one line apart.
pub(crate) fn created<H>(
    py: Python<'_>,
    doc: &Py<Document>,
    op: UpdateOp,
    pick: impl FnOnce(NewId) -> Option<RawId<H>>,
) -> PyResult<Py<AddResult>>
where
    H: Handle + PyClass<BaseType = PyAny>,
{
    // The mutable borrow lasts no longer than this statement. Everything below
    // it builds python objects, and doing that under the document's borrow is
    // how a nested borrow becomes a `PanicException`.
    let (warnings, new_id) = doc.borrow_mut(py).write(py, op)?;

    let id = new_id.and_then(pick).ok_or_else(|| {
        bug(format!(
            "the write that was to add this {} answered no {}",
            H::NOUN,
            <H::IdClass as crate::ids::IdClass>::CLASS,
        ))
    })?;

    let created = Py::new(py, H::mint(doc.clone_ref(py), id))?.into_any();

    Py::new(py, (AddResult { created }, OpResult::new(warnings)))
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
/// about to remove (`colloscopes/ops/src/cascade.rs`).
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
    m.add_class::<AddResult>()?;
    m.add_class::<Warning>()?;
    Ok(())
}
