//! Group-list generation, as a script drives it
//!
//! Three steps, and the document is touched only by the last of them:
//! `doc.default_generation_request()` hands over a selection to edit,
//! `doc.generate_group_lists(request)` turns it into the lists it describes,
//! and `doc.group_lists.add_generated(result.entries)` lands them as one undo
//! slot. In between there is nothing but a value: a generation that is never
//! landed changes nothing, and one that is landed twice adds the lists twice.
//!
//! It is the application's own generation, not a second one written for
//! scripts. The default selection comes from the function the generate dialog
//! itself calls, the names are the coverage labels its naming dialog seeds its
//! rows with, and the generator in between is the same
//! [collomatique_greedy_groups] the application runs — so a script and a click
//! produce the same lists out of the same document.
//!
//! What crosses the boundary here holds **ids**, never handles: the entries
//! pair a `GroupListData` — a detached value already carrying student ids —
//! with the `(period, subject)` coordinates it must serve. A value that was
//! half detached and half holding the document is the one shape the boundary
//! refuses.

use std::collections::BTreeSet;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyFrozenSet, PyList};

use collomatique_state_colloscopes::PeriodId as RawPeriodId;
use collomatique_state_colloscopes::SubjectId as RawSubjectId;
use collomatique_state_colloscopes::group_lists::GroupList as RawGroupList;

use crate::Document;
use crate::collections::{Period, Subject};
use crate::data::{GroupListData, Value as _};
use crate::handles::{argument, shown};
use crate::ids::{IdClass as _, PeriodId, SubjectId};

/// What one generation produced
///
/// ```python
/// result = doc.generate_group_lists(doc.default_generation_request())
/// doc.group_lists.add_generated(result.entries)
/// ```
///
/// The lists themselves, ready to land, and the pairs the generation left
/// alone because nobody is registered for them. Nothing here is attached to
/// the document: this is what a generation *would* write, and until
/// `add_generated` is called it has written nothing.
#[pyclass(module = "collomatique", frozen)]
pub struct GroupListsGenerationResult {
    /// The entries, built once when the result was
    ///
    /// Handed out by reference afterwards, like `SolveOutcome.colloscope`: a
    /// script that renames a list by editing a `GroupListData` in place is
    /// editing the object the next `.entries` hands back too, which is what
    /// makes the rename-then-land flow work at all.
    entries: Py<PyList>,
    /// The skipped pairs, built once alongside the entries
    skipped: Py<PyFrozenSet>,
    /// The two numbers the repr shows, kept so it needs no python
    counts: (usize, usize),
}

#[pymethods]
impl GroupListsGenerationResult {
    /// The generated lists, ready to land
    ///
    /// A list of `(GroupListData, frozenset of (PeriodId, SubjectId))` pairs
    /// — each list and the coordinates it must be associated to. It is
    /// exactly what `doc.group_lists.add_generated` takes, so landing a
    /// generation whole is one call.
    ///
    /// There is one entry per *distinct* list, not one per requested pair:
    /// pairs whose students and group-size range agree share a list, and its
    /// entry then carries both their coordinates.
    ///
    /// The names are the labels the application's own naming dialog starts
    /// from — « Sortilèges (période 1) », « Sortilèges et Métamorphose
    /// (périodes 1 et 2) ». Renaming is editing `.name` on these values
    /// before landing them; the objects are the result's own, so the edit
    /// stays.
    #[getter]
    fn entries(&self, py: Python<'_>) -> Py<PyList> {
        self.entries.clone_ref(py)
    }

    /// The requested pairs no list was built for
    ///
    /// A frozenset of `(PeriodId, SubjectId)` pairs: the ones nobody is
    /// registered for. Not a refusal — there is no list to build for a
    /// subject no student takes that period — and the application's own
    /// dialog reports the same set as a remark. A request that asks for
    /// nothing else produces an empty `entries` and no error.
    #[getter]
    fn skipped(&self, py: Python<'_>) -> Py<PyFrozenSet> {
        self.skipped.clone_ref(py)
    }

    fn __repr__(&self) -> String {
        format!(
            "<collomatique.GroupListsGenerationResult lists={} skipped={}>",
            self.counts.0, self.counts.1
        )
    }
}

/// The result for what the generator produced
///
/// Every value is built here, once, while the entries are still in hand: the
/// getters afterwards only hand out references to these objects.
pub(crate) fn build(
    py: Python<'_>,
    entries: Vec<(RawGroupList, BTreeSet<(RawPeriodId, RawSubjectId)>)>,
    skipped: &BTreeSet<(RawPeriodId, RawSubjectId)>,
) -> PyResult<Py<GroupListsGenerationResult>> {
    let counts = (entries.len(), skipped.len());

    let built: Vec<Bound<'_, PyAny>> = entries
        .iter()
        .map(|(group_list, covered)| {
            let value = GroupListData::to_py(py, group_list)?;
            let coverage = coverage_to_py(py, covered)?;
            Ok((value, coverage).into_pyobject(py)?.into_any())
        })
        .collect::<PyResult<_>>()?;

    Py::new(
        py,
        GroupListsGenerationResult {
            entries: PyList::new(py, built)?.unbind(),
            skipped: coverage_to_py(py, skipped)?.unbind(),
            counts,
        },
    )
}

/// A set of (period, subject) coordinates, as python holds one
///
/// A frozenset of id pairs. Frozen, where a request's `rebuild` is an ordinary
/// set: this one is something a generation reports, not something a script
/// fills in.
fn coverage_to_py<'py>(
    py: Python<'py>,
    pairs: &BTreeSet<(RawPeriodId, RawSubjectId)>,
) -> PyResult<Bound<'py, PyFrozenSet>> {
    PyFrozenSet::new(
        py,
        pairs
            .iter()
            .map(|&(period, subject)| (PeriodId::wrap(period), SubjectId::wrap(subject))),
    )
}

/// The entries one python value names, for [crate::collections::GroupLists]
///
/// Read structurally rather than by class: what `add_generated` takes is the
/// *shape* `result.entries` has, so a script that builds the same pairs by
/// hand — or lands a subset of a result — is served by the same door. Every
/// list goes through `GroupListData`'s own extraction, which is where a
/// student the document does not hold and a filling the model seals against
/// are refused, and every coordinate through the argument convention, which
/// is where a dead period or subject is.
pub(crate) fn entries_from_py(
    doc: &Py<Document>,
    obj: &Bound<'_, PyAny>,
) -> PyResult<Vec<(RawGroupList, BTreeSet<(RawPeriodId, RawSubjectId)>)>> {
    let items = obj.try_iter().map_err(|_| {
        PyTypeError::new_err(format!(
            "add_generated takes a list of (GroupListData, coverage) pairs, and {} cannot be \
             iterated over",
            shown(obj, "that object"),
        ))
    })?;

    items
        .map(|item| {
            let item = item?;
            let (value, coverage): (Bound<'_, PyAny>, Bound<'_, PyAny>) =
                item.extract().map_err(|_| {
                    PyTypeError::new_err(format!(
                        "add_generated holds pairs of a GroupListData and its (period, subject) \
                         coverage, and {} is not one",
                        shown(&item, "that pair"),
                    ))
                })?;

            let group_list = GroupListData::from_py(doc, &value)?;
            Ok((group_list, coverage_from_py(doc, &coverage)?))
        })
        .collect()
}

/// The coordinates one entry of [entries_from_py] must be associated to
fn coverage_from_py(
    doc: &Py<Document>,
    obj: &Bound<'_, PyAny>,
) -> PyResult<BTreeSet<(RawPeriodId, RawSubjectId)>> {
    let items = obj.try_iter().map_err(|_| {
        PyTypeError::new_err(format!(
            "an entry's coverage is a set of (period, subject) pairs, and {} cannot be iterated \
             over",
            shown(obj, "that value"),
        ))
    })?;

    items
        .map(|item| {
            let item = item?;
            let (period, subject): (Bound<'_, PyAny>, Bound<'_, PyAny>) =
                item.extract().map_err(|_| {
                    PyTypeError::new_err(format!(
                        "an entry's coverage holds (period, subject) pairs, and {} is not one",
                        shown(&item, "that pair"),
                    ))
                })?;

            Ok((
                argument::<Period>(doc, &period)?,
                argument::<Subject>(doc, &subject)?,
            ))
        })
        .collect()
}

/// Adds the generation classes to the module
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<GroupListsGenerationResult>()
}
