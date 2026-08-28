//! The incompatibilities of a document, and the busy windows they describe
//!
//! Reached as `doc.incompats`. An incompatibility says when the students of a
//! subject may be unavailable: the busy windows of its list, at least
//! `minimum_free_slots` of which must stay free, on the weeks its pattern — when
//! it carries one — leaves on.
//!
//! The subject is deliberately not required to run colles of its own: a student
//! can be declared in a subject purely so that an incompatibility can block
//! slots for them, without the subject having interrogations.
//!
//! Written through `add`, `update` and `remove`. The family sits at the leaf of
//! the reference graph — nothing in the document points at an incompatibility —
//! so a removal takes nothing with it and no write of this family ever makes
//! the cascade repair anything.
//!
//! Every refusal the three ops have is caught on this side of them, which is
//! why nothing here raises `IncompatibilitiesError`: the model can only object
//! to a dead incompatibility, a dead subject or a dead week pattern, and the
//! first is the argument convention's business ([crate::handles::argument])
//! while the other two are the value boundary's ([crate::data::IncompatData]).
//! A script meets a `StaleHandleError` naming the argument it got wrong,
//! instead of a refusal from a layer that knows nothing about handles.

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyTuple};

use collomatique_ops::{IncompatibilitiesUpdateOp, UpdateOp};
use collomatique_state_colloscopes::IncompatId as RawIncompatId;
use collomatique_state_colloscopes::{InnerData, NewId};

use crate::Document;
use crate::collections::subjects::Subject;
use crate::collections::week_patterns::WeekPattern;
use crate::data::{IncompatData, Value as _};
use crate::handles::{Handle, argument, handle_iterator, named, no_such, quoted};
use crate::ids::{IdClass, IncompatId};
use crate::results::{AddResult, OpResult};
use crate::values::TimeSlot;

/// The incompatibilities of one document, in id order
///
/// Frozen and holding nothing but the document: it is a view, so two of them on
/// the same document are interchangeable and neither can go stale.
///
/// The model keeps no display order for the incompatibilities — the application
/// lists them as the table hands them over — so the order here is the ids',
/// which is the one order the document itself has.
#[pyclass(module = "collomatique", frozen)]
pub struct Incompats {
    doc: Py<Document>,
}

impl Incompats {
    /// Builds the view — `doc.incompats` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> Incompats {
        Incompats { doc }
    }

    /// Reads the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }

    /// The incompatibility an id or a handle names, when this document still
    /// holds it
    fn resolve(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<RawIncompatId> {
        let id = named::<Incompat>(&self.doc, key)?;
        self.with_data(py, |data| Incompat::exists(data, id))
            .then_some(id)
    }
}

#[pymethods]
impl Incompats {
    /// How many incompatibilities the document holds
    fn __len__(&self, py: Python<'_>) -> usize {
        self.with_data(py, |data| data.params.incompats.incompat_map.len())
    }

    /// The incompatibilities, as handles, in id order
    fn __iter__(&self, py: Python<'_>) -> IncompatIter {
        let ids = self.with_data(py, |data| {
            data.params.incompats.incompat_map.keys().collect()
        });
        IncompatIter::new(self.doc.clone_ref(py), ids)
    }

    /// The incompatibility an id or a handle names
    ///
    /// Raises `KeyError` when it names nothing in this document — including for
    /// a handle bound to another document, whatever its id says.
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Incompat> {
        let id = self
            .resolve(py, key)
            .ok_or_else(|| no_such("incompatibility", key))?;
        Ok(Incompat::mint(self.doc.clone_ref(py), id))
    }

    /// The incompatibility an id or a handle names, or `None`
    fn get(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<Incompat> {
        let id = self.resolve(py, key)?;
        Some(Incompat::mint(self.doc.clone_ref(py), id))
    }

    fn __contains__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> bool {
        self.resolve(py, key).is_some()
    }

    /// Adds an incompatibility, and hands back the handle of the new one
    ///
    /// Takes an `IncompatData` — the whole of what an incompatibility is, since
    /// the entity and the op payload are the same type here — and answers an
    /// `AddResult`, whose `created` is the `Incompat` the document just minted.
    ///
    /// ```python
    /// doc.incompats.add(collomatique.IncompatData(
    ///     "Lundi Midi", maths,
    ///     slots=[clm.TimeSlot(clm.Weekday.MONDAY, datetime.time(12, 0), 60)]))
    /// ```
    ///
    /// The subject is deliberately not required to hold interrogations of its
    /// own: a student can be declared in a subject purely so that an
    /// incompatibility can block slots for them.
    fn add(&self, py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Py<AddResult>> {
        // Extracted before the mutable borrow, never inside it: a value naming
        // an entity is resolved against this document, which borrows it to ask.
        let incompat = IncompatData::from_py(&self.doc, data)?;

        crate::results::created::<Incompat>(
            py,
            &self.doc,
            UpdateOp::Incompatibilities(IncompatibilitiesUpdateOp::AddNewIncompat(incompat)),
            |new_id| match new_id {
                NewId::IncompatId(id) => Some(id),
                _ => None,
            },
        )
    }

    /// Rewrites an incompatibility whole
    ///
    /// The op carries the whole value, so this replaces every field at once:
    /// what the `IncompatData` says is what the incompatibility becomes. The id
    /// stays, and so does every handle naming it.
    ///
    /// The incompatibility is resolved before the value is read, so a call that
    /// is wrong about both says which incompatibility it could not find rather
    /// than what was wrong with a value meant for nothing.
    fn update(
        &self,
        py: Python<'_>,
        key: &Bound<'_, PyAny>,
        data: &Bound<'_, PyAny>,
    ) -> PyResult<OpResult> {
        let id = argument::<Incompat>(&self.doc, key)?;
        let incompat = IncompatData::from_py(&self.doc, data)?;

        self.write(
            py,
            UpdateOp::Incompatibilities(IncompatibilitiesUpdateOp::UpdateIncompat(id, incompat)),
        )
    }

    /// Removes an incompatibility
    ///
    /// Nothing in the document points at an incompatibility, so the removal
    /// takes nothing with it and the warnings are always empty. Handles naming
    /// it go stale, like every other removal's.
    fn remove(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<OpResult> {
        let id = argument::<Incompat>(&self.doc, key)?;

        self.write(
            py,
            UpdateOp::Incompatibilities(IncompatibilitiesUpdateOp::DeleteIncompat(id)),
        )
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!("<collomatique.Incompats count={}>", self.__len__(py))
    }
}

impl Incompats {
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
    /// The incompatibilities of a collection, minted as the loop asks for them
    IncompatIter yielding Incompat
}

/// One incompatibility of the document
///
/// A live view: every attribute reads the document as it stands now. Reading one
/// whose incompatibility has been removed raises `StaleHandleError`; `.id`, `==`
/// and `hash` keep working, since they never touch the state.
#[pyclass(module = "collomatique", frozen)]
pub struct Incompat {
    doc: Py<Document>,
    id: RawIncompatId,
}

impl Handle for Incompat {
    type IdClass = IncompatId;

    const CLASS: &'static str = "Incompat";
    const NOUN: &'static str = "incompatibility";

    fn mint(doc: Py<Document>, id: RawIncompatId) -> Incompat {
        Incompat { doc, id }
    }

    fn document(&self) -> &Py<Document> {
        &self.doc
    }

    fn raw_id(&self) -> RawIncompatId {
        self.id
    }

    fn exists(data: &InnerData, id: RawIncompatId) -> bool {
        data.params.incompats.incompat_map.contains(&id)
    }
}

#[pymethods]
impl Incompat {
    /// The incompatibility's id
    ///
    /// The one attribute that works on a stale handle: it reads nothing.
    #[getter]
    fn id(&self) -> IncompatId {
        IncompatId::wrap(self.id)
    }

    /// The incompatibility's name — « Lundi Midi » and the like
    ///
    /// A plain string, the empty one included: the model types this field as a
    /// `String` and python mirrors it rather than editorializing.
    #[getter]
    fn name(&self, py: Python<'_>) -> PyResult<String> {
        self.read(py, |data| {
            data.params
                .incompats
                .incompat_map
                .get(&self.id)
                .map(|incompat| incompat.name.clone())
        })
    }

    /// The subject whose students this incompatibility constrains
    ///
    /// Deliberately not required to hold interrogations of its own: a student
    /// can be declared in a subject purely so that an incompatibility can block
    /// slots for them, without the subject having colles.
    #[getter]
    fn subject(&self, py: Python<'_>) -> PyResult<Subject> {
        let subject_id = self.read(py, |data| {
            data.params
                .incompats
                .incompat_map
                .get(&self.id)
                .map(|incompat| incompat.subject_id)
        })?;
        Ok(Subject::mint(self.doc.clone_ref(py), subject_id))
    }

    /// The busy windows, as [TimeSlot] values, in the model's order
    ///
    /// The whole of what an incompatibility says: the students of the subject
    /// must have at least `minimum_free_slots` of these windows free. The
    /// windows are values, not handles — nothing points at one by position, so
    /// a script takes them apart and compares them as data.
    #[getter]
    fn slots<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let slots = self.read(py, |data| {
            data.params
                .incompats
                .incompat_map
                .get(&self.id)
                .map(|incompat| incompat.slots.clone())
        })?;

        let slots: Vec<_> = slots.iter().map(TimeSlot::from_model).collect();
        PyTuple::new(py, slots)
    }

    /// How many of the busy windows must stay free
    ///
    /// At least one: an incompatibility that could spare every window would be
    /// no incompatibility at all.
    #[getter]
    fn minimum_free_slots(&self, py: Python<'_>) -> PyResult<u32> {
        self.read(py, |data| {
            data.params
                .incompats
                .incompat_map
                .get(&self.id)
                .map(|incompat| incompat.minimum_free_slots.get())
        })
    }

    /// The pattern saying which weeks this incompatibility applies on, or `None`
    ///
    /// `None` means every week — the incompatibility has no pattern of its own,
    /// so only the weeks' own flags switch it off.
    #[getter]
    fn week_pattern(&self, py: Python<'_>) -> PyResult<Option<WeekPattern>> {
        let pattern_id = self.read(py, |data| {
            data.params
                .incompats
                .incompat_map
                .get(&self.id)
                .map(|incompat| incompat.week_pattern_id)
        })?;

        Ok(pattern_id.map(|pattern_id| WeekPattern::mint(self.doc.clone_ref(py), pattern_id)))
    }

    /// Nothing can point at an incompatibility: the reference registry has no
    /// site vocabulary for the kind, so the answer is always the empty tuple
    /// while the handle is alive. A stale handle raises `StaleHandleError` like
    /// every other read.
    fn referenced_by(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        crate::refs::never_referenced::<Self>(py, self)
    }

    /// This incompatibility, detached — an `IncompatData` holding what the
    /// handle shows
    ///
    /// A fresh object every call. The subject and the pattern come out as ids
    /// rather than as handles, because a value holding handles would carry this
    /// document around with it and keep it alive. The busy windows come out as
    /// a *list* of [TimeSlot] — the mutable container a value is for — where
    /// the handle's read hands back the read surface's tuple.
    ///
    /// A stale handle raises `StaleHandleError` like every other read.
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        use crate::data::Value as _;

        // Copied out of the borrow before anything python-facing happens:
        // building the value calls into python, and doing that under the
        // document's borrow is how a nested borrow becomes a `PanicException`.
        let incompat = self.read(py, |data| {
            data.params.incompats.incompat_map.get(&self.id).cloned()
        })?;

        crate::data::IncompatData::to_py(py, &incompat)
    }

    /// Whether two handles name the same incompatibility of the same document
    ///
    /// Never reads the state, so it keeps working once the incompatibility is
    /// gone — a dict holding handles must not blow up when an entity dies.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<Incompat>() {
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
                .incompats
                .incompat_map
                .get(&self.id)
                .map(|incompat| incompat.name.clone())
        });
        self.repr_text(name.map(|name| quoted(py, &name)))
    }
}
