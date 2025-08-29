use std::num::NonZeroU32;

use super::*;
use pyo3::types::PyString;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IncompatId {
    id: crate::rpc::cmd_msg::MsgIncompatId,
}

#[pymethods]
impl IncompatId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&crate::rpc::cmd_msg::MsgIncompatId> for IncompatId {
    fn from(value: &crate::rpc::cmd_msg::MsgIncompatId) -> Self {
        IncompatId { id: value.clone() }
    }
}

impl From<crate::rpc::cmd_msg::MsgIncompatId> for IncompatId {
    fn from(value: crate::rpc::cmd_msg::MsgIncompatId) -> Self {
        IncompatId::from(&value)
    }
}

impl From<&IncompatId> for crate::rpc::cmd_msg::MsgIncompatId {
    fn from(value: &IncompatId) -> Self {
        value.id.clone()
    }
}

impl From<IncompatId> for crate::rpc::cmd_msg::MsgIncompatId {
    fn from(value: IncompatId) -> Self {
        crate::rpc::cmd_msg::MsgIncompatId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incompat {
    #[pyo3(set, get)]
    pub subject_id: SubjectId,
    #[pyo3(set, get)]
    pub start_time: time::SlotStart,
    #[pyo3(set, get)]
    pub duration_in_minutes: NonZeroU32,
    #[pyo3(set, get)]
    pub week_pattern_id: Option<WeekPatternId>,
}

#[pymethods]
impl Incompat {
    #[new]
    fn new(
        subject_id: SubjectId,
        start_time: time::SlotStart,
        duration_in_minutes: NonZeroU32,
    ) -> Self {
        Incompat {
            subject_id,
            start_time,
            duration_in_minutes,
            week_pattern_id: None,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::incompats::Incompatibility> for Incompat {
    fn from(value: collomatique_state_colloscopes::incompats::Incompatibility) -> Self {
        Incompat {
            subject_id: MsgSubjectId::from(value.subject_id).into(),
            start_time: value.slot.start().clone().into(),
            duration_in_minutes: value.slot.duration().get(),
            week_pattern_id: value
                .week_pattern_id
                .map(|x| MsgWeekPatternId::from(x).into()),
        }
    }
}

impl From<Incompat> for crate::rpc::cmd_msg::incompatibilities::IncompatMsg {
    fn from(value: Incompat) -> Self {
        use crate::rpc::cmd_msg::incompatibilities::IncompatMsg;
        IncompatMsg {
            subject_id: MsgSubjectId::from(value.subject_id),
            start_day: collomatique_time::Weekday::from(value.start_time.weekday).into_inner(),
            start_time: value.start_time.start_time.into(),
            duration: value.duration_in_minutes,
            week_pattern_id: value.week_pattern_id.map(|x| MsgWeekPatternId::from(x)),
        }
    }
}
