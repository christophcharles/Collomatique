//! The periods of a document, and the date the colles start
//!
//! Reached as `doc.periods`. A period owns nothing but its existence and its
//! display order in the model, so the handle is pure navigation: its index, and
//! the weeks that belong to it.
//!
//! Written through `set_first_week` and `clear_first_week` for the start date,
//! and through `add`, `set_week_count`, `remove_with_weeks`, `cut` and
//! `merge_with_previous` for the periods themselves. A period is nothing but
//! its weeks and its place in the year, so every one of those five is really a
//! statement about weeks: they create them, drop them from the end of a period,
//! or hand them over to another period. That is what makes this the heaviest
//! cascade source the document has — a week that goes takes the colles written
//! on it and frees the week patterns that skipped it, and a period that goes
//! takes its assignment rows, its group-list associations and the exclusion
//! sets naming it. Every one of those repairs comes back on the `OpResult`.
//!
//! There is no `update` here and no `remove`: a period holds no value to
//! rewrite — [crate::collections::weeks] holds what a week says — and taking
//! one away is never only about the period, which is what
//! `remove_with_weeks` spells out in its own name.
//!
//! The family keeps two refusals for the model, and both reach a script as
//! `GeneralPlanningError`: a cut cannot keep more weeks than the period holds,
//! and the first period has no previous one to merge with. A period this
//! document does not hold is caught above the write, where the message can say
//! which argument was wrong ([crate::handles::argument]).

use chrono::{Datelike, NaiveDate};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyTuple};

use collomatique_ops::{GeneralPlanningUpdateOp, UpdateOp};
use collomatique_state_colloscopes::PeriodId as RawPeriodId;
use collomatique_state_colloscopes::{InnerData, NewId};
use collomatique_time::WeekStart;

use crate::Document;
use crate::collections::weeks::Week;
use crate::handles::{Handle, argument, handle_iterator, named, no_such};
use crate::ids::{IdClass, PeriodId};
use crate::results::{AddResult, OpResult};

/// The periods of one document
///
/// Frozen and holding nothing but the document: it is a view, so two of them
/// on the same document are interchangeable, and a script can keep one around
/// without it going stale.
#[pyclass(module = "collomatique", frozen)]
pub struct Periods {
    doc: Py<Document>,
}

impl Periods {
    /// Builds the view — `doc.periods` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> Periods {
        Periods { doc }
    }

    /// Reads the general-planning part of the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }

    /// The period an id or a handle names, when this document still holds it
    fn resolve(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<RawPeriodId> {
        let id = named::<Period>(&self.doc, key)?;
        self.with_data(py, |data| Period::exists(data, id))
            .then_some(id)
    }
}

#[pymethods]
impl Periods {
    /// How many periods the document holds
    fn __len__(&self, py: Python<'_>) -> usize {
        self.with_data(py, |data| data.params.periods.period_count())
    }

    /// The periods, as handles, in display order
    fn __iter__(&self, py: Python<'_>) -> PeriodIter {
        let ids = self.with_data(py, |data| data.params.periods.period_ids().collect());
        PeriodIter::new(self.doc.clone_ref(py), ids)
    }

    /// The period an id or a handle names
    ///
    /// Raises `KeyError` when it names nothing in this document — including for
    /// a handle bound to another document, whatever its id says.
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Period> {
        let id = self
            .resolve(py, key)
            .ok_or_else(|| no_such("period", key))?;
        Ok(Period::mint(self.doc.clone_ref(py), id))
    }

    /// The period an id or a handle names, or `None`
    fn get(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<Period> {
        let id = self.resolve(py, key)?;
        Some(Period::mint(self.doc.clone_ref(py), id))
    }

    fn __contains__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> bool {
        self.resolve(py, key).is_some()
    }

    /// The monday the colles start, as a `datetime.date`, or `None`
    ///
    /// `None` means the document has no start date. The weeks still exist and
    /// still have their order — they just have no dates to show.
    #[getter]
    fn first_week(&self, py: Python<'_>) -> Option<NaiveDate> {
        self.with_data(py, |data| {
            data.params
                .periods
                .first_week
                .as_ref()
                .map(|week| *week.monday())
        })
    }

    /// Sets the monday the colles start
    ///
    /// The argument is a `datetime.date` and it must be a monday: the document
    /// counts its weeks from one, and quietly moving a wednesday back to "its"
    /// monday would be a guess the script never sees. A date that is not a
    /// monday raises `ValueError`.
    ///
    /// ```python
    /// import datetime
    /// doc.periods.set_first_week(datetime.date(2026, 9, 7))
    /// ```
    fn set_first_week(&self, py: Python<'_>, date: NaiveDate) -> PyResult<OpResult> {
        let week = WeekStart::new(date).ok_or_else(|| {
            PyValueError::new_err(format!(
                "the colles start on a monday, and {date} is a {}",
                date.weekday()
            ))
        })?;

        self.write(
            py,
            UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::UpdateFirstWeek(week)),
        )
    }

    /// Clears the start date
    ///
    /// The weeks stay; they stop having dates. Clearing a document that had no
    /// start date is not an error — it is already what was asked for.
    fn clear_first_week(&self, py: Python<'_>) -> PyResult<OpResult> {
        self.write(
            py,
            UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::DeleteFirstWeek),
        )
    }

    /// Adds a period at the end of the year, and hands back its handle
    ///
    /// Takes the number of weeks the new period holds and answers an
    /// `AddResult`, whose `created` is the `Period` the document just minted.
    /// The weeks are created with it: every one of them holds interrogations
    /// and none is annotated, which is the shape the application's own "add a
    /// period" gives them. A period with no week at all is legal too — `add(0)`
    /// is the model's canonical empty period, not a refusal.
    ///
    /// ```python
    /// autumn = doc.periods.add(6).created
    /// ```
    ///
    /// The new period lands last, which is the only place one can be added: the
    /// list is the year in order, and a period inserted in the middle would be
    /// a cut of the period it lands in ([Periods::cut]).
    ///
    /// Nothing in the document can name a period that does not exist yet, and
    /// its weeks are as new as it is, so there is nothing for the cascade to
    /// repair: the answer's `warnings` is empty.
    fn add(&self, py: Python<'_>, week_count: usize) -> PyResult<Py<AddResult>> {
        crate::results::created::<Period>(
            py,
            &self.doc,
            UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::AddNewPeriod(week_count)),
            |new_id| match new_id {
                NewId::PeriodId(id) => Some(id),
                _ => None,
            },
        )
    }

    /// Gives a period exactly `week_count` weeks, growing or shrinking its end
    ///
    /// The front of the period never moves: the weeks it already holds keep
    /// their order, their annotations and their colles, and only the end of it
    /// changes. Asking for the count it already has writes nothing new — it is
    /// still a write, and still its own undo slot.
    ///
    /// ```python
    /// doc.periods.set_week_count(autumn, 8)
    /// ```
    ///
    /// A period that grows repeats its last week: the new weeks are copies of
    /// it, annotation included, since that is what the application's own
    /// week-count spinner produces. A period with no week to copy grows weeks
    /// that hold interrogations and say nothing.
    ///
    /// A period that shrinks loses the weeks off its end, and they take what
    /// stood on them: the colles written there go, and the week patterns that
    /// skipped those weeks stop naming them. The `OpResult` says which. Handles
    /// naming the dropped weeks go stale.
    fn set_week_count(
        &self,
        py: Python<'_>,
        period: &Bound<'_, PyAny>,
        week_count: usize,
    ) -> PyResult<OpResult> {
        let id = argument::<Period>(&self.doc, period)?;

        self.write(
            py,
            UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::UpdatePeriodWeekCount(
                id, week_count,
            )),
        )
    }

    /// Removes a period, and every week in it
    ///
    /// The name says the whole of it: a period is its weeks, so there is no
    /// removal that keeps them, and the api spells that out rather than
    /// offering a `remove` that quietly takes twenty weeks with it. The weeks
    /// go because the call asked for them, so they are not themselves repairs
    /// and no warning names them — what the `OpResult` carries is what *they*
    /// broke, and what the period itself was holding: the colles written on
    /// those weeks, the week patterns that skipped them, the period's
    /// assignment rows, the group lists its subjects used there, and the
    /// subjects, students and rules that excluded it.
    ///
    /// Handles naming the period go stale, and so do the ones naming its weeks.
    fn remove_with_weeks(&self, py: Python<'_>, period: &Bound<'_, PyAny>) -> PyResult<OpResult> {
        let id = argument::<Period>(&self.doc, period)?;

        self.write(
            py,
            UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::DeletePeriodAndWeeks(id)),
        )
    }

    /// Cuts a period in two, keeping `remaining` weeks in it
    ///
    /// The weeks past `remaining` move, in order, into a brand new period that
    /// lands right after this one — so the year keeps every week it had, and in
    /// the order it had them. This creates a period, so it answers an
    /// `AddResult` like [Periods::add] does, and its `created` is the tail: the
    /// new period is what a script wants next, and looking it up by index would
    /// be the one thing it cannot read off the call.
    ///
    /// ```python
    /// spring = doc.periods.cut(winter, 4).created
    /// ```
    ///
    /// The moved weeks keep their colles. The tail period is given what the cut
    /// one held — the subjects and students that skipped it skip the tail too,
    /// its assignment rows and its group-list associations are copied — before
    /// the first week moves, precisely so that a colle is as legal at its new
    /// coordinate as it was at the old one. So an ordinary cut repairs nothing
    /// and its `warnings` is empty.
    ///
    /// Keeping every week the period has is a legal cut: the tail is then an
    /// empty period, which is a period like any other. Keeping *more* weeks
    /// than it holds is not, and the model refuses it with a
    /// `GeneralPlanningError` naming both counts.
    fn cut(
        &self,
        py: Python<'_>,
        period: &Bound<'_, PyAny>,
        remaining: usize,
    ) -> PyResult<Py<AddResult>> {
        let id = argument::<Period>(&self.doc, period)?;

        crate::results::created::<Period>(
            py,
            &self.doc,
            UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::CutPeriod(id, remaining)),
            |new_id| match new_id {
                NewId::PeriodId(id) => Some(id),
                _ => None,
            },
        )
    }

    /// Merges a period into the one before it
    ///
    /// The undoing of a cut, and the neighbour is the previous period because
    /// that is what keeps the year in order: this period's weeks are appended
    /// to it, in their own order, and the emptied period goes. No week moves in
    /// the global order, so nothing a week pattern says changes meaning.
    ///
    /// A colle survives the move unless what the surviving period says about
    /// its subject makes it illegal there — the subject may not run on it, or
    /// the group list it uses there may have fewer groups than the colle names
    /// — and the `OpResult` carries exactly those repairs. What was keyed on
    /// the period that goes is dropped: silently where the surviving period
    /// says the very same thing, since the moved weeks then read exactly as
    /// they did, and as a repair where the two disagree.
    ///
    /// The first period has nothing before it, and asking anyway is refused
    /// with a `GeneralPlanningError` rather than quietly doing nothing.
    fn merge_with_previous(&self, py: Python<'_>, period: &Bound<'_, PyAny>) -> PyResult<OpResult> {
        let id = argument::<Period>(&self.doc, period)?;

        self.write(
            py,
            UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::MergeWithPreviousPeriod(id)),
        )
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        match self.first_week(py) {
            Some(date) => format!("<collomatique.Periods first_week={date}>"),
            None => "<collomatique.Periods first_week=None>".to_owned(),
        }
    }
}

impl Periods {
    /// Writes through the document the view came from
    ///
    /// The mutators that create nothing end here. The two that create a period
    /// — the `add` and the `cut` that hands the tail weeks to a new one — end
    /// in [crate::results::created], which takes the same borrow and keeps the
    /// id the op issued as well.
    fn write(&self, py: Python<'_>, op: UpdateOp) -> PyResult<OpResult> {
        let mut doc = self.doc.borrow_mut(py);
        doc.update(py, op)
    }
}

handle_iterator! {
    /// The periods of a collection, minted as the loop asks for them
    PeriodIter yielding Period
}

/// One period of the document
///
/// A live view: every attribute reads the document as it stands now. Reading
/// one whose period has been removed raises `StaleHandleError`; `.id`, `==` and
/// `hash` keep working, since they never touch the state.
///
/// A period carries no data of its own in the model — it owns its existence and
/// its place in the display order, and the weeks name it as their own. So the
/// handle is pure navigation.
#[pyclass(module = "collomatique", frozen)]
pub struct Period {
    doc: Py<Document>,
    id: RawPeriodId,
}

impl Handle for Period {
    type IdClass = PeriodId;

    const CLASS: &'static str = "Period";
    const NOUN: &'static str = "period";

    fn mint(doc: Py<Document>, id: RawPeriodId) -> Period {
        Period { doc, id }
    }

    fn document(&self) -> &Py<Document> {
        &self.doc
    }

    fn raw_id(&self) -> RawPeriodId {
        self.id
    }

    fn exists(data: &InnerData, id: RawPeriodId) -> bool {
        data.params.periods.find_period_position(id).is_some()
    }
}

#[pymethods]
impl Period {
    /// The period's id
    ///
    /// The one attribute that works on a stale handle: it reads nothing.
    #[getter]
    fn id(&self) -> PeriodId {
        PeriodId::wrap(self.id)
    }

    /// The period's display position, 0-based
    #[getter]
    fn index(&self, py: Python<'_>) -> PyResult<usize> {
        self.read(py, |data| data.params.periods.find_period_position(self.id))
    }

    /// The period's weeks, in order
    ///
    /// A snapshot, built when it is asked for: the tuple does not grow when the
    /// document does. The handles in it stay live.
    #[getter]
    fn weeks<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let ids = self.read(py, |data| {
            // A period with no week has no ordering row at all, which is the
            // model's canonical form and not a missing period: the existence
            // question is asked separately, and only it means staleness.
            data.params.periods.find_period_position(self.id)?;
            Some(
                data.params
                    .weeks
                    .weeks_for_period(self.id)
                    .into_iter()
                    .flatten()
                    .map(|(week_id, _week)| *week_id)
                    .collect::<Vec<_>>(),
            )
        })?;

        PyTuple::new(
            py,
            ids.into_iter()
                .map(|week_id| Week::mint(self.doc.clone_ref(py), week_id)),
        )
    }

    /// What points at this period — every site whose coordinates name it, as a
    /// tuple of `RefSite` values, in the registry's walk order. An empty tuple
    /// means nothing points here.
    ///
    /// The one reverse door of the read surface: it is the question to ask
    /// before a remove. A stale handle raises `StaleHandleError` like every
    /// other read.
    fn referenced_by(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        crate::refs::period_references(py, self)
    }

    /// Whether two handles name the same period of the same document
    ///
    /// Never reads the state, so it keeps working once the period is gone — a
    /// dict holding handles must not blow up when an entity dies.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<Period>() {
            Ok(other) => self.same_as(other.get()),
            Err(_) => false,
        }
    }

    fn __hash__(&self) -> u64 {
        self.hash_key()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let index = self.peek(py, |data| data.params.periods.find_period_position(self.id));
        self.repr_text(index.map(|index| format!("index={index}")))
    }
}
