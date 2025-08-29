use super::*;
use pyo3::types::PyString;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WeekPatternId {
    id: crate::rpc::cmd_msg::MsgWeekPatternId,
}

#[pymethods]
impl WeekPatternId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&crate::rpc::cmd_msg::MsgWeekPatternId> for WeekPatternId {
    fn from(value: &crate::rpc::cmd_msg::MsgWeekPatternId) -> Self {
        WeekPatternId { id: value.clone() }
    }
}

impl From<crate::rpc::cmd_msg::MsgWeekPatternId> for WeekPatternId {
    fn from(value: crate::rpc::cmd_msg::MsgWeekPatternId) -> Self {
        WeekPatternId::from(&value)
    }
}

impl From<&WeekPatternId> for crate::rpc::cmd_msg::MsgWeekPatternId {
    fn from(value: &WeekPatternId) -> Self {
        value.id.clone()
    }
}

impl From<WeekPatternId> for crate::rpc::cmd_msg::MsgWeekPatternId {
    fn from(value: WeekPatternId) -> Self {
        crate::rpc::cmd_msg::MsgWeekPatternId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeekPattern {
    #[pyo3(set, get)]
    pub name: String,
    #[pyo3(set, get)]
    pub weeks: Vec<bool>,
}

#[pymethods]
impl WeekPattern {
    #[new]
    fn new(name: String) -> Self {
        WeekPattern {
            name,
            weeks: vec![],
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::week_patterns::WeekPattern> for WeekPattern {
    fn from(value: collomatique_state_colloscopes::week_patterns::WeekPattern) -> Self {
        WeekPattern {
            name: value.name,
            weeks: value.weeks,
        }
    }
}

impl From<WeekPattern> for crate::rpc::cmd_msg::week_patterns::WeekPatternMsg {
    fn from(value: WeekPattern) -> Self {
        use crate::rpc::cmd_msg::week_patterns::WeekPatternMsg;
        WeekPatternMsg {
            name: value.name,
            weeks: value.weeks,
        }
    }
}
