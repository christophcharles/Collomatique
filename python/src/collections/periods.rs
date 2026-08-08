//! The periods of a document, and the date the colles start
//!
//! Reached as `doc.periods`. Only the whole-planning part is here for now:
//! everything that names one period — adding, cutting, merging — takes a period
//! id, which is the read surface's job (`docs/python/new_api_design.md` §2).

use chrono::{Datelike, NaiveDate};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use collomatique_ops::{GeneralPlanningUpdateOp, UpdateOp};
use collomatique_state_colloscopes::InnerData;
use collomatique_time::WeekStart;

use crate::Document;
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
}

#[pymethods]
impl Periods {
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
