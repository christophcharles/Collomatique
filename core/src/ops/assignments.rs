use super::*;

#[derive(Debug)]
pub enum AssignmentsUpdateOp {
    Assign(
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::SubjectId,
        bool,
    ),
}

#[derive(Debug, Error)]
pub enum AssignmentsUpdateError {
    #[error(transparent)]
    Assign(#[from] AssignError),
}

#[derive(Debug, Error)]
pub enum AssignError {
    /// period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),

    /// subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),

    /// student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),

    /// Subject does not run on given period
    #[error("invalid subject id {0:?} for period {1:?}")]
    SubjectDoesNotRunOnPeriod(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    ),

    /// Student is not present on given period
    #[error("invalid subject id {0:?} for period {1:?}")]
    StudentIsNotPresentOnPeriod(
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::PeriodId,
    ),
}

impl AssignmentsUpdateOp {
    pub fn get_desc(&self) -> String {
        match self {
            AssignmentsUpdateOp::Assign(_, _, _, status) => {
                if *status {
                    "Inscrire un élève à une matière".into()
                } else {
                    "Désinscrire un élève d'une matière".into()
                }
            }
        }
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data>>(
        &self,
        data: &mut T,
    ) -> Result<(), AssignmentsUpdateError> {
        match self {
            Self::Assign(period_id, student_id, subject_id, status) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Assignment(
                            collomatique_state_colloscopes::AssignmentOp::Assign(
                                *period_id,
                                *student_id,
                                *subject_id,
                                *status,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Assignment(ae) = e {
                            match ae {
                                collomatique_state_colloscopes::AssignmentError::InvalidPeriodId(period_id) => {
                                    AssignError::InvalidPeriodId(period_id)
                                }
                                collomatique_state_colloscopes::AssignmentError::InvalidStudentId(student_id) => {
                                    AssignError::InvalidStudentId(student_id)
                                }
                                collomatique_state_colloscopes::AssignmentError::InvalidSubjectId(subject_id) => {
                                    AssignError::InvalidSubjectId(subject_id)
                                }
                                collomatique_state_colloscopes::AssignmentError::StudentIsNotPresentOnPeriod(student_id, period_id) => {
                                    AssignError::StudentIsNotPresentOnPeriod(student_id, period_id)
                                }
                                collomatique_state_colloscopes::AssignmentError::SubjectDoesNotRunOnPeriod(subject_id, period_id) => {
                                    AssignError::SubjectDoesNotRunOnPeriod(subject_id, period_id)
                                }
                            }
                        } else {
                            panic!("Unexpected error during Assign: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                Ok(())
            }
        }
    }
}
