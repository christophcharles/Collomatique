use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TeachersUpdateWarning {
    LooseInterrogationSlots(collomatique_state_colloscopes::TeacherId),
}

impl TeachersUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            TeachersUpdateWarning::LooseInterrogationSlots(teacher_id) => {
                let Some(teacher) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .teachers
                    .teacher_map
                    .get(teacher_id)
                else {
                    return None;
                };
                Some(format!(
                    "Pertes des créneaux de colle du colleur {} {}",
                    teacher.desc.firstname, teacher.desc.surname,
                ))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TeachersUpdateOp {
    AddNewTeacher(collomatique_state_colloscopes::teachers::Teacher),
    UpdateTeacher(
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::teachers::Teacher,
    ),
    DeleteTeacher(collomatique_state_colloscopes::TeacherId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum TeachersUpdateError {
    #[error(transparent)]
    AddNewTeacher(#[from] AddNewTeacherError),
    #[error(transparent)]
    UpdateTeacher(#[from] UpdateTeacherError),
    #[error(transparent)]
    DeleteTeacher(#[from] DeleteTeacherError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddNewTeacherError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateTeacherError {
    #[error("Teacher ID {0:?} is invalid")]
    InvalidTeacherId(collomatique_state_colloscopes::TeacherId),
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeleteTeacherError {
    #[error("Teacher ID {0:?} is invalid")]
    InvalidTeacherId(collomatique_state_colloscopes::TeacherId),
}

impl TeachersUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<CleaningOp<TeachersUpdateWarning>> {
        match self {
            Self::AddNewTeacher(_) => None,
            Self::UpdateTeacher(teacher_id, teacher) => {
                for subject_id in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .subjects_with_slots()
                {
                    if teacher.subjects.contains(&subject_id) {
                        continue;
                    }
                    for (slot_id, slot) in data
                        .get_data()
                        .get_inner_data()
                        .params
                        .slots
                        .slots_for_subject(subject_id)
                        .into_iter()
                        .flatten()
                    {
                        if slot.teacher_id == *teacher_id {
                            return Some(CleaningOp {
                                warning: TeachersUpdateWarning::LooseInterrogationSlots(
                                    *teacher_id,
                                ),
                                op: UpdateOp::Slots(SlotsUpdateOp::DeleteSlot(*slot_id)),
                            });
                        }
                    }
                }

                None
            }
            Self::DeleteTeacher(teacher_id) => {
                for (slot_id, slot) in data.get_data().get_inner_data().params.slots.all_slots() {
                    if slot.teacher_id == *teacher_id {
                        return Some(CleaningOp {
                            warning: TeachersUpdateWarning::LooseInterrogationSlots(*teacher_id),
                            op: UpdateOp::Slots(SlotsUpdateOp::DeleteSlot(*slot_id)),
                        });
                    }
                }

                None
            }
        }
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::TeacherId>, TeachersUpdateError> {
        match self {
            Self::AddNewTeacher(teacher) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Teacher(
                            collomatique_state_colloscopes::TeacherOp::Add(teacher.clone()),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, FixableInvariant, Reference, SubjectRefSite,
                        };
                        match e {
                            // The pre-op state was valid, so any teacher->subject
                            // dangle in the set was introduced by this Add; the
                            // dangling target is the bad input subject id.
                            Error::BrokenInvariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Subject {
                                        target,
                                        site: SubjectRefSite::TeacherSubjects(_),
                                    }) = inv
                                    {
                                        return AddNewTeacherError::InvalidSubjectId(*target);
                                    }
                                }
                                panic!("Unexpected invariant breaks during AddNewTeacher: {set:?}");
                            }
                            _ => panic!("Unexpected error during AddNewTeacher: {e:?}"),
                        }
                    })?;
                let Some(collomatique_state_colloscopes::NewId::TeacherId(new_id)) = result else {
                    panic!("Unexpected result from TeacherOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateTeacher(teacher_id, teacher) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Teacher(
                            collomatique_state_colloscopes::TeacherOp::Update(
                                *teacher_id,
                                teacher.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, Convergence, FixableInvariant, InvalidOp, PrecheckError,
                            Reference, SubjectRefSite, TeacherPrecheckError,
                        };
                        match e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Teacher(
                                TeacherPrecheckError::InvalidTeacherId(id),
                            ))) => UpdateTeacherError::InvalidTeacherId(id),
                            Error::BrokenInvariants(set) => {
                                // Old validator order: validate_teacher (subject
                                // ids) fires before the dropped-subject slot scan.
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Subject {
                                        target,
                                        site: SubjectRefSite::TeacherSubjects(_),
                                    }) = inv
                                    {
                                        return UpdateTeacherError::InvalidSubjectId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::SlotTeacherDoesNotTeachSubject(_, _, _),
                                    ) = inv
                                    {
                                        panic!("Slots should be cleaned before updating subjects for teacher");
                                    }
                                }
                                panic!(
                                    "Unexpected invariant breaks during UpdateTeacher: {set:?}"
                                );
                            }
                            _ => panic!("Unexpected error during UpdateTeacher: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteTeacher(teacher_id) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Teacher(
                            collomatique_state_colloscopes::TeacherOp::Remove(*teacher_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, FixableInvariant, InvalidOp, PrecheckError, Reference,
                            TeacherPrecheckError, TeacherRefSite,
                        };
                        match e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Teacher(
                                TeacherPrecheckError::InvalidTeacherId(id),
                            ))) => DeleteTeacherError::InvalidTeacherId(id),
                            Error::BrokenInvariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Teacher {
                                        site: TeacherRefSite::SlotTeacher(_),
                                        ..
                                    }) = inv
                                    {
                                        panic!("Slots should be cleaned before removing teacher");
                                    }
                                }
                                panic!("Unexpected invariant breaks during DeleteTeacher: {set:?}");
                            }
                            _ => panic!("Unexpected error during DeleteTeacher: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Teachers,
            match self {
                TeachersUpdateOp::AddNewTeacher(_desc) => "Ajouter un colleur".into(),
                TeachersUpdateOp::UpdateTeacher(_id, _desc) => "Modifier un colleur".into(),
                TeachersUpdateOp::DeleteTeacher(_id) => "Supprimer un colleur".into(),
            },
        )
    }
}
