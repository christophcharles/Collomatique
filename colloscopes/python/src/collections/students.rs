//! The students of a document, and the periods they sit out
//!
//! Reached as `doc.students`. A student is a person — a name and, when they gave
//! them, contact details — together with the periods they take no part in.
//! Which subjects a student takes is not here: the model keeps that in a
//! junction table of its own, keyed by period and subject.
//!
//! Written through `add`, `update` and `remove`. A student's name is written
//! down all over the document — the rows of `doc.assignments`, the groups of a
//! prefilled group list, the excluded set of an automatic one, the per-student
//! entry of `doc.settings`, the placements of a colloscope — and none of those
//! sites can go on naming somebody the document no longer holds, so removing a
//! student takes their name out of every one of them, and each removal comes
//! back on the `OpResult`. An `update` cascades too, without anybody being
//! removed: a student who now sits a period out cannot be assigned in it, so
//! that period's assignment rows let them go.
//!
//! The family keeps no refusal for the model. `StudentsUpdateOp` can object to
//! two things — a student id the document does not hold, and an excluded period
//! that names nothing — and both are caught on this side, where the message can
//! say which argument was wrong: a dead student is the argument convention's
//! business ([crate::handles::argument]), and a dead period is the value
//! boundary's. So nothing here raises `StudentsError`, unlike the teachers next
//! door, where whether a subject runs colles is a statement about the document
//! that only the write can make.

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyFrozenSet, PyTuple};

use collomatique_ops::{StudentsUpdateOp, UpdateOp};
use collomatique_state_colloscopes::StudentId as RawStudentId;
use collomatique_state_colloscopes::{InnerData, NewId};

use crate::Document;
use crate::collections::periods::Period;
use crate::data::{StudentData, Value as _};
use crate::handles::{Handle, argument, handle_iterator, named, no_such, quoted};
use crate::ids::{IdClass, StudentId};
use crate::results::{AddResult, OpResult};

/// The students of one document, in id order
///
/// Frozen and holding nothing but the document: it is a view, so two of them on
/// the same document are interchangeable and neither can go stale.
///
/// The model keeps no display order for the students — the application sorts
/// them by name as it shows them — so the order here is the ids', which is the
/// one order the document itself has.
#[pyclass(module = "collomatique", frozen)]
pub struct Students {
    doc: Py<Document>,
}

impl Students {
    /// Builds the view — `doc.students` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> Students {
        Students { doc }
    }

    /// Reads the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }

    /// The student an id or a handle names, when this document still holds them
    fn resolve(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<RawStudentId> {
        let id = named::<Student>(&self.doc, key)?;
        self.with_data(py, |data| Student::exists(data, id))
            .then_some(id)
    }
}

#[pymethods]
impl Students {
    /// How many students the document holds
    fn __len__(&self, py: Python<'_>) -> usize {
        self.with_data(py, |data| data.params.students.student_map.len())
    }

    /// The students, as handles, in id order
    fn __iter__(&self, py: Python<'_>) -> StudentIter {
        let ids = self.with_data(py, |data| data.params.students.student_map.keys().collect());
        StudentIter::new(self.doc.clone_ref(py), ids)
    }

    /// The student an id or a handle names
    ///
    /// Raises `KeyError` when it names nothing in this document — including for
    /// a handle bound to another document, whatever its id says.
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Student> {
        let id = self
            .resolve(py, key)
            .ok_or_else(|| no_such("student", key))?;
        Ok(Student::mint(self.doc.clone_ref(py), id))
    }

    /// The student an id or a handle names, or `None`
    fn get(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<Student> {
        let id = self.resolve(py, key)?;
        Some(Student::mint(self.doc.clone_ref(py), id))
    }

    fn __contains__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> bool {
        self.resolve(py, key).is_some()
    }

    /// Adds a student, and hands back the handle of the new one
    ///
    /// Takes a `StudentData` — the whole of what a student is, since the entity
    /// and the op payload are the same type here — and answers an `AddResult`,
    /// whose `created` is the `Student` the document just minted.
    ///
    /// ```python
    /// doc.students.add(collomatique.StudentData(
    ///     "Luna", "Lovegood", email="luna@poudlard.fr",
    ///     excluded_periods={first_period}))
    /// ```
    ///
    /// A student arrives assigned to nothing and in no group, so there is
    /// nothing for the cascade to repair: the answer's `warnings` is empty.
    /// Which subjects they take is written afterwards, through
    /// `doc.assignments`.
    fn add(&self, py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Py<AddResult>> {
        // Extracted before the mutable borrow, never inside it: a value naming
        // an entity is resolved against this document, which borrows it to ask.
        let student = StudentData::from_py(&self.doc, data)?;

        crate::results::created::<Student>(
            py,
            &self.doc,
            UpdateOp::Students(StudentsUpdateOp::AddNewStudent(student)),
            |new_id| match new_id {
                NewId::StudentId(id) => Some(id),
                _ => None,
            },
        )
    }

    /// Rewrites a student whole
    ///
    /// The op carries the whole value, so this replaces every field at once:
    /// what the `StudentData` says is what the student becomes, the card and
    /// the excluded periods together. The id stays, and so does every handle
    /// naming it.
    ///
    /// Excluding a period the student was assigned in is a write like any
    /// other, and the cascade repairs what it broke: nobody can be assigned in
    /// a period they take no part in, so that period's assignment rows let them
    /// go, and the warnings say so.
    ///
    /// The student is resolved before the value is read, so a call that is
    /// wrong about both says which student it could not find rather than what
    /// was wrong with a value meant for nothing.
    fn update(
        &self,
        py: Python<'_>,
        key: &Bound<'_, PyAny>,
        data: &Bound<'_, PyAny>,
    ) -> PyResult<OpResult> {
        let id = argument::<Student>(&self.doc, key)?;
        let student = StudentData::from_py(&self.doc, data)?;

        self.write(
            py,
            UpdateOp::Students(StudentsUpdateOp::UpdateStudent(id, student)),
        )
    }

    /// Removes a student
    ///
    /// Every site that names the student lets go of them — the assignment rows
    /// they sat in, the prefilled group that held them, the automatic list that
    /// excluded them, their entry in the settings, their placements in a
    /// colloscope — and the `OpResult` carries every one of those repairs.
    /// Nothing else is removed with them: an assignment row, a group list and a
    /// limits entry all survive losing one name, so this cascade is wide rather
    /// than deep. Handles naming the student go stale.
    fn remove(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<OpResult> {
        let id = argument::<Student>(&self.doc, key)?;

        self.write(py, UpdateOp::Students(StudentsUpdateOp::DeleteStudent(id)))
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!("<collomatique.Students count={}>", self.__len__(py))
    }
}

impl Students {
    /// Writes through the document the view came from
    ///
    /// The two mutators that create nothing end here. The creating one ends in
    /// [crate::results::created], which takes the same borrow and keeps the id
    /// the op issued as well.
    fn write(&self, py: Python<'_>, op: UpdateOp) -> PyResult<OpResult> {
        let mut doc = self.doc.borrow_mut(py);
        doc.update(py, op)
    }
}

handle_iterator! {
    /// The students of a collection, minted as the loop asks for them
    StudentIter yielding Student
}

/// One student of the document
///
/// A live view: every attribute reads the document as it stands now. Reading one
/// whose student has been removed raises `StaleHandleError`; `.id`, `==` and
/// `hash` keep working, since they never touch the state.
///
/// The name and the contact details are flattened onto the handle, the way they
/// are on a [Teacher]: the model keeps them in a card the two entities share,
/// and a card of four fields is a level of nesting a script would only ever type
/// through.
///
/// [Teacher]: crate::collections::Teacher
#[pyclass(module = "collomatique", frozen)]
pub struct Student {
    doc: Py<Document>,
    id: RawStudentId,
}

impl Handle for Student {
    type IdClass = StudentId;

    const CLASS: &'static str = "Student";
    const NOUN: &'static str = "student";

    fn mint(doc: Py<Document>, id: RawStudentId) -> Student {
        Student { doc, id }
    }

    fn document(&self) -> &Py<Document> {
        &self.doc
    }

    fn raw_id(&self) -> RawStudentId {
        self.id
    }

    fn exists(data: &InnerData, id: RawStudentId) -> bool {
        data.params.students.student_map.contains(&id)
    }
}

#[pymethods]
impl Student {
    /// The student's id
    ///
    /// The one attribute that works on a stale handle: it reads nothing.
    #[getter]
    fn id(&self) -> StudentId {
        StudentId::wrap(self.id)
    }

    /// The student's surname
    ///
    /// A plain string, the empty one included: the model types this field as a
    /// `String` and python mirrors it rather than editorializing.
    #[getter]
    fn surname(&self, py: Python<'_>) -> PyResult<String> {
        self.read(py, |data| {
            data.params
                .students
                .student_map
                .get(&self.id)
                .map(|student| student.desc.surname.clone())
        })
    }

    /// The student's first name
    ///
    /// A plain string like the surname, and empty for the same reasons.
    #[getter]
    fn firstname(&self, py: Python<'_>) -> PyResult<String> {
        self.read(py, |data| {
            data.params
                .students
                .student_map
                .get(&self.id)
                .map(|student| student.desc.firstname.clone())
        })
    }

    /// The student's telephone number, or `None`
    ///
    /// `None` and not `""`: the model types this field as an optional non-empty
    /// string — a student who shared no number has none, rather than having an
    /// empty one — and python mirrors it rather than editorializing.
    #[getter]
    fn tel(&self, py: Python<'_>) -> PyResult<Option<String>> {
        self.read(py, |data| {
            data.params
                .students
                .student_map
                .get(&self.id)
                .map(|student| student.desc.tel.as_ref().map(|tel| tel.to_string()))
        })
    }

    /// The student's email address, or `None`
    ///
    /// `None` and not `""`, for the reason [Student::tel] gives.
    #[getter]
    fn email(&self, py: Python<'_>) -> PyResult<Option<String>> {
        self.read(py, |data| {
            data.params
                .students
                .student_map
                .get(&self.id)
                .map(|student| student.desc.email.as_ref().map(|email| email.to_string()))
        })
    }

    /// The periods this student is absent from, as a `frozenset` of [Period]
    ///
    /// A snapshot, built when it is asked for: the set does not grow when the
    /// document does. The handles in it stay live.
    #[getter]
    fn excluded_periods<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyFrozenSet>> {
        let ids = self.read(py, |data| {
            let student = data.params.students.student_map.get(&self.id)?;
            Some(student.excluded_periods.iter().copied().collect::<Vec<_>>())
        })?;

        let periods: Vec<_> = ids
            .into_iter()
            .map(|period_id| Period::mint(self.doc.clone_ref(py), period_id))
            .collect();
        PyFrozenSet::new(py, periods)
    }

    /// What points at this student — every site whose coordinates name it, as a
    /// tuple of `RefSite` values, in the registry's walk order. An empty tuple
    /// means nothing points here.
    ///
    /// A stale handle raises `StaleHandleError` like every other read.
    fn referenced_by(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        crate::refs::student_references(py, self)
    }

    /// This student, detached — a `StudentData` holding what the handle shows
    ///
    /// A fresh object every call, with the excluded periods as `PeriodId`s
    /// rather than as handles, for the reason [Teacher::to_data] gives.
    ///
    /// A stale handle raises `StaleHandleError` like every other read.
    ///
    /// [Teacher::to_data]: crate::collections::Teacher::to_data
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        use crate::data::Value as _;

        let student = self.read(py, |data| {
            data.params.students.student_map.get(&self.id).cloned()
        })?;

        crate::data::StudentData::to_py(py, &student)
    }

    /// Whether two handles name the same student of the same document
    ///
    /// Never reads the state, so it keeps working once the student is gone — a
    /// dict holding handles must not blow up when an entity dies.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<Student>() {
            Ok(other) => self.same_as(other.get()),
            Err(_) => false,
        }
    }

    fn __hash__(&self) -> u64 {
        self.hash_key()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let name = self.peek(py, |data| {
            data.params
                .students
                .student_map
                .get(&self.id)
                .map(|student| crate::collections::person_name(&student.desc))
        });
        self.repr_text(name.map(|name| quoted(py, &name)))
    }
}
