//! The slots of a document, and when a slot can carry a colle
//!
//! Reached as `doc.slots`, and as `subject.slots` for the slots of one subject.
//! A slot is a weekly appointment: a teacher, a day, a time, and — when it
//! carries one — the pattern that says which weeks it really runs on.
//!
//! A slot has no duration of its own. The subject fixes it, so the length of a
//! colle is `slot.subject.interrogation.duration`, and an alias here would only
//! hide where the model keeps it. That expression assumes the subject holds
//! colles at all — a slot on one that does not is reachable, and there
//! `subject.interrogation` is `None`.
//!
//! Whether a colle really happens on a given week is not the slot's answer
//! alone: the subject must run colles, must not skip that week's period, and the
//! week must be active under the slot's pattern. The [Document]'s own
//! `is_interrogation_possible` puts the three together.

use pyo3::prelude::*;
use pyo3::types::PyAny;

use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::SlotId as RawSlotId;

use crate::Document;
use crate::collections::subjects::Subject;
use crate::collections::teachers::Teacher;
use crate::collections::week_patterns::WeekPattern;
use crate::handles::{Handle, handle_iterator, named, no_such};
use crate::ids::{IdClass, SlotId};
use crate::values::Weekday;

/// The slots of one document, in subject-then-position order
///
/// Frozen and holding nothing but the document: it is a view, so two of them on
/// the same document are interchangeable and neither can go stale.
///
/// Both orders a slot has are user orders: the subjects come in the order the
/// application shows them, and each subject's slots in the order it keeps them
/// in. So `doc.slots` and `doc.subjects` agree — walking the subjects and their
/// slots gives exactly this walk — and `slot.index` counts inside the subject,
/// not along it, because inside a subject is where the model keeps a position.
#[pyclass(module = "collomatique", frozen)]
pub struct Slots {
    doc: Py<Document>,
}

impl Slots {
    /// Builds the view — `doc.slots` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> Slots {
        Slots { doc }
    }

    /// Every slot of the document, in the one order this view has
    ///
    /// The single definition of that order, so that `len` and iteration cannot
    /// come to differ: the subjects in display order, each followed by its own
    /// slots. A subject with no slots simply contributes none.
    fn walk(data: &InnerData) -> impl Iterator<Item = RawSlotId> + '_ {
        data.params
            .subjects
            .ordered_subject_list
            .keys()
            .flat_map(move |subject_id| {
                data.params
                    .slots
                    .slots_for_subject(subject_id)
                    .into_iter()
                    .flatten()
                    .map(|(slot_id, _slot)| *slot_id)
            })
    }

    /// Reads the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }

    /// The slot an id or a handle names, when this document still holds it
    fn resolve(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<RawSlotId> {
        let id = named::<Slot>(&self.doc, key)?;
        self.with_data(py, |data| Slot::exists(data, id))
            .then_some(id)
    }
}

#[pymethods]
impl Slots {
    /// How many slots the document holds, across every subject
    fn __len__(&self, py: Python<'_>) -> usize {
        self.with_data(py, |data| Slots::walk(data).count())
    }

    /// The slots, as handles, in subject-then-position order
    fn __iter__(&self, py: Python<'_>) -> SlotIter {
        let ids = self.with_data(py, |data| Slots::walk(data).collect());
        SlotIter::new(self.doc.clone_ref(py), ids)
    }

    /// The slot an id or a handle names
    ///
    /// Raises `KeyError` when it names nothing in this document — including for
    /// a handle bound to another document, whatever its id says.
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Slot> {
        let id = self.resolve(py, key).ok_or_else(|| no_such("slot", key))?;
        Ok(Slot::mint(self.doc.clone_ref(py), id))
    }

    /// The slot an id or a handle names, or `None`
    fn get(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<Slot> {
        let id = self.resolve(py, key)?;
        Some(Slot::mint(self.doc.clone_ref(py), id))
    }

    fn __contains__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> bool {
        self.resolve(py, key).is_some()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!("<collomatique.Slots count={}>", self.__len__(py))
    }
}

handle_iterator! {
    /// The slots of a collection, minted as the loop asks for them
    SlotIter yielding Slot
}

/// One slot of the document
///
/// A live view: every attribute reads the document as it stands now. Reading one
/// whose slot has been removed raises `StaleHandleError`; `.id`, `==` and `hash`
/// keep working, since they never touch the state.
#[pyclass(module = "collomatique", frozen)]
pub struct Slot {
    doc: Py<Document>,
    id: RawSlotId,
}

impl Handle for Slot {
    type IdClass = SlotId;

    const CLASS: &'static str = "Slot";
    const NOUN: &'static str = "slot";

    fn mint(doc: Py<Document>, id: RawSlotId) -> Slot {
        Slot { doc, id }
    }

    fn document(&self) -> &Py<Document> {
        &self.doc
    }

    fn raw_id(&self) -> RawSlotId {
        self.id
    }

    fn exists(data: &InnerData, id: RawSlotId) -> bool {
        data.params.slots.find_slot(id).is_some()
    }
}

#[pymethods]
impl Slot {
    /// The slot's id
    ///
    /// The one attribute that works on a stale handle: it reads nothing.
    #[getter]
    fn id(&self) -> SlotId {
        SlotId::wrap(self.id)
    }

    /// The slot's position among its subject's slots, 0-based
    ///
    /// The position inside the subject, and not in the walk `doc.slots` makes:
    /// the subject's list is the only order the model keeps for slots, so it is
    /// the only one a number can name.
    #[getter]
    fn index(&self, py: Python<'_>) -> PyResult<usize> {
        self.read(py, |data| {
            data.params
                .slots
                .find_slot_subject_and_position(self.id)
                .map(|(_subject_id, position)| position)
        })
    }

    /// The subject this slot belongs to
    ///
    /// Fixed when the slot is created: the model files the slot under this
    /// subject in the list that gives it its position, so an update that changed
    /// the subject would leave the slot in the wrong list, and `SlotOp::Update`
    /// refuses one (`state-colloscopes/src/slots.rs`).
    #[getter]
    fn subject(&self, py: Python<'_>) -> PyResult<Subject> {
        let subject_id = self.read(py, |data| {
            data.params
                .slots
                .find_slot(self.id)
                .map(|slot| slot.subject_id)
        })?;
        Ok(Subject::mint(self.doc.clone_ref(py), subject_id))
    }

    /// The teacher who runs this slot's colles
    #[getter]
    fn teacher(&self, py: Python<'_>) -> PyResult<Teacher> {
        let teacher_id = self.read(py, |data| {
            data.params
                .slots
                .find_slot(self.id)
                .map(|slot| slot.teacher_id)
        })?;
        Ok(Teacher::mint(self.doc.clone_ref(py), teacher_id))
    }

    /// The day of the week this slot runs on, as a [Weekday]
    #[getter]
    fn weekday(&self, py: Python<'_>) -> PyResult<Weekday> {
        self.read(py, |data| {
            data.params
                .slots
                .find_slot(self.id)
                .map(|slot| Weekday::from_model(slot.start_time.weekday))
        })
    }

    /// The time of day this slot starts, as a `datetime.time`
    ///
    /// Whole minutes: the model stores the time with minute precision, so the
    /// seconds and the microseconds are always zero.
    #[getter]
    fn start_time(&self, py: Python<'_>) -> PyResult<chrono::NaiveTime> {
        self.read(py, |data| {
            data.params
                .slots
                .find_slot(self.id)
                .map(|slot| *slot.start_time.start_time.inner())
        })
    }

    /// What the export prints beside this slot — a room number, and the like
    ///
    /// A plain string, the empty one included: the model types this field as a
    /// `String` and python mirrors it rather than editorializing.
    #[getter]
    fn extra_info(&self, py: Python<'_>) -> PyResult<String> {
        self.read(py, |data| {
            data.params
                .slots
                .find_slot(self.id)
                .map(|slot| slot.extra_info.clone())
        })
    }

    /// The pattern saying which weeks this slot runs on, or `None`
    ///
    /// `None` means every week — the slot has no pattern of its own, so only the
    /// weeks' own flags switch it off. Which weeks are left is
    /// `doc.is_week_active`, and whether a colle can really happen there is
    /// `doc.is_interrogation_possible`.
    #[getter]
    fn week_pattern(&self, py: Python<'_>) -> PyResult<Option<WeekPattern>> {
        let pattern_id = self.read(py, |data| {
            data.params
                .slots
                .find_slot(self.id)
                .map(|slot| slot.week_pattern)
        })?;

        Ok(pattern_id.map(|pattern_id| WeekPattern::mint(self.doc.clone_ref(py), pattern_id)))
    }

    /// What using this slot costs the solver
    ///
    /// Zero by default. A positive cost tells the solver to avoid the slot, a
    /// negative one to favour it.
    #[getter]
    fn cost(&self, py: Python<'_>) -> PyResult<i32> {
        self.read(py, |data| {
            data.params.slots.find_slot(self.id).map(|slot| slot.cost)
        })
    }

    /// Whether two handles name the same slot of the same document
    ///
    /// Never reads the state, so it keeps working once the slot is gone — a dict
    /// holding handles must not blow up when an entity dies.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<Slot>() {
            Ok(other) => self.same_as(other.get()),
            Err(_) => false,
        }
    }

    fn __hash__(&self) -> u64 {
        self.hash_key()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        // The day is the model's own capitalized french name — « Jeudi » — the
        // same word the application displays.
        let start = self.peek(py, |data| {
            data.params.slots.find_slot(self.id).map(|slot| {
                format!(
                    "{} {}",
                    slot.start_time.weekday.capitalize(),
                    slot.start_time.start_time.inner().format("%H:%M"),
                )
            })
        });
        self.repr_text(start)
    }
}
