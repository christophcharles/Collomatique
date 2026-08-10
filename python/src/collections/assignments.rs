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

use std::collections::BTreeSet;

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyFrozenSet, PyTuple};

use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::PeriodId as RawPeriodId;
use collomatique_state_colloscopes::StudentId as RawStudentId;
use collomatique_state_colloscopes::SubjectId as RawSubjectId;

use crate::Document;
use crate::collections::periods::Period;
use crate::collections::students::Student;
use crate::collections::subjects::Subject;
use crate::handles::{Handle, argument, shown};

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

    /// The view itself — `<collomatique.Assignments>`
    ///
    /// Deliberately without a row count: this collection has no `len`, and a
    /// repr that counted rows would contradict the one statement it makes.
    fn __repr__(&self) -> String {
        "<collomatique.Assignments>".to_owned()
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
