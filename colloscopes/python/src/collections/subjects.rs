//! The subjects of a document, and how their interrogations are laid out
//!
//! Reached as `doc.subjects`. A subject carries a name, the periods it does not
//! run in, and — when it holds interrogations at all — a whole set of parameters
//! for them, which is the [Interrogation] sub-view.
//!
//! The slots those colles happen in belong to the subject too, but the model
//! keeps them in a table of their own, so they live in
//! [crate::collections::slots] and are reached here as `subject.slots`.
//!
//! Written through `add`, `update`, `remove`, `move_up`, `move_down` and
//! `set_period_status`. This is the most referenced entity in the document —
//! eight kinds of place name a subject — so it also has the heaviest ordinary
//! removal: the teachers who hold its colles lose it, and their slots in it go
//! with them; its incompatibilities, the pairing rules relating it to another
//! subject, its balancing options, its enrolment rows and its group-list
//! associations all go too. Every one of those repairs comes back on the
//! `OpResult`. The `update` cascades in its own two ways — switching the
//! interrogations off dismantles everything that needed them, and lengthening
//! one over a slot too late in the day takes that slot — and
//! `set_period_status(…, False)` drops what the subject held on the period it
//! leaves: the enrolments, the colles already written on that period's weeks,
//! and the group list it used there.
//!
//! The value is larger than what the ops carry, the second family where that
//! happens: a `SubjectData` holds the excluded periods, and no subject op
//! does — the model keeps them beside the parameters, which the ops carry, and
//! beside the week pattern, which they carry too. So rather than dropping the
//! field on the floor, `add`
//! refuses a value that excludes anything and `update` refuses one whose
//! exclusions differ from the document's, both naming `set_period_status`,
//! which is the op that moves them. A read-modify-write never meets the second
//! refusal: `to_data()` fills the field with the subject's own exclusions.
//!
//! The family keeps two refusals for the model, and both reach a script as
//! `SubjectsError`: a subject at either end of the list has nowhere left to
//! move. What the model could otherwise object to is caught above the write,
//! where the message can say which argument was wrong — a dead subject or a
//! dead period is the argument convention's business
//! ([crate::handles::argument]).

use std::collections::BTreeSet;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyFrozenSet, PyTuple};

use collomatique_ops::{SubjectsUpdateOp, UpdateOp};
use collomatique_state_colloscopes::PeriodId as RawPeriodId;
use collomatique_state_colloscopes::SubjectId as RawSubjectId;
use collomatique_state_colloscopes::{InnerData, NewId, SubjectInterrogationParameters};

use crate::Document;
use crate::collections::periods::Period;
use crate::collections::slots::Slot;
use crate::data::{SubjectData, Value as _};
use crate::errors::StaleHandleError;
use crate::handles::{Handle, argument, handle_iterator, named, no_such, quoted};
use crate::ids::{IdClass, PeriodId, SubjectId};
use crate::results::{AddResult, OpResult};
use crate::values;

/// The subjects of one document, in user order
///
/// Frozen and holding nothing but the document: it is a view, so two of them on
/// the same document are interchangeable and neither can go stale.
#[pyclass(module = "collomatique", frozen)]
pub struct Subjects {
    doc: Py<Document>,
}

impl Subjects {
    /// Builds the view — `doc.subjects` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> Subjects {
        Subjects { doc }
    }

    /// Reads the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }

    /// The subject an id or a handle names, when this document still holds it
    fn resolve(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<RawSubjectId> {
        let id = named::<Subject>(&self.doc, key)?;
        self.with_data(py, |data| Subject::exists(data, id))
            .then_some(id)
    }
}

#[pymethods]
impl Subjects {
    /// How many subjects the document holds
    fn __len__(&self, py: Python<'_>) -> usize {
        self.with_data(py, |data| data.params.subjects.ordered_subject_list.len())
    }

    /// The subjects, as handles, in user order
    fn __iter__(&self, py: Python<'_>) -> SubjectIter {
        let ids = self.with_data(py, |data| {
            data.params.subjects.ordered_subject_list.keys().collect()
        });
        SubjectIter::new(self.doc.clone_ref(py), ids)
    }

    /// The subject an id or a handle names
    ///
    /// Raises `KeyError` when it names nothing in this document — including for
    /// a handle bound to another document, whatever its id says.
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Subject> {
        let id = self
            .resolve(py, key)
            .ok_or_else(|| no_such("subject", key))?;
        Ok(Subject::mint(self.doc.clone_ref(py), id))
    }

    /// The subject an id or a handle names, or `None`
    fn get(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<Subject> {
        let id = self.resolve(py, key)?;
        Some(Subject::mint(self.doc.clone_ref(py), id))
    }

    fn __contains__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> bool {
        self.resolve(py, key).is_some()
    }

    /// Adds a subject, and hands back the handle of the new one
    ///
    /// Takes a `SubjectData` and answers an `AddResult`, whose `created` is the
    /// `Subject` the document just minted. The new subject lands last in the
    /// list, which is where the application puts one too.
    ///
    /// ```python
    /// doc.subjects.add(collomatique.SubjectData("Spé maths"))
    /// doc.subjects.add(collomatique.SubjectData("Quidditch", interrogation=None))
    /// ```
    ///
    /// The exclusions are the one field the op cannot carry, and the mirror
    /// says so rather than discarding them: a value that excludes a period is a
    /// `ValueError`. A new subject runs on every period the document holds, and
    /// taking it off one is `set_period_status` — so a subject that skips a
    /// period is two calls, which a transaction makes one undo step.
    ///
    /// Nothing in the document can name a subject that does not exist yet, so
    /// there is nothing for the cascade to repair: the answer's `warnings` is
    /// empty.
    fn add(&self, py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Py<AddResult>> {
        // Extracted before the mutable borrow, never inside it: a value naming
        // an entity is resolved against this document, which borrows it to ask.
        let subject = SubjectData::from_py(&self.doc, data)?;

        if !subject.excluded_periods.is_empty() {
            return Err(PyValueError::new_err(format!(
                "no subject op carries the excluded periods, and a new subject runs on every \
                 period: that SubjectData excludes {}. Add the subject and take it off those \
                 periods with doc.subjects.set_period_status(subject, period, False).",
                periods(&subject.excluded_periods),
            )));
        }

        crate::results::created::<Subject>(
            py,
            &self.doc,
            UpdateOp::Subjects(SubjectsUpdateOp::AddNewSubject(
                subject.parameters,
                subject.week_pattern,
            )),
            |new_id| match new_id {
                NewId::SubjectId(id) => Some(id),
                _ => None,
            },
        )
    }

    /// Rewrites a subject whole, its excluded periods excepted
    ///
    /// The op carries the whole of the rest, so this replaces every other field
    /// at once: what the `SubjectData` says is what the subject becomes, the
    /// name and the interrogation parameters together. The id stays, the
    /// position stays, and so does every handle naming it.
    ///
    /// The exclusions are the one field the op cannot carry, and the mirror
    /// says so rather than discarding them: a value whose `excluded_periods`
    /// differ from what the document holds for this subject is a `ValueError`
    /// naming `set_period_status`, which is the op that moves them. A
    /// read-modify-write never meets that refusal, since `to_data()` fills the
    /// field with the subject's own exclusions.
    ///
    /// Both of the family's cascades live here. Setting `interrogation` to
    /// `None` is a write like any other, and the repairs dismantle what needed
    /// those colles: the teachers who held them lose the subject, their slots
    /// in it go, its group-list associations go and so do its own balancing
    /// options and the pairing rules naming it. Lengthening an interrogation
    /// over a slot that would then run past midnight takes that slot. The
    /// enrolments deliberately survive both: being registered in a subject says
    /// nothing about having colles in it.
    ///
    /// The subject is resolved before the value is read, so a call that is wrong
    /// about both says which subject it could not find rather than what was
    /// wrong with a value meant for nothing.
    fn update(
        &self,
        py: Python<'_>,
        key: &Bound<'_, PyAny>,
        data: &Bound<'_, PyAny>,
    ) -> PyResult<OpResult> {
        let id = argument::<Subject>(&self.doc, key)?;

        // Read here rather than after the value: the argument convention has
        // just found the subject and nothing has called into python since, so
        // the subject is still there. `from_py` below runs python code — a
        // dataclass is a python object — and the document could be written to
        // under it.
        let (excluded, week_pattern) = self
            .with_data(py, |data| {
                data.params
                    .subjects
                    .find_subject(id)
                    .map(|subject| (subject.excluded_periods.clone(), subject.week_pattern))
            })
            .expect("the argument convention has just found this subject");

        let subject = SubjectData::from_py(&self.doc, data)?;
        if subject.excluded_periods != excluded {
            return Err(PyValueError::new_err(format!(
                "no subject op carries the excluded periods: this subject skips {}, and that \
                 SubjectData names {}. Move one with \
                 doc.subjects.set_period_status(subject, period, active) instead.",
                periods(&excluded),
                periods(&subject.excluded_periods),
            )));
        }

        // The week pattern is the op's, but no `SubjectData` field says which
        // one yet, so the subject keeps the one it has: a write through this
        // mirror never moves it.
        self.write(
            py,
            UpdateOp::Subjects(SubjectsUpdateOp::UpdateSubject(
                id,
                subject.parameters,
                week_pattern,
            )),
        )
    }

    /// Removes a subject
    ///
    /// The heaviest ordinary removal the document has, because a subject is
    /// what everything else is about: the teachers who hold its colles lose it,
    /// and their slots in it go with them; its incompatibilities go, and the
    /// pairing rules relating it to another subject; its own balancing options,
    /// its enrolment rows and its group-list associations go too, and the colles
    /// standing in its slots go with the slots. The `OpResult` carries every
    /// repair, each one linked to the one that needed it. Handles naming the
    /// subject go stale, and so do the ones naming what went with it.
    fn remove(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<OpResult> {
        let id = argument::<Subject>(&self.doc, key)?;

        self.write(py, UpdateOp::Subjects(SubjectsUpdateOp::DeleteSubject(id)))
    }

    /// Moves a subject one place up the list
    ///
    /// The list order is the one the application shows, and the one
    /// `doc.subjects` walks in, so this swaps the subject with the one before it
    /// there. Nothing else moves: a position is display order, and nothing in
    /// the document reads one.
    ///
    /// A subject already first has nowhere to go, and that is a `SubjectsError`
    /// rather than a call that quietly did nothing.
    fn move_up(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<OpResult> {
        let id = argument::<Subject>(&self.doc, key)?;

        self.write(py, UpdateOp::Subjects(SubjectsUpdateOp::MoveSubjectUp(id)))
    }

    /// Moves a subject one place down the list
    ///
    /// The twin of [Subjects::move_up], and it refuses in the same way: a
    /// subject already last has nowhere to go, and says so with a
    /// `SubjectsError`.
    fn move_down(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<OpResult> {
        let id = argument::<Subject>(&self.doc, key)?;

        self.write(
            py,
            UpdateOp::Subjects(SubjectsUpdateOp::MoveSubjectDown(id)),
        )
    }

    /// Puts a subject on a period, or takes it off
    ///
    /// The one op that moves `SubjectData.excluded_periods`, one period at a
    /// time and the other way round: `active=True` means the subject runs on
    /// the period, which is the exclusion *gone*.
    ///
    /// ```python
    /// doc.subjects.set_period_status(maths, first_period, False)
    /// ```
    ///
    /// Taking a subject off a period drops the three things it held there, and
    /// the `OpResult` says which: the enrolments of that row, the colles already
    /// written on the period's weeks, and the group list the subject used there.
    /// Putting it back only ever widens what the document allows, so there is
    /// nothing to repair.
    #[pyo3(signature = (subject, period, active))]
    fn set_period_status(
        &self,
        py: Python<'_>,
        subject: &Bound<'_, PyAny>,
        period: &Bound<'_, PyAny>,
        active: bool,
    ) -> PyResult<OpResult> {
        let subject_id = argument::<Subject>(&self.doc, subject)?;
        let period_id = argument::<Period>(&self.doc, period)?;

        self.write(
            py,
            UpdateOp::Subjects(SubjectsUpdateOp::UpdatePeriodStatus(
                subject_id, period_id, active,
            )),
        )
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!("<collomatique.Subjects count={}>", self.__len__(py))
    }
}

impl Subjects {
    /// Writes through the document the view came from
    ///
    /// The five mutators that create nothing end here. The creating one ends in
    /// [crate::results::created], which takes the same borrow and keeps the id
    /// the op issued as well.
    fn write(&self, py: Python<'_>, op: UpdateOp) -> PyResult<OpResult> {
        let mut doc = self.doc.borrow_mut(py);
        doc.update(py, op)
    }
}

/// The periods a refusal names, as its message shows them
///
/// Ids and not handles: the two refusals above are about a value, and a value
/// names periods by id. The spelling is the id class's own `<PeriodId 3>`, so
/// what the message shows is what a script printing the value would see.
fn periods(ids: &BTreeSet<RawPeriodId>) -> String {
    if ids.is_empty() {
        return "no period".into();
    }

    ids.iter()
        .map(|id| PeriodId::text(*id))
        .collect::<Vec<_>>()
        .join(", ")
}

handle_iterator! {
    /// The subjects of a collection, minted as the loop asks for them
    SubjectIter yielding Subject
}

/// One subject of the document
///
/// A live view: every attribute reads the document as it stands now. Reading one
/// whose subject has been removed raises `StaleHandleError`; `.id`, `==` and
/// `hash` keep working, since they never touch the state.
#[pyclass(module = "collomatique", frozen)]
pub struct Subject {
    doc: Py<Document>,
    id: RawSubjectId,
}

impl Handle for Subject {
    type IdClass = SubjectId;

    const CLASS: &'static str = "Subject";
    const NOUN: &'static str = "subject";

    fn mint(doc: Py<Document>, id: RawSubjectId) -> Subject {
        Subject { doc, id }
    }

    fn document(&self) -> &Py<Document> {
        &self.doc
    }

    fn raw_id(&self) -> RawSubjectId {
        self.id
    }

    fn exists(data: &InnerData, id: RawSubjectId) -> bool {
        data.params.subjects.find_subject_position(id).is_some()
    }
}

#[pymethods]
impl Subject {
    /// The subject's id
    ///
    /// The one attribute that works on a stale handle: it reads nothing.
    #[getter]
    fn id(&self) -> SubjectId {
        SubjectId::wrap(self.id)
    }

    /// The subject's display position, 0-based
    #[getter]
    fn index(&self, py: Python<'_>) -> PyResult<usize> {
        self.read(py, |data| {
            data.params.subjects.find_subject_position(self.id)
        })
    }

    /// The subject's name
    ///
    /// A plain string, the empty one included: the model types this field as a
    /// `String` and python mirrors it rather than editorializing.
    #[getter]
    fn name(&self, py: Python<'_>) -> PyResult<String> {
        self.read(py, |data| {
            data.params
                .subjects
                .find_subject(self.id)
                .map(|subject| subject.parameters.name.clone())
        })
    }

    /// How this subject's interrogations are laid out, or `None`
    ///
    /// `None` means the subject holds no interrogations at all — the Quidditch
    /// practice that sits in the timetable without ever being a colle. What
    /// comes back otherwise is a live sub-view: asking again after the
    /// interrogations were switched off answers `None`, and the view handed out
    /// before that goes stale.
    #[getter]
    fn interrogation(&self, py: Python<'_>) -> PyResult<Option<Interrogation>> {
        let holds_them = self.read(py, |data| {
            let subject = data.params.subjects.find_subject(self.id)?;
            Some(subject.parameters.interrogation_parameters.is_some())
        })?;

        Ok(holds_them.then(|| Interrogation::mint(self.doc.clone_ref(py), self.id)))
    }

    /// The periods this subject does not run in, as a `frozenset` of [Period]
    ///
    /// A snapshot, built when it is asked for: the set does not grow when the
    /// document does. The handles in it stay live.
    #[getter]
    fn excluded_periods<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyFrozenSet>> {
        let ids = self.read(py, |data| {
            let subject = data.params.subjects.find_subject(self.id)?;
            Some(subject.excluded_periods.iter().copied().collect::<Vec<_>>())
        })?;

        let periods: Vec<_> = ids
            .into_iter()
            .map(|period_id| Period::mint(self.doc.clone_ref(py), period_id))
            .collect();
        PyFrozenSet::new(py, periods)
    }

    /// This subject's slots, as a tuple of [Slot], in their order
    ///
    /// The order is the subject's own, which is the only one the model keeps for
    /// slots: `slot.index` is the position in this tuple. A subject with no
    /// slots reads as an empty tuple, colles or no colles.
    ///
    /// A snapshot, built when it is asked for. The handles in it stay live.
    #[getter]
    fn slots<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let ids = self.read(py, |data| {
            // Asked first, and on its own: the slot table has no row for a
            // subject without slots, so "no row" must not be read as "no
            // subject" — one is an empty tuple and the other is staleness.
            data.params.subjects.find_subject(self.id)?;
            Some(
                data.params
                    .slots
                    .slots_for_subject(self.id)
                    .into_iter()
                    .flatten()
                    .map(|(slot_id, _slot)| *slot_id)
                    .collect::<Vec<_>>(),
            )
        })?;

        let slots: Vec<_> = ids
            .into_iter()
            .map(|slot_id| Slot::mint(self.doc.clone_ref(py), slot_id))
            .collect();
        PyTuple::new(py, slots)
    }

    /// What points at this subject — every site whose coordinates name it, as a
    /// tuple of `RefSite` values, in the registry's walk order. An empty tuple
    /// means nothing points here.
    ///
    /// A stale handle raises `StaleHandleError` like every other read.
    fn referenced_by(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        crate::refs::subject_references(py, self)
    }

    /// This subject, detached — a `SubjectData` holding what the handle shows
    ///
    /// A fresh object every call, and a whole one: the interrogation parameters
    /// come out as an `InterrogationData` of their own — or as `None` for a
    /// subject that holds no colles — and the excluded periods as `PeriodId`s,
    /// because a value holding handles would carry this document around with
    /// it.
    ///
    /// The exclusions are in the value although no subject op carries them:
    /// what `to_data()` hands back is the subject, whole, which is what makes
    /// `doc.snapshot()` buildable out of these classes.
    ///
    /// A stale handle raises `StaleHandleError` like every other read.
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        use crate::data::Value as _;

        // Copied out of the borrow before anything python-facing happens:
        // building the value calls into python, and doing that under the
        // document's borrow is how a nested borrow becomes a `PanicException`.
        let subject = self.read(py, |data| {
            data.params.subjects.find_subject(self.id).cloned()
        })?;

        crate::data::SubjectData::to_py(py, &subject)
    }

    /// Whether two handles name the same subject of the same document
    ///
    /// Never reads the state, so it keeps working once the subject is gone — a
    /// dict holding handles must not blow up when an entity dies.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<Subject>() {
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
                .subjects
                .find_subject(self.id)
                .map(|subject| subject.parameters.name.clone())
        });
        self.repr_text(name.map(|name| quoted(py, &name)))
    }
}

/// How one subject's interrogations are laid out
///
/// A sub-view, which is a handle in everything but the `.id`: it is bound to
/// its subject, reads the
/// current state on every access, and goes stale with it. `subject.interrogation`
/// asked again always answers the current truth.
///
/// It goes stale in two ways, and both mean the same thing — what the view was
/// viewing is gone: the subject was removed, or its interrogations were switched
/// off.
///
/// The number its repr shows is the subject's, since that is what the view is
/// bound to.
#[pyclass(module = "collomatique", frozen)]
pub struct Interrogation {
    doc: Py<Document>,
    id: RawSubjectId,
}

impl Handle for Interrogation {
    type IdClass = SubjectId;

    const CLASS: &'static str = "Interrogation";
    const NOUN: &'static str = "subject";

    fn mint(doc: Py<Document>, id: RawSubjectId) -> Interrogation {
        Interrogation { doc, id }
    }

    fn document(&self) -> &Py<Document> {
        &self.doc
    }

    fn raw_id(&self) -> RawSubjectId {
        self.id
    }

    /// Whether the subject this view is about still holds interrogations
    ///
    /// What the view *views*, and not merely whether the subject is there: the
    /// two are the sub-view's two ways of dying, and both mean it has nothing
    /// left to read. [Interrogation::read] is what tells them apart, since a
    /// script meeting the error wants to know which happened.
    fn exists(data: &InnerData, id: RawSubjectId) -> bool {
        data.params
            .subjects
            .find_subject(id)
            .is_some_and(|subject| subject.parameters.interrogation_parameters.is_some())
    }
}

impl Interrogation {
    /// Borrows the document, finds the parameters the view is about, and reads
    ///
    /// The two ways of being stale are told apart here rather than folded into
    /// one message: a subject that was removed and a subject that stopped
    /// holding colles are different things to have done, and a script reading
    /// the error wants to know which.
    fn read<R>(
        &self,
        py: Python<'_>,
        f: impl FnOnce(&SubjectInterrogationParameters) -> R,
    ) -> PyResult<R> {
        let doc = self.doc.borrow(py);
        let subjects = &doc.data().get_inner_data().params.subjects;

        let subject = subjects
            .find_subject(self.id)
            .ok_or_else(|| <Interrogation as Handle>::stale(self))?;

        let params = subject
            .parameters
            .interrogation_parameters
            .as_ref()
            .ok_or_else(|| {
                StaleHandleError::new_err(format!(
                    "this Interrogation view is stale: subject {} no longer holds interrogations",
                    SubjectId::text(self.id),
                ))
            })?;

        Ok(f(params))
    }
}

#[pymethods]
impl Interrogation {
    /// How many students one group holds, as a `(min, max)` range
    #[getter]
    fn students_per_group(&self, py: Python<'_>) -> PyResult<values::Range> {
        self.read(py, |params| {
            values::nonzero_range(&params.students_per_group)
        })
    }

    /// How many groups sit one interrogation together, as a `(min, max)` range
    #[getter]
    fn groups_per_interrogation(&self, py: Python<'_>) -> PyResult<values::Range> {
        self.read(py, |params| {
            values::nonzero_range(&params.groups_per_interrogation)
        })
    }

    /// How long one interrogation lasts, in minutes
    #[getter]
    fn duration(&self, py: Python<'_>) -> PyResult<u32> {
        self.read(py, |params| params.duration.get().get())
    }

    /// Whether this time counts against the limits on a student's week
    #[getter]
    fn take_duration_into_account(&self, py: Python<'_>) -> PyResult<bool> {
        self.read(py, |params| params.take_duration_into_account)
    }

    /// How often the interrogations come round, as one of the [Periodicity]
    /// values
    ///
    /// [Periodicity]: crate::values::Periodicity
    #[getter]
    fn periodicity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let periodicity = self.read(py, |params| params.periodicity.clone())?;
        values::periodicity(py, &periodicity)
    }

    /// These parameters, detached — an `InterrogationData` holding what the
    /// view shows
    ///
    /// A fresh object every call. The view's two ways of dying are the two ways
    /// this raises `StaleHandleError`, each with its own message: the subject
    /// was removed, or it stopped holding colles.
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        use crate::data::Value as _;

        // Out of the borrow first, for the same reason as everywhere else.
        let params = self.read(py, |params| params.clone())?;

        crate::data::InterrogationData::to_py(py, &params)
    }

    /// Whether two views are about the same subject of the same document
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<Interrogation>() {
            Ok(other) => self.same_as(other.get()),
            Err(_) => false,
        }
    }

    fn __hash__(&self) -> u64 {
        self.hash_key()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let duration = self.read(py, |params| params.duration.get().get()).ok();
        self.repr_text(duration.map(|duration| format!("duration={duration}")))
    }
}
