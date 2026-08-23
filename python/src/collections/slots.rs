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
//!
//! Written through `add`, `update`, `remove`, `move_up` and `move_down`. The
//! last two are the family's own pair: a slot has a position inside its
//! subject, so it has a way of moving that no other family needs.
//!
//! This is the first family whose value is larger than what its ops carry
//! (`docs/python/new_api_design.md` §2): a `SlotData` names its subject, and a
//! slot cannot change subject — the model files it under that subject in the
//! very list that gives it its position. So `add` *reads* the field, since
//! `AddNewSlot` takes the subject beside the slot payload, and `update` refuses
//! a value naming a different subject than the slot's own rather than dropping
//! the field on the floor. A read-modify-write never meets that refusal:
//! `to_data()` fills the field with the slot's own subject.
//!
//! Removing a slot takes what stood in it: the colloscope cells written on it,
//! and the slot pairing rules that related it to another slot. Every one of
//! those repairs comes back on the `OpResult`. The `update` cascades too, and in
//! one way: putting the slot on a pattern that switches a week off clears the
//! colles already written on that week, since the slot no longer runs there.
//!
//! The family keeps four refusals for the model, and each reaches a script as
//! `SlotsError`: a subject that holds no interrogations has no colles for a slot
//! to carry, a teacher must be declared in the subject they are given a slot in,
//! a colle that would run past midnight is refused rather than spilling into the
//! next day, and a slot at either end of its subject's list has nowhere left to
//! move. What the model could otherwise object to is caught on this side, where
//! the message can say which argument was wrong: a dead slot is the argument
//! convention's business ([crate::handles::argument]), and a dead subject,
//! teacher or week pattern is the value boundary's.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyTuple};

use collomatique_ops::{SlotsUpdateOp, UpdateOp};
use collomatique_state_colloscopes::SlotId as RawSlotId;
use collomatique_state_colloscopes::{InnerData, NewId};

use crate::Document;
use crate::collections::subjects::Subject;
use crate::collections::teachers::Teacher;
use crate::collections::week_patterns::WeekPattern;
use crate::data::{SlotData, Value as _};
use crate::handles::{Handle, argument, handle_iterator, named, no_such};
use crate::ids::{IdClass, SlotId, SubjectId};
use crate::results::{AddResult, OpResult};
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

    /// Adds a slot, and hands back the handle of the new one
    ///
    /// Takes a `SlotData` and answers an `AddResult`, whose `created` is the
    /// `Slot` the document just minted. The subject comes off the value:
    /// `AddNewSlot` takes it beside the slot payload, and the value is where a
    /// script has already written it down. The new slot lands last among that
    /// subject's slots, which is where the application puts one too.
    ///
    /// ```python
    /// doc.slots.add(collomatique.SlotData(
    ///     maths, snape, collomatique.Weekday.THURSDAY, datetime.time(14, 0)))
    /// ```
    ///
    /// Three of the family's four model refusals are reachable here, and each
    /// arrives as a `SlotsError`: a subject that runs no interrogations has no
    /// colle for a slot to carry, a teacher must be declared in the subject
    /// they are given a slot in, and a colle that would run past midnight is
    /// refused rather than spilling into the next day. All three are statements
    /// about the document rather than about the value, which is why
    /// [crate::data::SlotData] leaves them to the write.
    ///
    /// A new slot holds no colle and is related to nothing, so there is nothing
    /// for the cascade to repair: the answer's `warnings` is empty.
    fn add(&self, py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Py<AddResult>> {
        // Extracted before the mutable borrow, never inside it: a value naming
        // an entity is resolved against this document, which borrows it to ask
        // (`docs/python/new_api_design.md` §5).
        let slot = SlotData::from_py(&self.doc, data)?;

        crate::results::created::<Slot>(
            py,
            &self.doc,
            UpdateOp::Slots(SlotsUpdateOp::AddNewSlot(slot.subject_id, slot)),
            |new_id| match new_id {
                NewId::SlotId(id) => Some(id),
                _ => None,
            },
        )
    }

    /// Rewrites a slot whole, its subject excepted
    ///
    /// The op carries the whole value, so this replaces every other field at
    /// once: what the `SlotData` says is what the slot becomes, the teacher, the
    /// day, the time, the room, the pattern and the cost together. The id stays,
    /// the position stays, and so does every handle naming it.
    ///
    /// The subject is the one field the op cannot move, and the mirror says so
    /// rather than discarding it: a value naming a different subject than the
    /// slot's own is a `ValueError`. What a script means by that is a slot in
    /// the other subject, which is an `add` and a `remove`. A read-modify-write
    /// never meets the refusal, since `to_data()` fills the field with the
    /// slot's own subject.
    ///
    /// Putting the slot on a pattern that switches a week off is a write like
    /// any other, and the cascade repairs what it broke: the slot no longer runs
    /// on that week, so the colles already written there go, and the warnings
    /// say so.
    ///
    /// The slot is resolved before the value is read, so a call that is wrong
    /// about both says which slot it could not find rather than what was wrong
    /// with a value meant for nothing.
    fn update(
        &self,
        py: Python<'_>,
        key: &Bound<'_, PyAny>,
        data: &Bound<'_, PyAny>,
    ) -> PyResult<OpResult> {
        let id = argument::<Slot>(&self.doc, key)?;

        // Read here rather than after the value: the argument convention has
        // just found the slot and nothing has called into python since, so the
        // slot is still there. `from_py` below runs python code — a dataclass is
        // a python object — and the document could be written to under it.
        let subject_id = self
            .with_data(py, |data| {
                data.params.slots.find_slot(id).map(|slot| slot.subject_id)
            })
            .expect("the argument convention has just found this slot");

        let slot = SlotData::from_py(&self.doc, data)?;
        if slot.subject_id != subject_id {
            return Err(PyValueError::new_err(format!(
                "a slot cannot change subject: this one is {}'s, and that SlotData names {}. \
                 Add a slot to the other subject and remove this one instead.",
                SubjectId::text(subject_id),
                SubjectId::text(slot.subject_id),
            )));
        }

        self.write(py, UpdateOp::Slots(SlotsUpdateOp::UpdateSlot(id, slot)))
    }

    /// Removes a slot
    ///
    /// The slot goes and what stood in it goes with it: the colloscope cells
    /// written on it, since there is no slot left to hold those colles, and the
    /// slot pairing rules that related it to another slot, since a rule with one
    /// end missing relates nothing. The `OpResult` carries every repair. Handles
    /// naming the slot go stale, and so do the ones naming the rules that went.
    fn remove(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<OpResult> {
        let id = argument::<Slot>(&self.doc, key)?;

        self.write(py, UpdateOp::Slots(SlotsUpdateOp::DeleteSlot(id)))
    }

    /// Moves a slot one place up its subject's list
    ///
    /// The position is the one inside the subject — the only order the model
    /// keeps for slots — so this swaps the slot with the one before it there,
    /// and `doc.slots` walks the two in the new order afterwards. Nothing else
    /// moves: a position is display order, and no colle, rule or reference reads
    /// it.
    ///
    /// A slot already first has nowhere to go, and that is a `SlotsError` rather
    /// than a call that quietly did nothing.
    fn move_up(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<OpResult> {
        let id = argument::<Slot>(&self.doc, key)?;

        self.write(py, UpdateOp::Slots(SlotsUpdateOp::MoveSlotUp(id)))
    }

    /// Moves a slot one place down its subject's list
    ///
    /// The twin of [Slots::move_up], and it refuses in the same way: a slot
    /// already last has nowhere to go, and says so with a `SlotsError`.
    fn move_down(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<OpResult> {
        let id = argument::<Slot>(&self.doc, key)?;

        self.write(py, UpdateOp::Slots(SlotsUpdateOp::MoveSlotDown(id)))
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!("<collomatique.Slots count={}>", self.__len__(py))
    }
}

impl Slots {
    /// Writes through the document the view came from
    ///
    /// The four mutators that create nothing end here. The creating one ends in
    /// [crate::results::created], which takes the same borrow and keeps the id
    /// the op issued as well.
    fn write(&self, py: Python<'_>, op: UpdateOp) -> PyResult<OpResult> {
        let mut doc = self.doc.borrow_mut(py);
        doc.update(py, op)
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

    /// What points at this slot — every site whose coordinates name it, as a
    /// tuple of `RefSite` values, in the registry's walk order. An empty tuple
    /// means nothing points here.
    ///
    /// A stale handle raises `StaleHandleError` like every other read.
    fn referenced_by(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        crate::refs::slot_references(py, self)
    }

    /// This slot, detached — a `SlotData` holding what the handle shows
    ///
    /// A fresh object every call. The subject, the teacher and the pattern come
    /// out as ids rather than as handles, because a value holding handles would
    /// carry this document around with it and keep it alive.
    ///
    /// The subject is in the value although no slot op really carries it: what
    /// `to_data()` hands back is the slot, whole, which is what makes
    /// `doc.snapshot()` buildable out of these classes (§2.0). It also means a
    /// read-modify-write never trips over the field that cannot change —
    /// `slot.to_data()` fills it with this slot's own subject.
    ///
    /// A stale handle raises `StaleHandleError` like every other read.
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        use crate::data::Value as _;

        // Copied out of the borrow before anything python-facing happens:
        // building the value calls into python, and doing that under the
        // document's borrow is how a nested borrow becomes a `PanicException`.
        let slot = self.read(py, |data| data.params.slots.find_slot(self.id).cloned())?;

        crate::data::SlotData::to_py(py, &slot)
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
