//! The weeks of a document
//!
//! Reached as `doc.weeks`, and as `period.weeks` for the weeks of one period.
//! A week belongs to a period, carries whether interrogations happen on it and
//! an optional annotation, and — when the document has a start date — falls on
//! a datable monday.

use chrono::{Days, NaiveDate};
use pyo3::prelude::*;
use pyo3::types::PyAny;

use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::WeekId as RawWeekId;

use crate::Document;
use crate::collections::periods::Period;
use crate::errors::Error;
use crate::handles::{Handle, handle_iterator, named, no_such};
use crate::ids::{IdClass, WeekId};

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
