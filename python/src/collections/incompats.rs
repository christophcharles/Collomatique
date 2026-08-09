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

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyTuple};

use collomatique_state_colloscopes::IncompatId as RawIncompatId;
use collomatique_state_colloscopes::InnerData;

use crate::Document;
use crate::collections::subjects::Subject;
use crate::collections::week_patterns::WeekPattern;
use crate::handles::{Handle, handle_iterator, named, no_such, quoted};
use crate::ids::{IdClass, IncompatId};
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

    fn __repr__(&self, py: Python<'_>) -> String {
        format!("<collomatique.Incompats count={}>", self.__len__(py))
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
