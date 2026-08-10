//! The students of a document, and the periods they sit out
//!
//! Reached as `doc.students`. A student is a person — a name and, when they gave
//! them, contact details — together with the periods they take no part in.
//! Which subjects a student takes is not here: the model keeps that in a
//! junction table of its own, keyed by period and subject.

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyFrozenSet, PyTuple};

use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::StudentId as RawStudentId;

use crate::Document;
use crate::collections::periods::Period;
use crate::handles::{Handle, handle_iterator, named, no_such, quoted};
use crate::ids::{IdClass, StudentId};

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

    fn __repr__(&self, py: Python<'_>) -> String {
        format!("<collomatique.Students count={}>", self.__len__(py))
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
