use super::*;
use pyo3::types::PyString;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotId {
    id: crate::rpc::cmd_msg::MsgSlotId,
}

#[pymethods]
impl SlotId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&crate::rpc::cmd_msg::MsgSlotId> for SlotId {
    fn from(value: &crate::rpc::cmd_msg::MsgSlotId) -> Self {
        SlotId { id: value.clone() }
    }
}

impl From<crate::rpc::cmd_msg::MsgSlotId> for SlotId {
    fn from(value: crate::rpc::cmd_msg::MsgSlotId) -> Self {
        SlotId::from(&value)
    }
}

impl From<&SlotId> for crate::rpc::cmd_msg::MsgSlotId {
    fn from(value: &SlotId) -> Self {
        value.id.clone()
    }
}

impl From<SlotId> for crate::rpc::cmd_msg::MsgSlotId {
    fn from(value: SlotId) -> Self {
        crate::rpc::cmd_msg::MsgSlotId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    #[pyo3(set, get)]
    pub id: SlotId,
    #[pyo3(set, get)]
    pub parameters: SlotParameters,
}

#[pymethods]
impl Slot {
    #[new]
    fn new(id: SlotId, parameters: SlotParameters) -> Self {
        Slot { id, parameters }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotParameters {
    #[pyo3(set, get)]
    pub teacher_id: TeacherId,
    #[pyo3(set, get)]
    pub start_time: time::SlotStart,
    #[pyo3(set, get)]
    pub extra_info: String,
    #[pyo3(set, get)]
    pub week_pattern: Option<WeekPatternId>,
    #[pyo3(set, get)]
    pub cost: i32,
}

#[pymethods]
impl SlotParameters {
    #[new]
    fn new(teacher_id: TeacherId, start_time: time::SlotStart) -> Self {
        SlotParameters {
            teacher_id,
            start_time,
            extra_info: String::new(),
            week_pattern: None,
            cost: 0,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl
    From<
        collomatique_state_colloscopes::slots::Slot<
            collomatique_state_colloscopes::TeacherId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    > for SlotParameters
{
    fn from(
        value: collomatique_state_colloscopes::slots::Slot<
            collomatique_state_colloscopes::TeacherId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    ) -> Self {
        SlotParameters {
            teacher_id: MsgTeacherId::from(value.teacher_id).into(),
            start_time: value.start_time.into(),
            extra_info: value.extra_info,
            week_pattern: value.week_pattern.map(|x| MsgWeekPatternId::from(x).into()),
            cost: value.cost,
        }
    }
}

impl From<SlotParameters> for crate::rpc::cmd_msg::slots::SlotMsg {
    fn from(value: SlotParameters) -> Self {
        use crate::rpc::cmd_msg::slots::SlotMsg;
        SlotMsg {
            teacher_id: value.teacher_id.into(),
            start_day: collomatique_time::Weekday::from(value.start_time.weekday).into_inner(),
            start_time: collomatique_time::TimeOnMinutes::from(value.start_time.start_time)
                .into_inner(),
            extra_info: value.extra_info,
            week_pattern: value.week_pattern.map(|x| x.into()),
            cost: value.cost,
        }
    }
}
