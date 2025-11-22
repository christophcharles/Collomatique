use super::*;
use pyo3::types::PyString;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeriodId {
    id: collomatique_state_colloscopes::PeriodId,
}

#[pymethods]
impl PeriodId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::PeriodId> for PeriodId {
    fn from(value: &collomatique_state_colloscopes::PeriodId) -> Self {
        PeriodId { id: value.clone() }
    }
}

impl From<collomatique_state_colloscopes::PeriodId> for PeriodId {
    fn from(value: collomatique_state_colloscopes::PeriodId) -> Self {
        PeriodId::from(&value)
    }
}

impl From<&PeriodId> for collomatique_state_colloscopes::PeriodId {
    fn from(value: &PeriodId) -> Self {
        value.id.clone()
    }
}

impl From<PeriodId> for collomatique_state_colloscopes::PeriodId {
    fn from(value: PeriodId) -> Self {
        collomatique_state_colloscopes::PeriodId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Period {
    #[pyo3(set, get)]
    pub id: PeriodId,
    #[pyo3(set, get)]
    pub weeks_status: Vec<WeekDesc>,
}

#[pymethods]
impl Period {
    #[new]
    fn new(id: PeriodId, weeks_status: Vec<WeekDesc>) -> Self {
        Period { id, weeks_status }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeekDesc {
    #[pyo3(set, get)]
    pub interrogations: bool,
    #[pyo3(set, get)]
    pub annotation: String,
}

#[pymethods]
impl WeekDesc {
    #[new]
    fn new(interrogations: bool) -> Self {
        WeekDesc {
            interrogations,
            annotation: String::new(),
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<WeekDesc> for collomatique_state_colloscopes::periods::WeekDesc {
    fn from(value: WeekDesc) -> collomatique_state_colloscopes::periods::WeekDesc {
        collomatique_state_colloscopes::periods::WeekDesc {
            interrogations: value.interrogations,
            annotation: non_empty_string::NonEmptyString::new(value.annotation).ok(),
        }
    }
}

impl From<collomatique_state_colloscopes::periods::WeekDesc> for WeekDesc {
    fn from(value: collomatique_state_colloscopes::periods::WeekDesc) -> WeekDesc {
        WeekDesc {
            interrogations: value.interrogations,
            annotation: value.annotation.map(|x| x.into_inner()).unwrap_or_default(),
        }
    }
}
