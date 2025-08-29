use super::*;

use pyo3::types::PyString;

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
}

impl From<collomatique_time::NaiveMondayDate> for NaiveMondayDate {
    fn from(value: collomatique_time::NaiveMondayDate) -> Self {
        NaiveMondayDate { internal: value }
    }
}
