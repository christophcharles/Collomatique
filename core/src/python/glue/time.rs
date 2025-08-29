use super::*;

use pyo3::{exceptions::PyValueError, types::PyString};

#[derive(Clone, Debug, PartialEq, Eq)]
#[pyclass]
pub struct NaiveMondayDate {
    internal: collomatique_time::NaiveMondayDate,
}

#[pymethods]
impl NaiveMondayDate {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{}", self_.internal.format("%Y-%m-%d"));
        PyString::new(self_.py(), output.as_str())
    }

    #[new]
    fn new(year: i32, month: u32, day: u32) -> PyResult<Self> {
        let Some(date) = chrono::NaiveDate::from_ymd_opt(year, month, day) else {
            return Err(PyValueError::new_err(format!("Invalid date")));
        };

        let Some(internal) = collomatique_time::NaiveMondayDate::new(date) else {
            return Err(PyValueError::new_err(format!("Not a monday")));
        };

        Ok(NaiveMondayDate { internal })
    }
}

impl From<collomatique_time::NaiveMondayDate> for NaiveMondayDate {
    fn from(value: collomatique_time::NaiveMondayDate) -> Self {
        NaiveMondayDate { internal: value }
    }
}

impl From<NaiveMondayDate> for collomatique_time::NaiveMondayDate {
    fn from(value: NaiveMondayDate) -> Self {
        value.internal
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[pyclass]
pub struct NaiveDate {
    internal: chrono::NaiveDate,
}

#[pymethods]
impl NaiveDate {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{}", self_.internal.format("%Y-%m-%d"));
        PyString::new(self_.py(), output.as_str())
    }

    #[new]
    fn new(year: i32, month: u32, day: u32) -> PyResult<Self> {
        let Some(internal) = chrono::NaiveDate::from_ymd_opt(year, month, day) else {
            return Err(PyValueError::new_err(format!("Invalid date")));
        };

        Ok(NaiveDate { internal })
    }

    fn round_to_week(self_: PyRef<'_, Self>) -> NaiveMondayDate {
        NaiveMondayDate {
            internal: collomatique_time::NaiveMondayDate::round_from(self_.internal),
        }
    }
}

impl From<chrono::NaiveDate> for NaiveDate {
    fn from(value: chrono::NaiveDate) -> Self {
        NaiveDate { internal: value }
    }
}

impl From<NaiveDate> for chrono::NaiveDate {
    fn from(value: NaiveDate) -> Self {
        value.internal
    }
}
