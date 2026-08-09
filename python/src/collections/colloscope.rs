//! The colloscope of a document
//!
//! Reached as `doc.colloscope` (§3.14 of `docs/python/handle_api.md`): the
//! result of a resolution, stored in two sparse tables. The interrogation
//! table says which group numbers sit in which `(slot, week)` cell — numbers,
//! not students, because a group number names a group of the list the cell's
//! subject uses on that week's period. The placements table says how an
//! automatic group list was filled: each student, and the group they landed
//! in. A prefilled list never appears there — its groups are `gl.groups`.
//!
//! Both tables read as they are stored: only the non-empty cells exist, so an
//! absent cell is one thing, `None` — not a `KeyError`, and not an empty set.
//! Whether a cell *could* hold anything is `doc.is_interrogation_possible`'s
//! question, so the two reads pair.

use std::collections::{BTreeMap, BTreeSet};

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyFrozenSet};

use collomatique_state_colloscopes::GroupListId as RawGroupListId;
use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::SlotId as RawSlotId;
use collomatique_state_colloscopes::StudentId as RawStudentId;
use collomatique_state_colloscopes::WeekId as RawWeekId;

use crate::Document;
use crate::collections::group_lists::GroupList;
use crate::collections::slots::Slot;
use crate::collections::students::Student;
use crate::collections::weeks::Week;
use crate::handles::{Handle, argument};

/// The colloscope of one document
///
/// Frozen and holding nothing but the document: it is a view, so two of them
/// on the same document are interchangeable and neither can go stale.
///
/// A singleton view, and not a collection — the colloscope has no `len`, no
/// `in` and no `.get`: its two tables are sparse, so a row count is a
/// statement about the model's storage rather than about the data, and its
/// reads take coordinates, not ids.
#[pyclass(module = "collomatique", frozen)]
pub struct Colloscope {
    doc: Py<Document>,
}

impl Colloscope {
    /// Builds the view — `doc.colloscope` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> Colloscope {
        Colloscope { doc }
    }

    /// Reads the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }

    /// The stored interrogation cells, as a snapshot of their contents (§2.5)
    ///
    /// The single definition of what iteration yields: `colloscope.iter()` in
    /// the model's key order. Collected once when the iteration starts, so a
    /// removal made in the middle is safe and loud — the ids stand, and the
    /// handles minted for a dead entity raise on the first read.
    fn cells(&self, py: Python<'_>) -> Vec<((RawSlotId, RawWeekId), BTreeSet<u32>)> {
        self.with_data(py, |data| {
            data.colloscope
                .iter()
                .map(|((slot, week), groups)| ((slot, week), groups.clone()))
                .collect()
        })
    }

    /// The stored placements rows, as a snapshot of their contents (§2.5)
    fn placements(&self, py: Python<'_>) -> Vec<(RawGroupListId, BTreeMap<RawStudentId, u32>)> {
        self.with_data(py, |data| {
            data.colloscope
                .group_lists_iter()
                .map(|(group_list, placements)| (group_list, placements.clone()))
                .collect()
        })
    }
}

#[pymethods]
impl Colloscope {
    /// The group numbers assigned to a `(slot, week)` cell, or `None`
    ///
    /// ```python
    /// groups = doc.colloscope.interrogation(slot, week)
    /// if groups is None:
    ///     continue
    /// ```
    ///
    /// The numbers are indices into the group list the cell's subject uses on
    /// that week's period — the `(period, subject) → group list` hop is
    /// `doc.group_lists.association_for`, and the names are
    /// `gl.group_name(number)`. `None` is the single absent answer: nothing is
    /// scheduled there, which is not the same thing as « that cell is
    /// impossible ». Pair the read with `doc.is_interrogation_possible` to
    /// know whether a cell *could* hold anything.
    ///
    /// Both arguments take a handle or an id. A `slot` or a `week` this
    /// document does not hold raises `StaleHandleError` rather than answering:
    /// the cell was malformed before it had an answer (§2.4's argument
    /// convention).
    fn interrogation<'py>(
        &self,
        py: Python<'py>,
        slot: &Bound<'py, PyAny>,
        week: &Bound<'py, PyAny>,
    ) -> PyResult<Option<Bound<'py, PyFrozenSet>>> {
        let slot_id = argument::<Slot>(&self.doc, slot)?;
        let week_id = argument::<Week>(&self.doc, week)?;

        let groups = self.with_data(py, |data| {
            data.colloscope.interrogation(slot_id, week_id).cloned()
        });
        groups
            .map(|groups| PyFrozenSet::new(py, groups))
            .transpose()
    }

    /// The stored interrogation cells, as `(Slot, Week, frozenset)` triples,
    /// in key order
    ///
    /// Yields only the cells the model stores — the non-empty ones. The empty
    /// cell is not something to iterate, it is something to read:
    /// `interrogation` answers `None` there.
    fn interrogations(&self, py: Python<'_>) -> ColloscopeInterrogationIter {
        ColloscopeInterrogationIter::new(self.doc.clone_ref(py), self.cells(py))
    }

    /// How an automatic group list was filled, or `None`
    ///
    /// The placements the solver chose for a list, as a read-only mapping of
    /// [Student] to group number. The mapping is a `types.MappingProxyType`
    /// over a fresh dict: reading it is reading the document, mutating it is
    /// `TypeError`.
    ///
    /// `None` — not an empty mapping — when the document holds no placements
    /// for that list. A prefilled list answers `None` too, in the same breath:
    /// its groups are `gl.groups`, and the model never fills it here. A `gl`
    /// this document does not hold raises `StaleHandleError`, like every
    /// argument of this api.
    fn group_list<'py>(
        &self,
        py: Python<'py>,
        group_list: &Bound<'py, PyAny>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        let group_list_id = argument::<GroupList>(&self.doc, group_list)?;

        let placements = self.with_data(py, |data| {
            data.colloscope.group_list(group_list_id).cloned()
        });
        placements
            .map(|placements| placements_mapping(py, &self.doc, placements))
            .transpose()
    }

    /// The stored placements, as `(GroupList, mapping)` pairs, in key order
    ///
    /// Yields only the rows the model stores — the automatic lists that were
    /// filled. The unplaced list is not something to iterate, it is something
    /// to read: `group_list` answers `None` there.
    fn group_lists(&self, py: Python<'_>) -> ColloscopeGroupListIter {
        ColloscopeGroupListIter::new(self.doc.clone_ref(py), self.placements(py))
    }

    /// The view itself — `<collomatique.Colloscope>`
    ///
    /// Deliberately without a row count: the view has two tables, and a repr
    /// that counted one of them would be describing half the colloscope.
    fn __repr__(&self) -> String {
        "<collomatique.Colloscope>".to_owned()
    }
}

/// The placements of one group list, as a read-only mapping of live handles
///
/// A fresh dict keyed by the [Student] handles the model placed, wrapped in
/// `types.MappingProxyType` — the read-only mapping of §2.5
/// (`docs/python/handle_api.md`): the proxy cannot be written through, and
/// the dict under it is unreachable, so there is nothing to mutate by
/// accident. The handles in it stay live.
fn placements_mapping<'py>(
    py: Python<'py>,
    doc: &Py<Document>,
    placements: BTreeMap<RawStudentId, u32>,
) -> PyResult<Bound<'py, PyAny>> {
    let dict = PyDict::new(py);
    for (student, group) in placements {
        dict.set_item(Student::mint(doc.clone_ref(py), student), group)?;
    }
    py.import("types")?
        .getattr("MappingProxyType")?
        .call1((dict,))
}

/// The interrogation cells of a colloscope, minted as the loop asks for them
///
/// A cell is a triple: the `Slot` and `Week` handles of the coordinates, and
/// the assigned group numbers as a frozenset of ints. The ids and the groups
/// were snapshotted when the iteration started (§2.5 of
/// `docs/python/handle_api.md`), so a removal in the middle leaves them
/// standing and the handles minted for a dead entity raise `StaleHandleError`
/// on the first read.
#[pyclass]
pub struct ColloscopeInterrogationIter {
    doc: Py<Document>,
    rows: std::vec::IntoIter<((RawSlotId, RawWeekId), BTreeSet<u32>)>,
}

impl ColloscopeInterrogationIter {
    /// Builds the iterator over an already-taken snapshot of the cells
    pub(crate) fn new(
        doc: Py<Document>,
        rows: Vec<((RawSlotId, RawWeekId), BTreeSet<u32>)>,
    ) -> ColloscopeInterrogationIter {
        ColloscopeInterrogationIter {
            doc,
            rows: rows.into_iter(),
        }
    }
}

#[pymethods]
impl ColloscopeInterrogationIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(
        &mut self,
        py: Python<'py>,
    ) -> PyResult<Option<(Slot, Week, Bound<'py, PyFrozenSet>)>> {
        let Some(((slot, week), groups)) = self.rows.next() else {
            return Ok(None);
        };
        Ok(Some((
            Slot::mint(self.doc.clone_ref(py), slot),
            Week::mint(self.doc.clone_ref(py), week),
            PyFrozenSet::new(py, groups)?,
        )))
    }
}

/// The placements rows of a colloscope, minted as the loop asks for them
///
/// A row is a pair: the `GroupList` handle that was filled, and the
/// placements as a read-only mapping of [Student] to group number. The ids
/// and the placements were snapshotted when the iteration started (§2.5), so
/// a removal in the middle leaves them standing and the handles minted for a
/// dead entity raise `StaleHandleError` on the first read.
#[pyclass]
pub struct ColloscopeGroupListIter {
    doc: Py<Document>,
    rows: std::vec::IntoIter<(RawGroupListId, BTreeMap<RawStudentId, u32>)>,
}

impl ColloscopeGroupListIter {
    /// Builds the iterator over an already-taken snapshot of the rows
    pub(crate) fn new(
        doc: Py<Document>,
        rows: Vec<(RawGroupListId, BTreeMap<RawStudentId, u32>)>,
    ) -> ColloscopeGroupListIter {
        ColloscopeGroupListIter {
            doc,
            rows: rows.into_iter(),
        }
    }
}

#[pymethods]
impl ColloscopeGroupListIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(
        &mut self,
        py: Python<'py>,
    ) -> PyResult<Option<(GroupList, Bound<'py, PyAny>)>> {
        let Some((group_list, placements)) = self.rows.next() else {
            return Ok(None);
        };
        Ok(Some((
            GroupList::mint(self.doc.clone_ref(py), group_list),
            placements_mapping(py, &self.doc, placements)?,
        )))
    }
}
