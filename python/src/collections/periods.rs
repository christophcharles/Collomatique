//! The periods of a document, and the date the colles start
//!
//! Reached as `doc.periods`. A period owns nothing but its existence and its
//! display order in the model, so the handle is pure navigation: its index, and
//! the weeks that belong to it.

use chrono::{Datelike, NaiveDate};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyTuple};

use collomatique_ops::{GeneralPlanningUpdateOp, UpdateOp};
use collomatique_state_colloscopes::InnerData;
use collomatique_state_colloscopes::PeriodId as RawPeriodId;
use collomatique_time::WeekStart;

use crate::Document;
use crate::collections::weeks::Week;
use crate::handles::{Handle, handle_iterator, named, no_such};
use crate::ids::{IdClass, PeriodId};
use crate::results::OpResult;

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
        self.with_data(py, |data| {
            data.params.periods.find_period_position(id).is_some()
        })
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

        self.update(
            py,
            UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::UpdateFirstWeek(week)),
        )
    }

    /// Clears the start date
    ///
    /// The weeks stay; they stop having dates. Clearing a document that had no
    /// start date is not an error — it is already what was asked for.
    fn clear_first_week(&self, py: Python<'_>) -> PyResult<OpResult> {
        self.update(
            py,
            UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::DeleteFirstWeek),
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
    fn update(&self, py: Python<'_>, op: UpdateOp) -> PyResult<OpResult> {
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
