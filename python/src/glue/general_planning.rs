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
    pub weeks_status: Vec<bool>,
}

#[pymethods]
impl Period {
    #[new]
    fn new(id: PeriodId, weeks_status: Vec<bool>) -> Self {
        Period { id, weeks_status }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColloscopePeriodId {
    id: collomatique_state_colloscopes::ColloscopePeriodId,
}

#[pymethods]
impl ColloscopePeriodId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&collomatique_state_colloscopes::ColloscopePeriodId> for ColloscopePeriodId {
    fn from(value: &collomatique_state_colloscopes::ColloscopePeriodId) -> Self {
        ColloscopePeriodId { id: value.clone() }
    }
}

impl From<collomatique_state_colloscopes::ColloscopePeriodId> for ColloscopePeriodId {
    fn from(value: collomatique_state_colloscopes::ColloscopePeriodId) -> Self {
        ColloscopePeriodId::from(&value)
    }
}

impl From<&ColloscopePeriodId> for collomatique_state_colloscopes::ColloscopePeriodId {
    fn from(value: &ColloscopePeriodId) -> Self {
        value.id.clone()
    }
}

impl From<ColloscopePeriodId> for collomatique_state_colloscopes::ColloscopePeriodId {
    fn from(value: ColloscopePeriodId) -> Self {
        collomatique_state_colloscopes::ColloscopePeriodId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopePeriod {
    #[pyo3(set, get)]
    pub id: ColloscopePeriodId,
    #[pyo3(set, get)]
    pub weeks_status: Vec<bool>,
}

#[pymethods]
impl ColloscopePeriod {
    #[new]
    fn new(id: ColloscopePeriodId, weeks_status: Vec<bool>) -> Self {
        ColloscopePeriod { id, weeks_status }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}
