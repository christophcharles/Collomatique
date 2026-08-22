//! The junction table: which students take which subject in which period
//!
//! Reached as `doc.assignments`. The model stores assignments as a sparse
//! table keyed by `(period, subject)`: a row exists exactly when at least one
//! student is assigned, and the canonical form is invisible from python — an
//! absent row *is* the empty frozenset.
//!
//! Because the reads are total, this is not a mapping: there is no `len`, no
//! `in`, no `.get` — row count and row membership are statements about the
//! model's storage, not about the data. And the address arguments follow the
//! *argument* convention: a `period` or `subject` this document does
//! not hold raises `StaleHandleError`, because `KeyError` could never mean
//! "no row" — the only failure a read can have is a bad address, and a bad
//! address is a stale reference.
//!
//! Whether a subject *runs* in a period is not this table's question —
//! `subject.excluded_periods` answers it.
//!
//! Written through `set`, `set_all` and `duplicate_previous_period`. There is
//! no value class here and no `add`: a row is nothing but the three ids it is
//! made of, so the whole family is argument-convention wiring, and none of it
//! creates an entity. It never removes one either — an emptied row is stored
//! as no row at all, but that is the model's canonical form and not a
//! removal — so no handle ever goes stale of a write made here.
//!
//! Nothing in the document points *at* a row: a row names a period, a subject
//! and its students, and nothing names it back. So no write of this family
//! ever gives the cascade anything to repair, and every `OpResult` it hands
//! back carries an empty `warnings`. What a colloscope holds is untouched too
//! — the model relates a placement to the group list it is in, never to the
//! assignment row of the same coordinates.
//!
//! The family keeps three refusals for the model, and they all reach a script
//! as `AssignmentsError`: a subject that does not run on a period holds
//! nobody there (`set` and `set_all`), a student who takes no part in a period
//! cannot be assigned in it (`set`), and the first period has no previous one
//! to copy from (`duplicate_previous_period`). All three are statements about
//! the document rather than about an argument's shape — which is what tells
//! them from the argument convention, where a period, a subject or a student
//! this document does not hold is caught before the op is even built, and
//! where the message can say which argument was wrong.

use std::collections::BTreeSet;

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyFrozenSet, PyTuple};

use collomatique_ops::{AssignmentsUpdateOp, UpdateOp};
use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::PeriodId as RawPeriodId;
use collomatique_state_colloscopes::StudentId as RawStudentId;
use collomatique_state_colloscopes::SubjectId as RawSubjectId;

use crate::Document;
use crate::collections::periods::Period;
use crate::collections::students::Student;
use crate::collections::subjects::Subject;
use crate::handles::{Handle, argument, shown};
use crate::results::OpResult;

/// The assignments of one document
///
/// Frozen and holding nothing but the document: it is a view, so two of them
/// on the same document are interchangeable and neither can go stale.
#[pyclass(module = "collomatique", frozen)]
pub struct Assignments {
    doc: Py<Document>,
}

impl Assignments {
    /// Builds the view — `doc.assignments` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> Assignments {
        Assignments { doc }
    }

    /// Reads the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }

    /// The stored rows, as a snapshot of their ids
    ///
    /// The single definition of what iteration yields: `params.assignments.iter()`
    /// in the model's key order. Collected once when the iteration starts, so a
    /// removal made in the middle is safe and loud — the ids stand, and the
    /// handles minted for a dead entity raise on the first read.
    fn rows(&self, py: Python<'_>) -> Vec<(RawPeriodId, RawSubjectId, BTreeSet<RawStudentId>)> {
        self.with_data(py, |data| {
            data.params
                .assignments
                .iter()
                .map(|(period, subject, students)| (period, subject, students.clone()))
                .collect()
        })
    }
}

#[pymethods]
impl Assignments {
    /// The students taking this subject in this period, as a frozenset
    ///
    /// The address is a `(period, subject)` pair, each a handle or an id —
    /// `doc.assignments[p, s]` and `doc.assignments[p.id, s.id]` answer the
    /// same. The read is total over valid addresses: a pair the model stores
    /// no row for reads as the empty frozenset, never a `KeyError` — an
    /// absent row *is* nobody assigned.
    ///
    /// A `period` or `subject` this document does not hold raises
    /// `StaleHandleError` rather than answering, because the address was
    /// malformed before it had an answer (the argument convention, reached
    /// through the indexing spelling — the one deliberate wrinkle of that
    /// convention).
    fn __getitem__<'py>(
        &self,
        py: Python<'py>,
        key: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyFrozenSet>> {
        let (period, subject) = Address::of(key)?;
        let period_id = argument::<Period>(&self.doc, &period)?;
        let subject_id = argument::<Subject>(&self.doc, &subject)?;

        let student_ids = self.with_data(py, |data| {
            data.params
                .assignments
                .students(period_id, subject_id)
                .cloned()
                .unwrap_or_default()
        });
        let students: Vec<Bound<'py, PyAny>> = student_ids
            .into_iter()
            .map(|id| {
                Py::new(py, Student::mint(self.doc.clone_ref(py), id))
                    .map(|student| student.into_bound(py).into_any())
            })
            .collect::<PyResult<_>>()?;
        PyFrozenSet::new(py, &students)
    }

    /// The stored rows, as `(Period, Subject, frozenset)` triples, in key order
    ///
    /// Yields only the rows the model stores — the non-empty ones. The empty
    /// frozenset is not something to iterate, it is something to read.
    fn __iter__(&self, py: Python<'_>) -> AssignmentIter {
        AssignmentIter::new(self.doc.clone_ref(py), self.rows(py))
    }

    /// Assigns one student to one subject for one period, or takes them off it
    ///
    /// `assigned` says what the row must hold afterwards rather than toggling
    /// anything: `True` for a student who is already assigned is accepted and
    /// changes nothing, and so is `False` for one who was never there. The
    /// three addresses are handles or ids, interchangeably, as every argument
    /// of this api is.
    ///
    /// ```python
    /// doc.assignments.set(period, maths, student, True)
    /// ```
    ///
    /// The row is addressed here in the order the read above uses — the
    /// `(period, subject)` key, then the student inside it — so that the same
    /// row is named the same way whether it is being read or written.
    ///
    /// Two things the model refuses, both as `AssignmentsError`: a subject
    /// that does not run on the period holds nobody there, and a student who
    /// takes no part in the period cannot be assigned in it. Neither is about
    /// the arguments' shape — one this document does not hold raises
    /// `StaleHandleError` before the op is built.
    fn set(
        &self,
        py: Python<'_>,
        period: &Bound<'_, PyAny>,
        subject: &Bound<'_, PyAny>,
        student: &Bound<'_, PyAny>,
        assigned: bool,
    ) -> PyResult<OpResult> {
        let period_id = argument::<Period>(&self.doc, period)?;
        let subject_id = argument::<Subject>(&self.doc, subject)?;
        let student_id = argument::<Student>(&self.doc, student)?;

        // The op spells its three ids `(period, student, subject)`; the
        // surface keeps the reads' order and this is where the two meet.
        self.write(
            py,
            UpdateOp::Assignments(AssignmentsUpdateOp::Assign(
                period_id, student_id, subject_id, assigned,
            )),
        )
    }

    /// Assigns every student to one subject for one period, or empties the row
    ///
    /// The whole row in one write, and one undo slot: `True` assigns every
    /// student the period does not exclude — a student who takes no part in
    /// the period is skipped rather than making the write fail — and `False`
    /// leaves nobody assigned at all.
    ///
    /// ```python
    /// doc.assignments.set_all(period, maths, True)
    /// ```
    ///
    /// A subject that does not run on the period holds nobody there, and the
    /// model refuses both directions of the write with an `AssignmentsError`:
    /// there is no row to empty either.
    fn set_all(
        &self,
        py: Python<'_>,
        period: &Bound<'_, PyAny>,
        subject: &Bound<'_, PyAny>,
        assigned: bool,
    ) -> PyResult<OpResult> {
        let period_id = argument::<Period>(&self.doc, period)?;
        let subject_id = argument::<Subject>(&self.doc, subject)?;

        self.write(
            py,
            UpdateOp::Assignments(AssignmentsUpdateOp::AssignAll(
                period_id, subject_id, assigned,
            )),
        )
    }

    /// Copies the previous period's assignments into this one
    ///
    /// The whole point of the op is the second period of a year looking like
    /// the first: every subject that has a row on `period` takes the
    /// membership the *previous* period gives it. A subject with no row on
    /// `period` is not given one, and neither is one the previous period
    /// leaves empty — this rewrites the rows that are there rather than
    /// rebuilding the table.
    ///
    /// A student either of the two periods excludes is left exactly as they
    /// are: they cannot be assigned in a period they take no part in, and what
    /// the period they missed says about them is no reason to change what this
    /// one does.
    ///
    /// The first period has nothing before it, and asking anyway is refused
    /// with an `AssignmentsError` rather than quietly doing nothing.
    fn duplicate_previous_period(
        &self,
        py: Python<'_>,
        period: &Bound<'_, PyAny>,
    ) -> PyResult<OpResult> {
        let period_id = argument::<Period>(&self.doc, period)?;

        self.write(
            py,
            UpdateOp::Assignments(AssignmentsUpdateOp::DuplicatePreviousPeriod(period_id)),
        )
    }

    /// The view itself — `<collomatique.Assignments>`
    ///
    /// Deliberately without a row count: this collection has no `len`, and a
    /// repr that counted rows would contradict the one statement it makes.
    fn __repr__(&self) -> String {
        "<collomatique.Assignments>".to_owned()
    }
}

impl Assignments {
    /// Writes through the document the view came from
    ///
    /// The whole family ends here: none of its three ops creates anything, so
    /// none of them needs [crate::results::created]'s second half.
    fn write(&self, py: Python<'_>, op: UpdateOp) -> PyResult<OpResult> {
        let mut doc = self.doc.borrow_mut(py);
        doc.update(py, op)
    }
}

/// The rows of a collection, minted as the loop asks for them
///
/// A row is a triple: the `Period` and `Subject` handles of the key, and the
/// assigned students as a frozenset of `Student` handles. The ids were
/// snapshotted when the iteration started, so a removal in the middle
/// leaves the ids standing and the handles minted for a dead entity raise
/// `StaleHandleError` on the first read.
#[pyclass]
pub struct AssignmentIter {
    doc: Py<Document>,
    rows: std::vec::IntoIter<(RawPeriodId, RawSubjectId, BTreeSet<RawStudentId>)>,
}

impl AssignmentIter {
    /// Builds the iterator over an already-taken snapshot of the rows
    pub(crate) fn new(
        doc: Py<Document>,
        rows: Vec<(RawPeriodId, RawSubjectId, BTreeSet<RawStudentId>)>,
    ) -> AssignmentIter {
        AssignmentIter {
            doc,
            rows: rows.into_iter(),
        }
    }
}

#[pymethods]
impl AssignmentIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(
        &mut self,
        py: Python<'py>,
    ) -> PyResult<Option<(Period, Subject, Bound<'py, PyFrozenSet>)>> {
        let Some((period, subject, students)) = self.rows.next() else {
            return Ok(None);
        };
        let students: Vec<Bound<'py, PyAny>> = students
            .into_iter()
            .map(|id| {
                Py::new(py, Student::mint(self.doc.clone_ref(py), id))
                    .map(|student| student.into_bound(py).into_any())
            })
            .collect::<PyResult<_>>()?;
        Ok(Some((
            Period::mint(self.doc.clone_ref(py), period),
            Subject::mint(self.doc.clone_ref(py), subject),
            PyFrozenSet::new(py, &students)?,
        )))
    }
}

/// The `(period, subject)` pair an indexing address must be
///
/// `doc.assignments[p, s]` hands python's indexing a tuple, and nothing else
/// is an address: a bare key, a pair of another length, or a list are all
/// `TypeError`, because the address is written in python and python's spelling
/// of a pair is the tuple. What the two elements name is the caller's
/// question — [argument] answers it, with the two lookup conventions.
struct Address;

impl Address {
    fn of<'py>(key: &Bound<'py, PyAny>) -> PyResult<(Bound<'py, PyAny>, Bound<'py, PyAny>)> {
        let pair = key.cast::<PyTuple>().map_err(|_| {
            pyo3::exceptions::PyTypeError::new_err(format!(
                "doc.assignments[...] takes a (period, subject) pair, and {} is not one",
                shown(key, "that key")
            ))
        })?;
        if pair.len() != 2 {
            return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "doc.assignments[...] takes a (period, subject) pair, and a tuple of {} \
                 elements is not one",
                pair.len()
            )));
        }
        Ok((pair.get_item(0)?, pair.get_item(1)?))
    }
}
