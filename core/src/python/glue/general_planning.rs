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

impl From<&PeriodId> for crate::rpc::cmd_msg::MsgPeriodId {
    fn from(value: &PeriodId) -> Self {
        value.id.clone().into()
    }
}

impl From<PeriodId> for crate::rpc::cmd_msg::MsgPeriodId {
    fn from(value: PeriodId) -> Self {
        crate::rpc::cmd_msg::MsgPeriodId::from(&value)
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
