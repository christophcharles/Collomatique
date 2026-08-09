//! The teachers of a document, and the subjects they interrogate in
//!
//! Reached as `doc.teachers`. A teacher is a person — a name and, when they gave
//! them, contact details — together with the set of subjects they interrogate
//! in.

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyFrozenSet};

use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::TeacherId as RawTeacherId;

use crate::Document;
use crate::collections::subjects::Subject;
use crate::handles::{Handle, handle_iterator, named, no_such, quoted};
use crate::ids::{IdClass, TeacherId};

/// The teachers of one document, in id order
///
/// Frozen and holding nothing but the document: it is a view, so two of them on
/// the same document are interchangeable and neither can go stale.
///
/// The model keeps no display order for the teachers — the application sorts
/// them by name as it shows them — so the order here is the ids', which is the
/// one order the document itself has.
#[pyclass(module = "collomatique", frozen)]
pub struct Teachers {
    doc: Py<Document>,
}

impl Teachers {
    /// Builds the view — `doc.teachers` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> Teachers {
        Teachers { doc }
    }

    /// Reads the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }

    /// The teacher an id or a handle names, when this document still holds them
    fn resolve(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<RawTeacherId> {
        let id = named::<Teacher>(&self.doc, key)?;
        self.with_data(py, |data| Teacher::exists(data, id))
            .then_some(id)
    }
}

#[pymethods]
impl Teachers {
    /// How many teachers the document holds
    fn __len__(&self, py: Python<'_>) -> usize {
        self.with_data(py, |data| data.params.teachers.teacher_map.len())
    }

    /// The teachers, as handles, in id order
    fn __iter__(&self, py: Python<'_>) -> TeacherIter {
        let ids = self.with_data(py, |data| data.params.teachers.teacher_map.keys().collect());
        TeacherIter::new(self.doc.clone_ref(py), ids)
    }

    /// The teacher an id or a handle names
    ///
    /// Raises `KeyError` when it names nothing in this document — including for
    /// a handle bound to another document, whatever its id says.
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Teacher> {
        let id = self
            .resolve(py, key)
            .ok_or_else(|| no_such("teacher", key))?;
        Ok(Teacher::mint(self.doc.clone_ref(py), id))
    }

    /// The teacher an id or a handle names, or `None`
    fn get(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<Teacher> {
        let id = self.resolve(py, key)?;
        Some(Teacher::mint(self.doc.clone_ref(py), id))
    }

    fn __contains__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> bool {
        self.resolve(py, key).is_some()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!("<collomatique.Teachers count={}>", self.__len__(py))
    }
}

handle_iterator! {
    /// The teachers of a collection, minted as the loop asks for them
    TeacherIter yielding Teacher
}

/// One teacher of the document
///
/// A live view: every attribute reads the document as it stands now. Reading one
/// whose teacher has been removed raises `StaleHandleError`; `.id`, `==` and
/// `hash` keep working, since they never touch the state.
///
/// The model keeps the name and the contact details in a card of their own, one
/// it shares with the students. Python flattens it onto the handle, because a
/// card that holds four fields and nothing else is a level of nesting a script
/// would only ever type through (`docs/python/handle_api.md` §3.4).
#[pyclass(module = "collomatique", frozen)]
pub struct Teacher {
    doc: Py<Document>,
    id: RawTeacherId,
}

impl Handle for Teacher {
    type IdClass = TeacherId;

    const CLASS: &'static str = "Teacher";
    const NOUN: &'static str = "teacher";

    fn mint(doc: Py<Document>, id: RawTeacherId) -> Teacher {
        Teacher { doc, id }
    }

    fn document(&self) -> &Py<Document> {
        &self.doc
    }

    fn raw_id(&self) -> RawTeacherId {
        self.id
    }

    fn exists(data: &InnerData, id: RawTeacherId) -> bool {
        data.params.teachers.teacher_map.contains(&id)
    }
}

#[pymethods]
impl Teacher {
    /// The teacher's id
    ///
    /// The one attribute that works on a stale handle: it reads nothing.
    #[getter]
    fn id(&self) -> TeacherId {
        TeacherId::wrap(self.id)
    }

    /// The teacher's surname
    ///
    /// A plain string, the empty one included: the model types this field as a
    /// `String` and python mirrors it rather than editorializing.
    #[getter]
    fn surname(&self, py: Python<'_>) -> PyResult<String> {
        self.read(py, |data| {
            data.params
                .teachers
                .teacher_map
                .get(&self.id)
                .map(|teacher| teacher.desc.surname.clone())
        })
    }

    /// The teacher's first name
    ///
    /// A plain string like the surname, and empty for the same reasons.
    #[getter]
    fn firstname(&self, py: Python<'_>) -> PyResult<String> {
        self.read(py, |data| {
            data.params
                .teachers
                .teacher_map
                .get(&self.id)
                .map(|teacher| teacher.desc.firstname.clone())
        })
    }

    /// The teacher's telephone number, or `None`
    ///
    /// `None` and not `""`: the model types this field as an optional non-empty
    /// string — a teacher who shared no number has none, rather than having an
    /// empty one — and python mirrors it rather than editorializing.
    #[getter]
    fn tel(&self, py: Python<'_>) -> PyResult<Option<String>> {
        self.read(py, |data| {
            data.params
                .teachers
                .teacher_map
                .get(&self.id)
                .map(|teacher| teacher.desc.tel.as_ref().map(|tel| tel.to_string()))
        })
    }

    /// The teacher's email address, or `None`
    ///
    /// `None` and not `""`, for the reason [Teacher::tel] gives.
    #[getter]
    fn email(&self, py: Python<'_>) -> PyResult<Option<String>> {
        self.read(py, |data| {
            data.params
                .teachers
                .teacher_map
                .get(&self.id)
                .map(|teacher| teacher.desc.email.as_ref().map(|email| email.to_string()))
        })
    }

    /// The subjects this teacher interrogates in, as a `frozenset` of [Subject]
    ///
    /// A snapshot, built when it is asked for: the set does not grow when the
    /// document does. The handles in it stay live.
    #[getter]
    fn subjects<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyFrozenSet>> {
        let ids = self.read(py, |data| {
            let teacher = data.params.teachers.teacher_map.get(&self.id)?;
            Some(teacher.subjects.iter().copied().collect::<Vec<_>>())
        })?;

        let subjects: Vec<_> = ids
            .into_iter()
            .map(|subject_id| Subject::mint(self.doc.clone_ref(py), subject_id))
            .collect();
        PyFrozenSet::new(py, subjects)
    }

    /// Whether two handles name the same teacher of the same document
    ///
    /// Never reads the state, so it keeps working once the teacher is gone — a
    /// dict holding handles must not blow up when an entity dies.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<Teacher>() {
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
                .teachers
                .teacher_map
                .get(&self.id)
                .map(|teacher| crate::collections::person_name(&teacher.desc))
        });
        self.repr_text(name.map(|name| quoted(py, &name)))
    }
}
