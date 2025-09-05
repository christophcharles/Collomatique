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
    pub name: String,
    #[pyo3(set, get)]
    pub slots: Vec<time::SlotWithDuration>,
    #[pyo3(set, get)]
    pub minimum_free_slots: NonZeroU32,
    #[pyo3(set, get)]
    pub week_pattern_id: Option<WeekPatternId>,
}

#[pymethods]
impl Incompat {
    #[new]
    fn new(
        subject_id: SubjectId,
        name: String,
        slots: Vec<time::SlotWithDuration>,
        minimum_free_slots: NonZeroU32,
    ) -> Self {
        Incompat {
            subject_id,
            name,
            slots,
            minimum_free_slots,
            week_pattern_id: None,
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl
    From<
        collomatique_state_colloscopes::incompats::Incompatibility<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    > for Incompat
{
    fn from(
        value: collomatique_state_colloscopes::incompats::Incompatibility<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    ) -> Self {
        Incompat {
            subject_id: MsgSubjectId::from(value.subject_id).into(),
            name: value.name,
            slots: value.slots.into_iter().map(|x| x.into()).collect(),
            minimum_free_slots: value.minimum_free_slots,
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
            name: value.name,
            slots: value.slots.into_iter().map(|x| x.into()).collect(),
            minimum_free_slots: value.minimum_free_slots,
            week_pattern_id: value.week_pattern_id.map(|x| MsgWeekPatternId::from(x)),
        }
    }
}
