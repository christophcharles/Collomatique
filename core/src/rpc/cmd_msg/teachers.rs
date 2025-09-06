use crate::rpc::error_msg::teachers::TeachersError;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeachersCmdMsg {
    AddNewTeacher(TeacherMsg),
    UpdateTeacher(MsgTeacherId, TeacherMsg),
    DeleteTeacher(MsgTeacherId),
}

impl TeachersCmdMsg {
    pub fn promote(
        self,
        data: &collomatique_state_colloscopes::Data,
    ) -> Result<crate::ops::TeachersUpdateOp, TeachersError> {
        use crate::ops::TeachersUpdateOp;
        Ok(match self {
            TeachersCmdMsg::AddNewTeacher(teacher_msg) => {
                let new_teacher = match data
                    .get_inner_data()
                    .main_params
                    .promote_teacher(teacher_msg.into())
                {
                    Ok(t) => t,
                    Err(subject_id) => {
                        return Err(TeachersError::AddNewTeacher(
                            error_msg::AddNewTeacherError::InvalidSubjectId(MsgSubjectId(
                                subject_id,
                            )),
                        ))
                    }
                };

                TeachersUpdateOp::AddNewTeacher(new_teacher)
            }
            TeachersCmdMsg::UpdateTeacher(id, teacher_msg) => {
                let Some(teacher_id) = data.get_inner_data().main_params.validate_teacher_id(id.0)
                else {
                    return Err(error_msg::UpdateTeacherError::InvalidTeacherId(id).into());
                };
                let new_teacher = match data
                    .get_inner_data()
                    .main_params
                    .promote_teacher(teacher_msg.into())
                {
                    Ok(t) => t,
                    Err(subject_id) => {
                        return Err(TeachersError::AddNewTeacher(
                            error_msg::AddNewTeacherError::InvalidSubjectId(MsgSubjectId(
                                subject_id,
                            )),
                        ))
                    }
                };
                TeachersUpdateOp::UpdateTeacher(teacher_id, new_teacher)
            }
            TeachersCmdMsg::DeleteTeacher(id) => {
                let Some(teacher_id) = data.get_inner_data().main_params.validate_teacher_id(id.0)
                else {
                    return Err(error_msg::DeleteTeacherError::InvalidTeacherId(id).into());
                };
                TeachersUpdateOp::DeleteTeacher(teacher_id)
            }
        })
    }
}

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeacherMsg {
    pub desc: PersonWithContactMsg,
    pub subjects: BTreeSet<MsgSubjectId>,
}

impl From<TeacherMsg> for collomatique_state_colloscopes::teachers::TeacherExternalData {
    fn from(value: TeacherMsg) -> Self {
        collomatique_state_colloscopes::teachers::TeacherExternalData {
            desc: value.desc.into(),
            subjects: value.subjects.into_iter().map(|x| x.0).collect(),
        }
    }
}
