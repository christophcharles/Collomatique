use crate::rpc::error_msg::subjects::SubjectsError;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubjectsCmdMsg {
    AddNewSubject(SubjectParametersMsg),
    UpdateSubject(MsgSubjectId, SubjectParametersMsg),
    DeleteSubject(MsgSubjectId),
    MoveUp(MsgSubjectId),
    MoveDown(MsgSubjectId),
    UpdatePeriodStatus(MsgSubjectId, MsgPeriodId, bool),
}

impl SubjectsCmdMsg {
    pub fn promote(
        self,
        data: &collomatique_state_colloscopes::Data,
    ) -> Result<crate::ops::SubjectsUpdateOp, SubjectsError> {
        use crate::ops::SubjectsUpdateOp;
        Ok(match self {
            SubjectsCmdMsg::AddNewSubject(params) => SubjectsUpdateOp::AddNewSubject(params.into()),
            SubjectsCmdMsg::UpdateSubject(id, params) => {
                let Some(subject_id) = data.validate_subject_id(id.0) else {
                    return Err(error_msg::UpdateSubjectError::InvalidSubjectId(id).into());
                };
                SubjectsUpdateOp::UpdateSubject(subject_id, params.into())
            }
            SubjectsCmdMsg::DeleteSubject(id) => {
                let Some(subject_id) = data.validate_subject_id(id.0) else {
                    return Err(error_msg::DeleteSubjectError::InvalidSubjectId(id).into());
                };
                SubjectsUpdateOp::DeleteSubject(subject_id)
            }
            SubjectsCmdMsg::MoveUp(id) => {
                let Some(subject_id) = data.validate_subject_id(id.0) else {
                    return Err(error_msg::MoveUpError::InvalidSubjectId(id).into());
                };
                SubjectsUpdateOp::MoveUp(subject_id)
            }
            SubjectsCmdMsg::MoveDown(id) => {
                let Some(subject_id) = data.validate_subject_id(id.0) else {
                    return Err(error_msg::MoveDownError::InvalidSubjectId(id).into());
                };
                SubjectsUpdateOp::MoveDown(subject_id)
            }
            SubjectsCmdMsg::UpdatePeriodStatus(subject_id, period_id, new_status) => {
                let Some(subject_id) = data.validate_subject_id(subject_id.0) else {
                    return Err(
                        error_msg::UpdatePeriodStatusError::InvalidSubjectId(subject_id).into(),
                    );
                };
                let Some(period_id) = data.validate_period_id(period_id.0) else {
                    return Err(
                        error_msg::UpdatePeriodStatusError::InvalidPeriodId(period_id).into(),
                    );
                };
                SubjectsUpdateOp::UpdatePeriodStatus(subject_id, period_id, new_status)
            }
        })
    }
}

use std::num::NonZeroU32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectParametersMsg {
    pub name: String,
    pub students_per_group: std::ops::RangeInclusive<NonZeroU32>,
    pub groups_per_interrogation: std::ops::RangeInclusive<NonZeroU32>,
    pub duration: NonZeroU32,
    pub take_duration_into_account: bool,
    pub periodicity: SubjectPeriodicityMsg,
}

impl From<SubjectParametersMsg> for collomatique_state_colloscopes::SubjectParameters {
    fn from(value: SubjectParametersMsg) -> Self {
        collomatique_state_colloscopes::SubjectParameters {
            name: value.name,
            students_per_group: value.students_per_group,
            groups_per_interrogation: value.groups_per_interrogation,
            duration: value.duration.into(),
            take_duration_into_account: value.take_duration_into_account,
            periodicity: value.periodicity.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubjectPeriodicityMsg {
    OnceForEveryBlockOfWeeks {
        weeks_per_block: u32,
    },
    ExactlyPeriodic {
        periodicity_in_weeks: NonZeroU32,
    },
    AmountInYear {
        interrogation_count_in_year: std::ops::RangeInclusive<u32>,
        minimum_week_separation: u32,
    },
    OnceForEveryArbitraryBlock {
        blocks: Vec<SubjectWeekBlock>,
    },
}

use std::num::NonZeroUsize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectWeekBlock {
    delay: usize,
    size: NonZeroUsize,
}

impl From<SubjectPeriodicityMsg> for collomatique_state_colloscopes::SubjectPeriodicity {
    fn from(value: SubjectPeriodicityMsg) -> Self {
        match value {
            SubjectPeriodicityMsg::OnceForEveryBlockOfWeeks { weeks_per_block } => {
                collomatique_state_colloscopes::SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                    weeks_per_block,
                }
            }
            SubjectPeriodicityMsg::ExactlyPeriodic {
                periodicity_in_weeks,
            } => collomatique_state_colloscopes::SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks,
            },
            SubjectPeriodicityMsg::AmountInYear {
                interrogation_count_in_year,
                minimum_week_separation,
            } => collomatique_state_colloscopes::SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year,
                minimum_week_separation,
            },
            SubjectPeriodicityMsg::OnceForEveryArbitraryBlock { blocks } => {
                collomatique_state_colloscopes::SubjectPeriodicity::OnceForEveryArbitraryBlock {
                    blocks: blocks.into_iter().map(|b| b.into()).collect(),
                }
            }
        }
    }
}

impl From<SubjectWeekBlock> for collomatique_state_colloscopes::subjects::WeekBlock {
    fn from(value: SubjectWeekBlock) -> Self {
        collomatique_state_colloscopes::subjects::WeekBlock {
            delay_in_weeks: value.delay,
            size_in_weeks: value.size,
        }
    }
}
