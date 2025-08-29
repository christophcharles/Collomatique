use super::*;
use pyo3::types::PyString;

use std::num::NonZeroU32;

#[pyclass(eq, hash, frozen)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubjectId {
    id: crate::rpc::cmd_msg::MsgSubjectId,
}

#[pymethods]
impl SubjectId {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<&crate::rpc::cmd_msg::MsgSubjectId> for SubjectId {
    fn from(value: &crate::rpc::cmd_msg::MsgSubjectId) -> Self {
        SubjectId { id: value.clone() }
    }
}

impl From<crate::rpc::cmd_msg::MsgSubjectId> for SubjectId {
    fn from(value: crate::rpc::cmd_msg::MsgSubjectId) -> Self {
        SubjectId::from(&value)
    }
}

impl From<&SubjectId> for crate::rpc::cmd_msg::MsgSubjectId {
    fn from(value: &SubjectId) -> Self {
        value.id.clone()
    }
}

impl From<SubjectId> for crate::rpc::cmd_msg::MsgSubjectId {
    fn from(value: SubjectId) -> Self {
        crate::rpc::cmd_msg::MsgSubjectId::from(&value)
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    #[pyo3(set, get)]
    pub id: SubjectId,
    #[pyo3(set, get)]
    pub parameters: SubjectParameters,
}

#[pymethods]
impl Subject {
    #[new]
    fn new(id: SubjectId, parameters: SubjectParameters) -> Self {
        Subject { id, parameters }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectParameters {
    #[pyo3(set, get)]
    name: String,
    #[pyo3(set, get)]
    students_per_group_min: NonZeroU32,
    #[pyo3(set, get)]
    students_per_group_max: NonZeroU32,
    #[pyo3(set, get)]
    groups_per_interrogation_min: NonZeroU32,
    #[pyo3(set, get)]
    groups_per_interrogation_max: NonZeroU32,
    #[pyo3(set, get)]
    duration: NonZeroU32,
    #[pyo3(set, get)]
    take_duration_into_account: bool,
    #[pyo3(set, get)]
    periodicity: SubjectPeriodicity,
}

impl From<collomatique_state_colloscopes::SubjectParameters> for SubjectParameters {
    fn from(value: collomatique_state_colloscopes::SubjectParameters) -> Self {
        SubjectParameters {
            name: value.name,
            students_per_group_min: value.students_per_group.start().clone(),
            students_per_group_max: value.students_per_group.end().clone(),
            groups_per_interrogation_min: value.students_per_group.start().clone(),
            groups_per_interrogation_max: value.students_per_group.end().clone(),
            duration: value.duration.get(),
            take_duration_into_account: value.take_duration_into_account,
            periodicity: value.periodicity.into(),
        }
    }
}

impl From<SubjectParameters> for crate::rpc::cmd_msg::subjects::SubjectParametersMsg {
    fn from(value: SubjectParameters) -> Self {
        use crate::rpc::cmd_msg::subjects::SubjectParametersMsg;
        SubjectParametersMsg {
            name: value.name,
            students_per_group: value.students_per_group_min..=value.students_per_group_max,
            groups_per_interrogation: value.groups_per_interrogation_min
                ..=value.groups_per_interrogation_max,
            duration: value.duration,
            take_duration_into_account: value.take_duration_into_account,
            periodicity: value.periodicity.into(),
        }
    }
}

#[pymethods]
impl SubjectParameters {
    #[new]
    fn new(name: String) -> Self {
        SubjectParameters {
            name,
            students_per_group_min: NonZeroU32::new(2).unwrap(),
            students_per_group_max: NonZeroU32::new(3).unwrap(),
            groups_per_interrogation_min: NonZeroU32::new(1).unwrap(),
            groups_per_interrogation_max: NonZeroU32::new(1).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            take_duration_into_account: true,
            periodicity: SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks: NonZeroU32::new(2).unwrap(),
            },
        }
    }

    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubjectPeriodicity {
    OnceForEveryBlockOfWeeks {
        weeks_per_block: NonZeroU32,
        minimum_week_separation: NonZeroU32,
    },
    ExactlyPeriodic {
        periodicity_in_weeks: NonZeroU32,
    },
    AmountInYear {
        interrogation_count_in_year_min: u32,
        interrogation_count_in_year_max: u32,
        minimum_week_separation: u32,
    },
    OnceForEveryArbitraryBlock {
        minimum_week_separation: u32,
        blocks: Vec<SubjectWeekBlock>,
    },
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectWeekBlock {
    #[pyo3(set, get)]
    pub delay_in_weeks: u32,
    #[pyo3(set, get)]
    pub size_in_weeks: NonZeroU32,
    #[pyo3(set, get)]
    pub interrogation_count_in_block_min: u32,
    #[pyo3(set, get)]
    pub interrogation_count_in_block_max: u32,
}

impl From<collomatique_state_colloscopes::SubjectPeriodicity> for SubjectPeriodicity {
    fn from(value: collomatique_state_colloscopes::SubjectPeriodicity) -> Self {
        match value {
            collomatique_state_colloscopes::SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                weeks_per_block,
                minimum_week_separation,
            } => SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                weeks_per_block,
                minimum_week_separation,
            },
            collomatique_state_colloscopes::SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks,
            } => SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks,
            },
            collomatique_state_colloscopes::SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year,
                minimum_week_separation,
            } => SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year_min: interrogation_count_in_year.start().clone(),
                interrogation_count_in_year_max: interrogation_count_in_year.end().clone(),
                minimum_week_separation,
            },
            collomatique_state_colloscopes::SubjectPeriodicity::AmountForEveryArbitraryBlock {
                blocks,
                minimum_week_separation,
            } => SubjectPeriodicity::OnceForEveryArbitraryBlock {
                minimum_week_separation,
                blocks: blocks.into_iter().map(|b| b.into()).collect(),
            },
        }
    }
}

impl From<SubjectPeriodicity> for crate::rpc::cmd_msg::subjects::SubjectPeriodicityMsg {
    fn from(value: SubjectPeriodicity) -> Self {
        use crate::rpc::cmd_msg::subjects::SubjectPeriodicityMsg;
        match value {
            SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                weeks_per_block,
                minimum_week_separation,
            } => SubjectPeriodicityMsg::OnceForEveryBlockOfWeeks {
                weeks_per_block,
                minimum_week_separation,
            },
            SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks,
            } => SubjectPeriodicityMsg::ExactlyPeriodic {
                periodicity_in_weeks,
            },
            SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year_min,
                interrogation_count_in_year_max,
                minimum_week_separation,
            } => Self::AmountInYear {
                interrogation_count_in_year: interrogation_count_in_year_min
                    ..=interrogation_count_in_year_max,
                minimum_week_separation,
            },
            SubjectPeriodicity::OnceForEveryArbitraryBlock {
                blocks,
                minimum_week_separation,
            } => SubjectPeriodicityMsg::AmountForEveryArbitraryBlock {
                minimum_week_separation,
                blocks: blocks.into_iter().map(|b| b.into()).collect(),
            },
        }
    }
}

#[pymethods]
impl SubjectPeriodicity {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}

impl From<collomatique_state_colloscopes::subjects::WeekBlock> for SubjectWeekBlock {
    fn from(value: collomatique_state_colloscopes::subjects::WeekBlock) -> Self {
        SubjectWeekBlock {
            delay_in_weeks: value.delay_in_weeks,
            size_in_weeks: value.size_in_weeks,
            interrogation_count_in_block_min: value.interrogation_count_in_block.start().clone(),
            interrogation_count_in_block_max: value.interrogation_count_in_block.end().clone(),
        }
    }
}

impl From<SubjectWeekBlock> for crate::rpc::cmd_msg::subjects::SubjectWeekBlock {
    fn from(value: SubjectWeekBlock) -> Self {
        use crate::rpc::cmd_msg::subjects::SubjectWeekBlock;
        SubjectWeekBlock {
            delay: value.delay_in_weeks,
            size: value.size_in_weeks,
            interrogation_count: value.interrogation_count_in_block_min
                ..=value.interrogation_count_in_block_max,
        }
    }
}

#[pymethods]
impl SubjectWeekBlock {
    fn __repr__(self_: PyRef<'_, Self>) -> Bound<'_, PyString> {
        let output = format!("{:?}", *self_);
        PyString::new(self_.py(), output.as_str())
    }
}
