use std::collections::BTreeSet;

use super::*;

#[derive(Debug)]
pub enum SubjectsUpdateOp {
    AddNewSubject(collomatique_state_colloscopes::subjects::SubjectParameters),
    UpdateSubject(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::subjects::SubjectParameters,
    ),
    DeleteSubject(collomatique_state_colloscopes::SubjectId),
    MoveUp(collomatique_state_colloscopes::SubjectId),
    MoveDown(collomatique_state_colloscopes::SubjectId),
    UpdatePeriodStatus(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
        bool,
    ),
}

#[derive(Debug, Error)]
pub enum SubjectsUpdateError {
    #[error(transparent)]
    AddNewSubject(#[from] AddNewSubjectError),
    #[error(transparent)]
    UpdateSubject(#[from] UpdateSubjectError),
    #[error(transparent)]
    DeleteSubject(#[from] DeleteSubjectError),
    #[error(transparent)]
    MoveUp(#[from] MoveUpError),
    #[error(transparent)]
    MoveDown(#[from] MoveDownError),
    #[error(transparent)]
    UpdatePeriodStatus(#[from] UpdatePeriodStatusError),
}

#[derive(Debug, Error)]
pub enum AddNewSubjectError {
    #[error("Students per group range should allow at least one value")]
    StudentsPerGroupRangeIsEmpty,
    #[error("Groups per interrogations range should allow at least one value")]
    GroupsPerInterrogationRangeIsEmpty,
}

#[derive(Debug, Error)]
pub enum UpdateSubjectError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Students per group range should allow at least one value")]
    StudentsPerGroupRangeIsEmpty,
    #[error("Groups per interrogations range should allow at least one value")]
    GroupsPerInterrogationRangeIsEmpty,
}

#[derive(Debug, Error)]
pub enum DeleteSubjectError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
}

#[derive(Debug, Error)]
pub enum MoveUpError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Subject is already the first subject")]
    NoUpperPosition,
}

#[derive(Debug, Error)]
pub enum MoveDownError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Subject is already the last subject")]
    NoLowerPosition,
}

#[derive(Debug, Error)]
pub enum UpdatePeriodStatusError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

impl SubjectsUpdateOp {
    pub fn get_desc(&self) -> String {
        match self {
            SubjectsUpdateOp::AddNewSubject(_desc) => "Ajouter une matière".into(),
            SubjectsUpdateOp::UpdateSubject(_id, _desc) => "Modifier une matière".into(),
            SubjectsUpdateOp::DeleteSubject(_id) => "Supprimer une matière".into(),
            SubjectsUpdateOp::MoveUp(_id) => "Remonter une matière".into(),
            SubjectsUpdateOp::MoveDown(_id) => "Descendre une matière".into(),
            Self::UpdatePeriodStatus(_subject_id, _period_id, status) => {
                if *status {
                    "Dispenser une matière sur une période".into()
                } else {
                    "Ne pas dispenser une matière sur une période".into()
                }
            }
        }
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data>>(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::SubjectId>, SubjectsUpdateError> {
        match self {
            Self::AddNewSubject(params) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::AddAfter(
                                data.get_data().get_subjects().ordered_subject_list.last().map(|x| x.0),
                                collomatique_state_colloscopes::Subject {
                                    parameters: params.clone(),
                                    excluded_periods: BTreeSet::new(),
                                }
                            )
                        ),
                        self.get_desc()
                    ).map_err(|e| if let collomatique_state_colloscopes::Error::Subject(se) = e {
                        match se {
                            collomatique_state_colloscopes::SubjectError::GroupsPerInterrogationRangeIsEmpty => AddNewSubjectError::GroupsPerInterrogationRangeIsEmpty,
                            collomatique_state_colloscopes::SubjectError::StudentsPerGroupRangeIsEmpty => AddNewSubjectError::StudentsPerGroupRangeIsEmpty,
                            _ => panic!("Unexpected subject error during AddNewSubject: {:?}", se),
                        }
                    } else {
                        panic!("Unexpected error during AddNewSubject: {:?}", e);
                    })?;
                let Some(collomatique_state_colloscopes::NewId::SubjectId(new_id)) = result else {
                    panic!("Unexpected result from SubjectOp::AddAfter");
                };
                Ok(Some(new_id))
            }
            Self::UpdateSubject(subject_id, params) => {
                let excluded_periods = data
                    .get_data()
                    .get_subjects()
                    .find_subject(*subject_id)
                    .ok_or(UpdateSubjectError::InvalidSubjectId(*subject_id))?
                    .excluded_periods
                    .clone();

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::Update(
                                *subject_id,
                                collomatique_state_colloscopes::Subject {
                                    parameters: params.clone(),
                                    excluded_periods,
                                }
                            )
                        ),
                        self.get_desc()
                    ).map_err(|e| if let collomatique_state_colloscopes::Error::Subject(se) = e {
                        match se {
                            collomatique_state_colloscopes::SubjectError::GroupsPerInterrogationRangeIsEmpty => AddNewSubjectError::GroupsPerInterrogationRangeIsEmpty,
                            collomatique_state_colloscopes::SubjectError::StudentsPerGroupRangeIsEmpty => AddNewSubjectError::StudentsPerGroupRangeIsEmpty,
                            _ => panic!("Unexpected subject error during UpdateSubject: {:?}", se),
                        }
                    } else {
                        panic!("Unexpected error during UpdateSubject: {:?}", e);
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteSubject(subject_id) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::Remove(*subject_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Subject(se) = e {
                            match se {
                                collomatique_state_colloscopes::SubjectError::InvalidSubjectId(
                                    id,
                                ) => DeleteSubjectError::InvalidSubjectId(id),
                                _ => panic!(
                                    "Unexpected subject error during DeleteSubject: {:?}",
                                    se
                                ),
                            }
                        } else {
                            panic!("Unexpected error during DeleteSubject: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::MoveUp(subject_id) => {
                let current_position = data
                    .get_data()
                    .get_subjects()
                    .find_subject_position(*subject_id)
                    .ok_or(MoveUpError::InvalidSubjectId(*subject_id))?;

                if current_position == 0 {
                    Err(MoveUpError::NoUpperPosition)?;
                }

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::ChangePosition(
                                *subject_id,
                                current_position - 1,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("No error should be possible at this point");

                assert!(result.is_none());

                Ok(None)
            }
            Self::MoveDown(subject_id) => {
                let current_position = data
                    .get_data()
                    .get_subjects()
                    .find_subject_position(*subject_id)
                    .ok_or(MoveDownError::InvalidSubjectId(*subject_id))?;

                if current_position == data.get_data().get_subjects().ordered_subject_list.len() - 1
                {
                    Err(MoveDownError::NoLowerPosition)?;
                }

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::ChangePosition(
                                *subject_id,
                                current_position + 1,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("No error should be possible at this point");

                assert!(result.is_none());

                Ok(None)
            }
            Self::UpdatePeriodStatus(subject_id, period_id, new_status) => {
                if data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .is_none()
                {
                    Err(UpdatePeriodStatusError::InvalidPeriodId(*period_id))?;
                }
                let mut subject = data
                    .get_data()
                    .get_subjects()
                    .find_subject(*subject_id)
                    .ok_or(UpdatePeriodStatusError::InvalidSubjectId(*subject_id))?
                    .clone();

                if *new_status {
                    subject.excluded_periods.insert(*period_id);
                } else {
                    subject.excluded_periods.remove(period_id);
                }

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::Update(*subject_id, subject),
                        ),
                        self.get_desc(),
                    )
                    .expect("No error should be possible at this point");
                assert!(result.is_none());

                Ok(None)
            }
        }
    }
}
