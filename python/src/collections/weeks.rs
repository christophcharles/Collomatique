//! The weeks of a document
//!
//! Reached as `doc.weeks`, and as `period.weeks` for the weeks of one period.
//! A week belongs to a period, carries whether interrogations happen on it and
//! an optional annotation, and — when the document has a start date — falls on
//! a datable monday.
//!
//! Written through `set_status` and `set_annotation`, which are the two things
//! a week says about itself. There is no `add` and no `remove` here: a week is
//! created and destroyed by what happens to its period, so
//! [crate::collections::periods] holds those — `add`, `set_week_count`,
//! `remove_with_weeks` and `cut`.
//!
//! The model's own week ops address a week as a period and a position within
//! it. The surface takes the `Week` handle instead — a script names a week the
//! way it reads one — and the mutators translate, which is also why neither of
//! them can meet the two refusals those ops carry: an unknown period and a
//! position past the end are both about coordinates the mutator read off the
//! document itself. What is left is the argument convention, where a week this
//! document does not hold is caught before the op is built
//! ([crate::handles::argument]).

use chrono::{Days, NaiveDate};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyTuple};

use collomatique_ops::{GeneralPlanningUpdateOp, UpdateOp};
use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::PeriodId as RawPeriodId;
use collomatique_state_colloscopes::WeekId as RawWeekId;

use crate::Document;
use crate::collections::periods::Period;
use crate::errors::Error;
use crate::handles::{Handle, argument, handle_iterator, named, no_such};
use crate::ids::{IdClass, WeekId};
use crate::results::OpResult;

/// The weeks of one document, in global week order
///
/// Frozen and holding nothing but the document: it is a view, so two of them on
/// the same document are interchangeable and neither can go stale.
///
/// The order is the model's own week walk — period display order, then position
/// within the period — so `week.index` is the position a week has here.
#[pyclass(module = "collomatique", frozen)]
pub struct Weeks {
    doc: Py<Document>,
}

impl Weeks {
    /// Builds the view — `doc.weeks` is the only way to get one
    pub(crate) fn new(doc: Py<Document>) -> Weeks {
        Weeks { doc }
    }

    /// Reads the document behind the view
    fn with_data<R>(&self, py: Python<'_>, f: impl FnOnce(&InnerData) -> R) -> R {
        let doc = self.doc.borrow(py);
        f(doc.data().get_inner_data())
    }

    /// The week an id or a handle names, when this document still holds it
    fn resolve(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<RawWeekId> {
        let id = named::<Week>(&self.doc, key)?;
        self.with_data(py, |data| Week::exists(data, id))
            .then_some(id)
    }

    /// The `(period, position in that period)` pair the week ops address a week
    /// by
    ///
    /// The translation the two mutators share. The argument convention has just
    /// found the week and nothing has called into python since, so the document
    /// still holds it and it still has a position.
    fn coordinates(&self, py: Python<'_>, id: RawWeekId) -> (RawPeriodId, usize) {
        self.with_data(py, |data| data.params.weeks.week_position(id))
            .expect("the argument convention has just found this week")
    }

    /// Writes through the document the view came from
    ///
    /// Neither mutator creates anything — a week is created by what happens to
    /// its period — so neither needs [crate::results::created]'s second half.
    fn write(&self, py: Python<'_>, op: UpdateOp) -> PyResult<OpResult> {
        let mut doc = self.doc.borrow_mut(py);
        doc.update(py, op)
    }
}

#[pymethods]
impl Weeks {
    /// How many weeks the document holds, across every period
    fn __len__(&self, py: Python<'_>) -> usize {
        self.with_data(py, |data| data.params.weeks.count_weeks())
    }

    /// The weeks, as handles, in global week order
    fn __iter__(&self, py: Python<'_>) -> WeekIter {
        let ids = self.with_data(py, |data| data.params.week_ids().collect());
        WeekIter::new(self.doc.clone_ref(py), ids)
    }

    /// The week an id or a handle names
    ///
    /// Raises `KeyError` when it names nothing in this document — including for
    /// a handle bound to another document, whatever its id says.
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Week> {
        let id = self.resolve(py, key).ok_or_else(|| no_such("week", key))?;
        Ok(Week::mint(self.doc.clone_ref(py), id))
    }

    /// The week an id or a handle names, or `None`
    fn get(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> Option<Week> {
        let id = self.resolve(py, key)?;
        Some(Week::mint(self.doc.clone_ref(py), id))
    }

    fn __contains__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> bool {
        self.resolve(py, key).is_some()
    }

    /// Switches the colles of one week on or off
    ///
    /// A week that holds no interrogations is a week of the year the colles
    /// skip — a holiday, the week of a mock exam — and it still exists, still
    /// counts in the order, and still has its date. This is the flag
    /// `week.interrogations` reads.
    ///
    /// ```python
    /// doc.weeks.set_status(week, False)
    /// ```
    ///
    /// Switching the colles off takes the ones already written on that week:
    /// they cannot stand on a week that holds none, and the `OpResult` says
    /// which went. Switching them back on only ever widens what the document
    /// allows, so there is nothing to repair — and nothing comes back either:
    /// the colles a week lost are gone, and turning it on again does not
    /// remember them. Undo does.
    fn set_status(
        &self,
        py: Python<'_>,
        week: &Bound<'_, PyAny>,
        active: bool,
    ) -> PyResult<OpResult> {
        let id = argument::<Week>(&self.doc, week)?;
        let (period_id, position) = self.coordinates(py, id);

        self.write(
            py,
            UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::UpdateWeekStatus(
                period_id, position, active,
            )),
        )
    }

    /// Annotates one week, or clears its annotation
    ///
    /// « Rentrée », « Vacances de Noël » — free text the application shows
    /// beside the week, and `None` clears it. Nothing in the document reads an
    /// annotation, so this repairs nothing and its `warnings` is always empty.
    ///
    /// ```python
    /// doc.weeks.set_annotation(week, "Vacances")
    /// doc.weeks.set_annotation(week, None)
    /// ```
    ///
    /// The empty string is refused with a `ValueError` rather than taken as a
    /// clear: the model types the field as an optional non-empty string, and a
    /// week that says nothing is `None` here as it is everywhere else in this
    /// api.
    #[pyo3(signature = (week, annotation))]
    fn set_annotation(
        &self,
        py: Python<'_>,
        week: &Bound<'_, PyAny>,
        annotation: Option<String>,
    ) -> PyResult<OpResult> {
        let id = argument::<Week>(&self.doc, week)?;
        let (period_id, position) = self.coordinates(py, id);

        // Written out rather than through a `?` on a `TryFrom`: the model's
        // string type is foreign — it comes from `non_empty_string` — and this
        // crate names it nowhere, so what fixes the type is the op field the
        // value lands in.
        let annotation = match annotation {
            None => None,
            Some(text) => Some(text.try_into().map_err(|_| {
                PyValueError::new_err(
                    "a week's annotation is a non-empty string or None, and '' is neither",
                )
            })?),
        };

        self.write(
            py,
            UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::UpdateWeekAnnotation(
                period_id, position, annotation,
            )),
        )
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!("<collomatique.Weeks count={}>", self.__len__(py))
    }
}

handle_iterator! {
    /// The weeks of a collection, minted as the loop asks for them
    WeekIter yielding Week
}

/// One week of the document
///
/// A live view: every attribute reads the document as it stands now. Reading
/// one whose week has been removed raises `StaleHandleError`; `.id`, `==` and
/// `hash` keep working, since they never touch the state.
#[pyclass(module = "collomatique", frozen)]
pub struct Week {
    doc: Py<Document>,
    id: RawWeekId,
}

impl Handle for Week {
    type IdClass = WeekId;

    const CLASS: &'static str = "Week";
    const NOUN: &'static str = "week";

    fn mint(doc: Py<Document>, id: RawWeekId) -> Week {
        Week { doc, id }
    }

    fn document(&self) -> &Py<Document> {
        &self.doc
    }

    fn raw_id(&self) -> RawWeekId {
        self.id
    }

    fn exists(data: &InnerData, id: RawWeekId) -> bool {
        data.params.weeks.find_week(id).is_some()
    }
}

#[pymethods]
impl Week {
    /// The week's id
    ///
    /// The one attribute that works on a stale handle: it reads nothing.
    #[getter]
    fn id(&self) -> WeekId {
        WeekId::wrap(self.id)
    }

    /// The period this week belongs to
    #[getter]
    fn period(&self, py: Python<'_>) -> PyResult<Period> {
        let period_id = self.read(py, |data| {
            data.params
                .weeks
                .find_week(self.id)
                .map(|week| week.period_id)
        })?;
        Ok(Period::mint(self.doc.clone_ref(py), period_id))
    }

    /// The week's position in global week order, 0-based across all periods
    #[getter]
    fn index(&self, py: Python<'_>) -> PyResult<usize> {
        self.read(py, |data| {
            data.params
                .weeks
                .global_week_position(&data.params.periods, self.id)
        })
    }

    /// Whether the week holds interrogations at all
    #[getter]
    fn interrogations(&self, py: Python<'_>) -> PyResult<bool> {
        self.read(py, |data| {
            data.params
                .weeks
                .find_week(self.id)
                .map(|week| week.interrogations)
        })
    }

    /// The week's annotation — « Rentrée », « Vacances » — or `None`
    ///
    /// `None` and not `""`: the model types this field as an optional non-empty
    /// string, and python mirrors it rather than editorializing.
    #[getter]
    fn annotation(&self, py: Python<'_>) -> PyResult<Option<String>> {
        self.read(py, |data| {
            data.params
                .weeks
                .find_week(self.id)
                .map(|week| week.annotation.as_ref().map(|text| text.to_string()))
        })
    }

    /// The monday this week falls on, as a `datetime.date`, or `None`
    ///
    /// `None` means the document has no start date: the week still exists and
    /// still has its order, it just has no date to show. Otherwise the weeks run
    /// consecutively from the start date, in global order — the way the xlsx
    /// export dates them (`xlsx/src/lib.rs`, `generate_week_dates_title`).
    #[getter]
    fn monday(&self, py: Python<'_>) -> PyResult<Option<NaiveDate>> {
        let dated = self.read(py, |data| {
            let index = data
                .params
                .weeks
                .global_week_position(&data.params.periods, self.id)?;
            let first_week = data
                .params
                .periods
                .first_week
                .as_ref()
                .map(|week| *week.monday());
            Some((first_week, index))
        })?;

        let (Some(first_week), index) = dated else {
            return Ok(None);
        };

        // The arm is written out rather than unwrapped because §6 of the design
        // says a script never gets a panic. Reaching it takes a document with
        // some hundred million weeks in it, so nothing about it is expected.
        first_week
            .checked_add_days(Days::new(7 * index as u64))
            .map(Some)
            .ok_or_else(|| {
                Error::new_err(format!(
                    "week {index} of this document falls past the last date there is"
                ))
            })
    }

    /// What points at this week — every site whose coordinates name it, as a
    /// tuple of `RefSite` values, in the registry's walk order. An empty tuple
    /// means nothing points here.
    ///
    /// A stale handle raises `StaleHandleError` like every other read.
    fn referenced_by(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        crate::refs::week_references(py, self)
    }

    /// This week, detached — a `WeekData` holding what the handle shows
    ///
    /// A fresh object every call: two calls give two values that compare equal
    /// and share nothing. The period comes out as a `PeriodId` rather than as
    /// a handle, because a value holding handles would carry this document
    /// around with it and keep it alive.
    ///
    /// A stale handle raises `StaleHandleError` like every other read.
    fn to_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        use crate::data::Value as _;

        // Copied out of the borrow before anything python-facing happens:
        // building the value calls into python, and doing that under the
        // document's borrow is how a nested borrow becomes a `PanicException`.
        let week = self.read(py, |data| data.params.weeks.find_week(self.id).cloned())?;

        crate::data::WeekData::to_py(py, &week)
    }

    /// Whether two handles name the same week of the same document
    ///
    /// Never reads the state, so it keeps working once the week is gone — a
    /// dict holding handles must not blow up when an entity dies.
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<Week>() {
            Ok(other) => self.same_as(other.get()),
            Err(_) => false,
        }
    }

    fn __hash__(&self) -> u64 {
        self.hash_key()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let index = self.peek(py, |data| {
            data.params
                .weeks
                .global_week_position(&data.params.periods, self.id)
        });
        self.repr_text(index.map(|index| format!("index={index}")))
    }
}
