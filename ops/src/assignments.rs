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
                // Build the whole target row from the current one (the rest of
                // it came from a valid state, so the only id `SetRow` can reject
                // as invalid is this op's own student).
                let mut new_row = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .students(*period_id, *subject_id)
                    .cloned()
                    .unwrap_or_default();
                if *status {
                    new_row.insert(*student_id);
                } else {
                    new_row.remove(student_id);
                }

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Assignment(
                            collomatique_state_colloscopes::AssignmentOp::SetRow(
                                *period_id,
                                *subject_id,
                                new_row,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            AssignmentPrecheckError, Convergence, Error, FixableInvariant,
                            InvalidOp, PrecheckError,
                        };
                        match e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Assignment(
                                pe,
                            ))) => match pe {
                                AssignmentPrecheckError::InvalidPeriodId(id) => {
                                    AssignError::InvalidPeriodId(id)
                                }
                                AssignmentPrecheckError::InvalidStudentId(id) => {
                                    AssignError::InvalidStudentId(id)
                                }
                                AssignmentPrecheckError::InvalidSubjectId(id) => {
                                    AssignError::InvalidSubjectId(id)
                                }
                            },
                            // The pre-op state was valid, so any convergence break
                            // in the set was introduced by this Assign. Old validator
                            // order (colloscope_params validate): subject-not-running
                            // before student-not-present.
                            Error::BrokenInvariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::AssignmentForSubjectNotRunningOnPeriod(
                                            period,
                                            subject,
                                        ),
                                    ) = inv
                                    {
                                        return AssignError::SubjectDoesNotRunOnPeriod(
                                            *subject, *period,
                                        );
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::AssignedStudentNotPresentForPeriod {
                                            period,
                                            student,
                                            ..
                                        },
                                    ) = inv
                                    {
                                        return AssignError::StudentIsNotPresentOnPeriod(
                                            *student, *period,
                                        );
                                    }
                                }
                                panic!("Unexpected invariant breaks during Assign: {set:?}");
                            }
                            _ => panic!("Unexpected error during Assign: {e:?}"),
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
                    return Err(DuplicatePreviousPeriodError::InvalidPeriodId(*period_id).into());
                };

                if position == 0 {
                    return Err(
                        DuplicatePreviousPeriodError::FirstPeriodHasNoPreviousPeriod(*period_id)
                            .into(),
                    );
                }

                let previous_period_id = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .period_id_at(position - 1)
                    .expect("position > 0 checked above");
                let current_period_assignments: std::collections::BTreeMap<_, _> = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .subjects_for_period(*period_id)
                    .map(|(subject_id, students)| (subject_id, students.clone()))
                    .collect();
                let previous_period_assignments: std::collections::BTreeMap<_, _> = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .subjects_for_period(previous_period_id)
                    .map(|(subject_id, students)| (subject_id, students.clone()))
                    .collect();

                let student_map = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .student_map
                    .clone();

                // One SetRow per current-period subject that also has a
                // previous-period row: non-excluded students copy the previous
                // period's membership; students excluded from either period keep
                // their current status. Same observable result as the old
                // per-(student, subject) loop, one history entry per subject.
                for (subject_id, current_students) in &current_period_assignments {
                    let Some(previous_students) = previous_period_assignments.get(subject_id)
                    else {
                        continue;
                    };

                    let mut new_row = std::collections::BTreeSet::new();
                    for (student_id, student) in student_map.iter() {
                        let excluded = student.excluded_periods.contains(period_id)
                            || student.excluded_periods.contains(&previous_period_id);
                        let assigned = if excluded {
                            current_students.contains(&student_id)
                        } else {
                            previous_students.contains(&student_id)
                        };
                        if assigned {
                            new_row.insert(student_id);
                        }
                    }

                    data.apply(
                        collomatique_state_colloscopes::Op::Assignment(
                            collomatique_state_colloscopes::AssignmentOp::SetRow(
                                *period_id,
                                *subject_id,
                                new_row,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("All data should be valid at this point");
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
                    return Err(AssignAllError::InvalidPeriodId(*period_id).into());
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

                // One SetRow for the whole row: every non-excluded student for
                // `status == true`, the empty set (row removal) for `false` —
                // every assigned student in a valid state is non-excluded, so
                // clearing them all is exactly the row's removal.
                let new_row: std::collections::BTreeSet<_> = if *status {
                    data.get_data()
                        .get_inner_data()
                        .params
                        .students
                        .student_map
                        .iter()
                        .filter(|(_, student)| !student.excluded_periods.contains(period_id))
                        .map(|(student_id, _)| student_id)
                        .collect()
                } else {
                    std::collections::BTreeSet::new()
                };

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Assignment(
                            collomatique_state_colloscopes::AssignmentOp::SetRow(
                                *period_id,
                                *subject_id,
                                new_row,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("All data should be valid at this point");

                assert!(result.is_none());

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
