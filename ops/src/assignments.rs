use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssignmentsUpdateWarning {}

impl AssignmentsUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<String> {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssignmentsUpdateOp {
    Assign(
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::SubjectId,
        bool,
    ),
    DuplicatePreviousPeriod(collomatique_state_colloscopes::PeriodId),
    AssignAll(
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::SubjectId,
        bool,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssignmentsUpdateError {
    #[error(transparent)]
    Assign(#[from] AssignError),
    #[error(transparent)]
    DuplicatePreviousPeriod(#[from] DuplicatePreviousPeriodError),
    #[error(transparent)]
    AssignAll(#[from] AssignAllError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssignAllError {
    /// period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),

    /// subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),

    /// Subject does not run on given period
    #[error("invalid subject id {0:?} for period {1:?}")]
    SubjectDoesNotRunOnPeriod(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DuplicatePreviousPeriodError {
    /// period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),

    /// trying to override first period
    #[error("given period ({0:?}) is the first period")]
    FirstPeriodHasNoPreviousPeriod(collomatique_state_colloscopes::PeriodId),
}

impl AssignmentsUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<CleaningOp<AssignmentsUpdateWarning>> {
        None
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
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
            Self::DuplicatePreviousPeriod(period_id) => {
                let Some(position) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                else {
                    return Err(
                        DuplicatePreviousPeriodError::InvalidPeriodId(period_id.clone()).into(),
                    );
                };

                if position == 0 {
                    return Err(
                        DuplicatePreviousPeriodError::FirstPeriodHasNoPreviousPeriod(
                            period_id.clone(),
                        )
                        .into(),
                    );
                }

                let previous_period_id = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .ordered_period_list[position - 1]
                    .0;
                let current_period_assignments = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .period_map
                    .get(period_id)
                    .expect("Period id should be valid at this point")
                    .clone();
                let previous_period_assignments = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .period_map
                    .get(&previous_period_id)
                    .expect("Previous period id should be valid at this point")
                    .clone();

                let student_map = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .student_map
                    .clone();

                for (student_id, student) in &student_map {
                    if student.excluded_periods.contains(period_id) {
                        continue;
                    }
                    if student.excluded_periods.contains(&previous_period_id) {
                        continue;
                    }

                    for (subject_id, _) in &current_period_assignments.subject_map {
                        let Some(previous_assigned_students) =
                            previous_period_assignments.subject_map.get(subject_id)
                        else {
                            continue;
                        };

                        let previous_status = previous_assigned_students.contains(student_id);

                        data.apply(
                            collomatique_state_colloscopes::Op::Assignment(
                                collomatique_state_colloscopes::AssignmentOp::Assign(
                                    *period_id,
                                    *student_id,
                                    *subject_id,
                                    previous_status,
                                ),
                            ),
                            self.get_desc(),
                        )
                        .expect("All data should be valid at this point");
                    }
                }

                Ok(())
            }
            Self::AssignAll(period_id, subject_id, status) => {
                if data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_none()
                {
                    return Err(AssignAllError::InvalidPeriodId(period_id.clone()).into());
                };

                let Some(subject) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                else {
                    return Err(AssignAllError::InvalidSubjectId(*subject_id).into());
                };

                if subject.excluded_periods.contains(period_id) {
                    return Err(
                        AssignAllError::SubjectDoesNotRunOnPeriod(*subject_id, *period_id).into(),
                    );
                }

                let student_map = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .student_map
                    .clone();
                for (student_id, student) in &student_map {
                    if student.excluded_periods.contains(period_id) {
                        continue;
                    }

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
                        .expect("All data should be valid at this point");

                    assert!(result.is_none());
                }

                Ok(())
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Assignments,
            match self {
                AssignmentsUpdateOp::Assign(_, _, _, status) => {
                    if *status {
                        "Inscrire un élève à une matière".into()
                    } else {
                        "Désinscrire un élève d'une matière".into()
                    }
                }
                AssignmentsUpdateOp::DuplicatePreviousPeriod(_) => {
                    "Dupliquer les inscriptions d'une période".into()
                }
                AssignmentsUpdateOp::AssignAll(_, _, status) => {
                    if *status {
                        "Inscrire tous les élèves à une matière".into()
                    } else {
                        "Désinscrire tous les élèves d'une matière".into()
                    }
                }
            },
        )
    }
}
