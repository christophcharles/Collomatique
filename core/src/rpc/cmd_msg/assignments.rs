use crate::rpc::error_msg::assignments::AssignmentsError;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignmentsCmdMsg {
    Assign(MsgPeriodId, MsgStudentId, MsgSubjectId, bool),
    DuplicatePreviousPeriod(MsgPeriodId),
}

impl AssignmentsCmdMsg {
    pub fn promote(
        self,
        data: &collomatique_state_colloscopes::Data,
    ) -> Result<crate::ops::AssignmentsUpdateOp, AssignmentsError> {
        use crate::ops::AssignmentsUpdateOp;
        Ok(match self {
            AssignmentsCmdMsg::Assign(period_id, student_id, subject_id, status) => {
                let Some(period_id) = data.validate_period_id(period_id.0) else {
                    return Err(error_msg::AssignError::InvalidPeriodId(period_id).into());
                };
                let Some(student_id) = data.validate_student_id(student_id.0) else {
                    return Err(error_msg::AssignError::InvalidStudentId(student_id).into());
                };
                let Some(subject_id) = data.validate_subject_id(subject_id.0) else {
                    return Err(error_msg::AssignError::InvalidSubjectId(subject_id).into());
                };
                AssignmentsUpdateOp::Assign(period_id, student_id, subject_id, status)
            }
            AssignmentsCmdMsg::DuplicatePreviousPeriod(period_id) => {
                let Some(period_id) = data.validate_period_id(period_id.0) else {
                    return Err(error_msg::DuplicatePreviousPeriodError::InvalidPeriodId(
                        period_id,
                    )
                    .into());
                };
                AssignmentsUpdateOp::DuplicatePreviousPeriod(period_id)
            }
        })
    }
}
