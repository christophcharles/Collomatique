//! The colloscope of a document
//!
//! Reached as `doc.colloscope`: the
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
//!
//! Written one row at a time, through `set_interrogation` and `set_group_list`,
//! and emptied wholesale through `erase` and `erase_group_lists`. The sparse
//! shape holds on the write exactly as it does on the read: an empty set of
//! groups and an empty placement mapping are the absent row, which is what
//! clears one — there is no `remove` here, because there is nothing to remove
//! that writing nothing does not already say.
//!
//! Nothing in the document points at a colloscope row, so no write of this
//! family can break anything the cascade would have to repair: every
//! `OpResult` here carries an empty `warnings`. The traffic runs the other way
//! — a group list, a slot or a subject changing is what trims the colloscope —
//! and those repairs come back on *their* families' results.
//!
//! Both writes measure what they are given against the document, and each
//! refusal reaches a script as `ColloscopeError`. A cell must name a
//! coordinate a colle can really sit on — the slot's subject must run on the
//! week's period, and the week must be active for the slot — and its group
//! numbers must fit the list that coordinate uses. A placement row belongs to
//! an automatic list, since a prefilled one holds its groups itself, and it
//! must name students the list does not exclude, in groups it really has. What
//! the model could otherwise object to is caught above the write, where the
//! message can say which argument was wrong: a dead group list, slot or week
//! is the argument convention's business ([crate::handles::argument]), and so
//! is a placed student the document does not hold.
//!
//! The family's fifth op, `install`, is the whole-colloscope door: it takes a
//! `ColloscopeData` and makes the document hold exactly its rows. It is the
//! solver's landing door — what a solve's outcome is put back through — and
//! the row-by-row writes stay for everything smaller.

use std::collections::{BTreeMap, BTreeSet};

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyFrozenSet};

use collomatique_ops::{ColloscopeContents, ColloscopeUpdateOp, UpdateOp};
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
use crate::data::ColloscopeData;
use crate::data::Value as _;
use crate::handles::{Handle, argument, shown};
use crate::ids::{IdClass, StudentId};
use crate::results::OpResult;

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

    /// The stored interrogation cells, as a snapshot of their contents
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

    /// The stored placements rows, as a snapshot of their contents
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
    /// the cell was malformed before it had an answer (the argument
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

    /// The whole colloscope, detached — a `ColloscopeData` holding the two
    /// sparse tables as the document stores them
    ///
    /// A fresh object every call. Nothing here can go stale: the view is
    /// bound to the document alone, so this never raises `StaleHandleError`.
    /// [Colloscope::install] takes one back whole; the row-by-row doors,
    /// [Colloscope::set_interrogation] and [Colloscope::set_group_list],
    /// remain for a single cell.
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Copied out of the borrow before anything python-facing happens:
        // building the value calls into python, and doing that under the
        // document's borrow is how a nested borrow becomes a `PanicException`.
        let contents = self.with_data(py, |data| ColloscopeContents::from(&data.colloscope));

        ColloscopeData::to_py(py, &contents)
    }

    /// Writes one `(slot, week)` cell of the interrogation table
    ///
    /// ```python
    /// doc.colloscope.set_interrogation(slot, week, {0, 2})
    /// doc.colloscope.set_interrogation(slot, week, set())
    /// ```
    ///
    /// The cell becomes exactly the group numbers it is given, whatever it held
    /// before: this is a write of the whole cell and not an addition to it. An
    /// empty set is the absent cell, the same `None` the read answers — so
    /// emptying a cell is how a cell is cleared, and there is no `remove` here.
    ///
    /// `groups` is any iterable of group numbers — a `frozenset`, the shape
    /// `interrogation` hands back, or a list a script built up. The numbers are
    /// indices into the group list the cell's subject uses on that week's
    /// period, so what fits is what that list has: `doc.group_lists` is where
    /// the names live, and a number past the end is refused.
    ///
    /// The slot and the week both take a handle or an id, and a dead one raises
    /// `StaleHandleError` — the coordinate was malformed before it named a
    /// cell. Three refusals are the model's, and each is a `ColloscopeError`:
    /// the slot's subject must run on the week's period, the week must be one
    /// the slot really runs on, and every group number must fit the bound.
    /// Together they are `doc.is_interrogation_possible`'s question, asked of a
    /// write: a cell that could hold nothing takes nothing.
    ///
    /// The colloscope is pointed at by nothing, so this repairs nothing:
    /// `warnings` is empty.
    fn set_interrogation(
        &self,
        py: Python<'_>,
        slot: &Bound<'_, PyAny>,
        week: &Bound<'_, PyAny>,
        groups: &Bound<'_, PyAny>,
    ) -> PyResult<OpResult> {
        let slot_id = argument::<Slot>(&self.doc, slot)?;
        let week_id = argument::<Week>(&self.doc, week)?;
        let groups = group_numbers(groups)?;

        self.write(
            py,
            UpdateOp::Colloscope(ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                slot_id, week_id, groups,
            )),
        )
    }

    /// Writes the placements row of one automatic group list
    ///
    /// ```python
    /// doc.colloscope.set_group_list(gl, {harry: 0, ron: 2})
    /// doc.colloscope.set_group_list(gl, {})
    /// ```
    ///
    /// The row becomes exactly the mapping it is given: a student the mapping
    /// leaves out is a student the list places nowhere. An empty mapping is the
    /// absent row, the same `None` the read answers, so it is how a row is
    /// cleared.
    ///
    /// The students are named as every entity is, by a handle or an id, and the
    /// group numbers are indices into the list's own groups. The list takes a
    /// handle or an id too, and a dead one — or a student this document does
    /// not hold — raises `StaleHandleError` rather than reaching the model.
    ///
    /// Three refusals are the model's, all `ColloscopeError`: a prefilled list
    /// holds its groups itself and has no row here, a student the list excludes
    /// cannot be placed in it, and a group number must be one the list really
    /// has.
    ///
    /// The colloscope is pointed at by nothing, so this repairs nothing:
    /// `warnings` is empty.
    fn set_group_list(
        &self,
        py: Python<'_>,
        group_list: &Bound<'_, PyAny>,
        placements: &Bound<'_, PyAny>,
    ) -> PyResult<OpResult> {
        let group_list_id = argument::<GroupList>(&self.doc, group_list)?;
        let placements = placement_row(&self.doc, placements)?;

        self.write(
            py,
            UpdateOp::Colloscope(ColloscopeUpdateOp::UpdateColloscopeGroupList(
                group_list_id,
                placements,
            )),
        )
    }

    /// Empties the interrogation table
    ///
    /// Every cell goes, and nothing else does: the placements stand, since how
    /// an automatic list was filled is not a colle. `erase_group_lists` is the
    /// other half, and calling both leaves the colloscope of a document that
    /// never had one.
    ///
    /// One operation, and so one undo slot, however many cells it emptied.
    /// Clearing only ever removes, and an absent row contradicts nothing, so
    /// `warnings` is empty.
    fn erase(&self, py: Python<'_>) -> PyResult<OpResult> {
        self.write(
            py,
            UpdateOp::Colloscope(ColloscopeUpdateOp::EraseColloscope),
        )
    }

    /// Empties the placements table
    ///
    /// Every automatic list loses the row saying how it was filled, and the
    /// colles stand: a cell names group *numbers*, and the numbers mean what
    /// the list says they mean whether or not anybody has been placed in it.
    ///
    /// One operation, and so one undo slot, however many rows it emptied.
    /// `warnings` is empty, for the reason `erase` gives.
    fn erase_group_lists(&self, py: Python<'_>) -> PyResult<OpResult> {
        self.write(
            py,
            UpdateOp::Colloscope(ColloscopeUpdateOp::EraseGroupLists),
        )
    }

    /// Replaces the whole colloscope
    ///
    /// ```python
    /// outcome = run.wait()
    /// doc.colloscope.install(outcome.colloscope)
    /// ```
    ///
    /// Afterwards the document holds exactly the value's rows and no others:
    /// a row the value does not name is gone, the way `erase` would have left
    /// it. One operation, and so one undo slot, however much changed — the op
    /// *carries* a whole colloscope but *lands* as a diff, so a row the
    /// document already holds costs nothing.
    ///
    /// The value is measured against the document the way the row-by-row
    /// writes are, and the refusals are the same ones: each is a
    /// `ColloscopeError` naming the offending row. The colloscope is pointed
    /// at by nothing, so this repairs nothing and `warnings` is empty.
    fn install(&self, py: Python<'_>, colloscope: &Bound<'_, PyAny>) -> PyResult<OpResult> {
        // Extracted before the borrow below and never inside it: reading the
        // value calls into python, and doing that under the document's borrow
        // is how a nested borrow becomes a `PanicException`.
        let contents = ColloscopeData::from_py(&self.doc, colloscope)?;

        self.write(
            py,
            UpdateOp::Colloscope(ColloscopeUpdateOp::InstallColloscope(contents)),
        )
    }

    /// The view itself — `<collomatique.Colloscope>`
    ///
    /// Deliberately without a row count: the view has two tables, and a repr
    /// that counted one of them would be describing half the colloscope.
    fn __repr__(&self) -> String {
        "<collomatique.Colloscope>".to_owned()
    }
}

impl Colloscope {
    /// Writes through the document the view came from
    ///
    /// The five mutators end here: none of them creates anything, so none of
    /// them needs the id half [crate::results::created] keeps.
    fn write(&self, py: Python<'_>, op: UpdateOp) -> PyResult<OpResult> {
        let mut doc = self.doc.borrow_mut(py);
        doc.update(py, op)
    }
}

/// The group numbers one cell is being given
///
/// Any iterable of them: the read hands back a `frozenset`, and a script that
/// built a list is saying the same thing. Reading it into a set is what makes
/// the two the same — the model stores a set, so a number written twice is
/// written once.
fn group_numbers(obj: &Bound<'_, PyAny>) -> PyResult<BTreeSet<u32>> {
    let items = obj.try_iter().map_err(|_| {
        PyTypeError::new_err(format!(
            "the groups of a cell are an iterable of group numbers, and {} is not one",
            shown(obj, "that value"),
        ))
    })?;

    let mut groups = BTreeSet::new();
    for item in items {
        let item = item?;
        let group: u32 = item.extract().map_err(|_| {
            PyTypeError::new_err(format!(
                "the groups of a cell are group numbers, and {} is not one",
                shown(&item, "that value"),
            ))
        })?;
        groups.insert(group);
    }

    Ok(groups)
}

/// The placements one group list is being given
///
/// A mapping of students to group numbers, read the way the value boundary
/// reads the same table (`crate::data`): every student is a handle or an id,
/// resolved against this document, so a foreign handle and a dead id are
/// refused here rather than by the model. A student named twice — once by
/// handle and once by id, the only way a mapping can hold them both — is a
/// call that says two things at once, and it is refused rather than settled by
/// iteration order.
fn placement_row(
    doc: &Py<Document>,
    obj: &Bound<'_, PyAny>,
) -> PyResult<BTreeMap<RawStudentId, u32>> {
    let items = obj.call_method0("items").map_err(|_| {
        PyTypeError::new_err(format!(
            "the placements of a group list are a mapping of students to group numbers, and \
             {} is not one",
            shown(obj, "that value"),
        ))
    })?;

    let mut placements = BTreeMap::new();
    for entry in items.try_iter()? {
        let entry = entry?;
        let (student, group): (Bound<'_, PyAny>, Bound<'_, PyAny>) =
            entry.extract().map_err(|_| {
                PyTypeError::new_err(format!(
                    "the placements of a group list are pairs of a student and a group \
                     number, and {} is not one",
                    shown(&entry, "that pair"),
                ))
            })?;
        let group: u32 = group.extract().map_err(|_| {
            PyTypeError::new_err(format!(
                "the placements of a group list hold group numbers, and {} is not one",
                shown(&group, "that value"),
            ))
        })?;

        let student = argument::<Student>(doc, &student)?;
        if placements.insert(student, group).is_some() {
            return Err(PyValueError::new_err(format!(
                "the placements of a group list name {} twice",
                StudentId::text(student),
            )));
        }
    }

    Ok(placements)
}

/// The placements of one group list, as a read-only mapping of live handles
///
/// A fresh dict keyed by the [Student] handles the model placed, wrapped in
/// `types.MappingProxyType` — a read-only mapping: the proxy cannot be written
/// through, and
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
/// were snapshotted when the iteration started, so a removal in the middle
/// leaves them
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
/// and the placements were snapshotted when the iteration started, so
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
