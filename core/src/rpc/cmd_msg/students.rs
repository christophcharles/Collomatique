use crate::rpc::error_msg::students::StudentsError;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StudentsCmdMsg {
    AddNewStudent(StudentMsg),
    UpdateStudent(MsgStudentId, StudentMsg),
    DeleteStudent(MsgStudentId),
}

impl StudentsCmdMsg {
    pub fn promote(
        self,
        data: &collomatique_state_colloscopes::Data,
    ) -> Result<crate::ops::StudentsUpdateOp, StudentsError> {
        use crate::ops::StudentsUpdateOp;
        Ok(match self {
            StudentsCmdMsg::AddNewStudent(student_msg) => {
                let new_student = match data
                    .get_inner_data()
                    .main_params
                    .promote_student(student_msg.into())
                {
                    Ok(t) => t,
                    Err(period_id) => {
                        return Err(StudentsError::AddNewStudent(
                            error_msg::AddNewStudentError::InvalidPeriodId(MsgPeriodId(period_id)),
                        ))
                    }
                };

                StudentsUpdateOp::AddNewStudent(new_student)
            }
            StudentsCmdMsg::UpdateStudent(id, student_msg) => {
                let Some(student_id) = data.get_inner_data().main_params.validate_student_id(id.0)
                else {
                    return Err(error_msg::UpdateStudentError::InvalidStudentId(id).into());
                };
                let new_student = match data
                    .get_inner_data()
                    .main_params
                    .promote_student(student_msg.into())
                {
                    Ok(t) => t,
                    Err(period_id) => {
                        return Err(StudentsError::AddNewStudent(
                            error_msg::AddNewStudentError::InvalidPeriodId(MsgPeriodId(period_id)),
                        ))
                    }
                };
                StudentsUpdateOp::UpdateStudent(student_id, new_student)
            }
            StudentsCmdMsg::DeleteStudent(id) => {
                let Some(student_id) = data.get_inner_data().main_params.validate_student_id(id.0)
                else {
                    return Err(error_msg::DeleteStudentError::InvalidStudentId(id).into());
                };
                StudentsUpdateOp::DeleteStudent(student_id)
            }
        })
    }
}

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudentMsg {
    pub desc: PersonWithContactMsg,
    pub excluded_periods: BTreeSet<MsgPeriodId>,
}

impl From<StudentMsg> for collomatique_state_colloscopes::students::StudentExternalData {
    fn from(value: StudentMsg) -> Self {
        collomatique_state_colloscopes::students::StudentExternalData {
            desc: value.desc.into(),
            excluded_periods: value.excluded_periods.into_iter().map(|x| x.0).collect(),
        }
    }
}
