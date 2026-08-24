//! The teachers of a document, and the subjects they interrogate in
//!
//! Reached as `doc.teachers`. A teacher is a person — a name and, when they gave
//! them, contact details — together with the set of subjects they interrogate
//! in.
//!
//! Written through `add`, `update` and `remove`. This is the first family whose
//! removal cascades: a slot names the teacher who holds it, and there is no
//! teacher-less slot to fall back to, so removing a teacher takes their slots
//! with it — and whatever those slots held in their turn, the colloscope cells
//! in them and the slot pairing rules that related them. Every one of those
//! repairs comes back on the `OpResult`, and a script that removes a teacher
//! without reading them is throwing away the only account of what else moved.
//!
//! The family keeps one refusal for the model, and it reaches a script as
//! `TeachersError`: a teacher may only be declared in a subject that holds
//! interrogations — there are no colles to hold in one that does not — and the
//! model refuses it for `add` and for `update` alike. Whether a subject runs
//! interrogations is a statement about the document rather than about the
//! value, which is why [crate::data::TeacherData] leaves it to the write. What
//! the model could otherwise object to is caught on this side, where the
//! message can say which argument was wrong: a dead teacher is the argument
//! convention's business ([crate::handles::argument]), and a dead subject is
//! the value boundary's.

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyFrozenSet, PyTuple};

use collomatique_ops::{TeachersUpdateOp, UpdateOp};
use collomatique_state_colloscopes::TeacherId as RawTeacherId;
use collomatique_state_colloscopes::{InnerData, NewId};

use crate::Document;
use crate::collections::subjects::Subject;
use crate::data::{TeacherData, Value as _};
use crate::handles::{Handle, argument, handle_iterator, named, no_such, quoted};
use crate::ids::{IdClass, TeacherId};
use crate::results::{AddResult, OpResult};

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

    /// Adds a teacher, and hands back the handle of the new one
    ///
    /// Takes a `TeacherData` — the whole of what a teacher is, since the entity
    /// and the op payload are the same type here — and answers an `AddResult`,
    /// whose `created` is the `Teacher` the document just minted.
    ///
    /// ```python
    /// doc.teachers.add(collomatique.TeacherData(
    ///     "Emmy", "Noether", email="noether@lycee.fr", subjects={maths}))
    /// ```
    ///
    /// Every subject the value names must run interrogations: nobody can be
    /// declared to teach a subject that holds no colles, and the model refuses
    /// it with a `TeachersError`. A teacher who interrogates in nothing at all
    /// is perfectly ordinary, on the other hand — an empty `subjects` is what a
    /// new teacher starts with.
    fn add(&self, py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Py<AddResult>> {
        // Extracted before the mutable borrow, never inside it: a value naming
        // an entity is resolved against this document, which borrows it to ask
        // (`docs/python/new_api_design.md` §5).
        let teacher = TeacherData::from_py(&self.doc, data)?;

        crate::results::created::<Teacher>(
            py,
            &self.doc,
            UpdateOp::Teachers(TeachersUpdateOp::AddNewTeacher(teacher)),
            |new_id| match new_id {
                NewId::TeacherId(id) => Some(id),
                _ => None,
            },
        )
    }

    /// Rewrites a teacher whole
    ///
    /// The op carries the whole value, so this replaces every field at once:
    /// what the `TeacherData` says is what the teacher becomes, the card and
    /// the subjects together. The id stays, and so does every handle naming it.
    ///
    /// Dropping a subject from the set is a write like any other, and the
    /// cascade repairs what it broke: the teacher's slots in that subject have
    /// nobody to hold them any more, so they go, and the warnings say so.
    ///
    /// The teacher is resolved before the value is read, so a call that is
    /// wrong about both says which teacher it could not find rather than what
    /// was wrong with a value meant for nothing.
    fn update(
        &self,
        py: Python<'_>,
        key: &Bound<'_, PyAny>,
        data: &Bound<'_, PyAny>,
    ) -> PyResult<OpResult> {
        let id = argument::<Teacher>(&self.doc, key)?;
        let teacher = TeacherData::from_py(&self.doc, data)?;

        self.write(
            py,
            UpdateOp::Teachers(TeachersUpdateOp::UpdateTeacher(id, teacher)),
        )
    }

    /// Removes a teacher
    ///
    /// A slot names the teacher who holds it and cannot do without one, so the
    /// removal takes the teacher's slots with it, and whatever those slots held
    /// in their turn. The `OpResult` carries every repair, each one linked to
    /// the one that needed it. Handles naming the teacher go stale, and so do
    /// the ones naming the slots that went.
    fn remove(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<OpResult> {
        let id = argument::<Teacher>(&self.doc, key)?;

        self.write(py, UpdateOp::Teachers(TeachersUpdateOp::DeleteTeacher(id)))
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!("<collomatique.Teachers count={}>", self.__len__(py))
    }
}

impl Teachers {
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
/// would only ever type through.
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

    /// What points at this teacher — every site whose coordinates name it, as a
    /// tuple of `RefSite` values, in the registry's walk order. An empty tuple
    /// means nothing points here.
    ///
    /// A stale handle raises `StaleHandleError` like every other read.
    fn referenced_by(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        crate::refs::teacher_references(py, self)
    }

    /// This teacher, detached — a `TeacherData` holding what the handle shows
    ///
    /// A fresh object every call: two calls give two values that compare equal
    /// and share nothing, and writing to one changes nothing anywhere. The
    /// subjects come out as `SubjectId`s rather than as handles, because a value
    /// holding handles would carry this document around with it and keep it
    /// alive.
    ///
    /// A stale handle raises `StaleHandleError` like every other read.
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        use crate::data::Value as _;

        // Copied out of the borrow before anything python-facing happens.
        // Building the value calls into python — the dataclass's own
        // `__init__` — and doing that while the document is borrowed is how a
        // nested borrow becomes a `PanicException`.
        let teacher = self.read(py, |data| {
            data.params.teachers.teacher_map.get(&self.id).cloned()
        })?;

        crate::data::TeacherData::to_py(py, &teacher)
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
