use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SettingsUpdateWarning {}

impl SettingsUpdateWarning {
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
pub enum SettingsUpdateOp {
    UpdateGlobalLimits(collomatique_state_colloscopes::settings::Limits),
    UpdateStudentLimits(
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::settings::Limits,
    ),
    RemoveStudentLimits(collomatique_state_colloscopes::StudentId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum SettingsUpdateError {
    #[error(transparent)]
    UpdateStudentLimits(#[from] UpdateStudentLimitsError),
    #[error(transparent)]
    RemoveStudentLimits(#[from] RemoveStudentLimitsError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateStudentLimitsError {
    #[error("Student ID {0:?} is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemoveStudentLimitsError {
    #[error("Student ID {0:?} is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
    #[error("No limits definied for student {0:?}")]
    NoLimitsForStudent(collomatique_state_colloscopes::StudentId),
}

impl SettingsUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<CleaningOp<WeekPatternsUpdateWarning>> {
        match self {
            SettingsUpdateOp::UpdateGlobalLimits(_) => None,
            SettingsUpdateOp::UpdateStudentLimits(_, _) => None,
            SettingsUpdateOp::RemoveStudentLimits(_) => None,
        }
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<(), SettingsUpdateError> {
        match self {
            Self::UpdateGlobalLimits(limits) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Settings(
                            collomatique_state_colloscopes::SettingsOp::SetGlobal(limits.clone()),
                        ),
                        self.get_desc(),
                    )
                    .expect("SettingsOp::SetGlobal should never fail");

                assert!(result.is_none());

                Ok(())
            }
            Self::UpdateStudentLimits(student_id, limits) => {
                if !data
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .student_map
                    .contains(student_id)
                {
                    return Err(UpdateStudentLimitsError::InvalidStudentId(*student_id).into());
                }

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Settings(
                            collomatique_state_colloscopes::SettingsOp::SetStudent(
                                *student_id,
                                Some(limits.clone()),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("SettingsOp::SetStudent should not fail on a checked student id");

                assert!(result.is_none());

                Ok(())
            }
            Self::RemoveStudentLimits(student_id) => {
                if !data
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .student_map
                    .contains(student_id)
                {
                    return Err(RemoveStudentLimitsError::InvalidStudentId(*student_id).into());
                }

                // `SetStudent(_, None)` is a no-op on a student without an
                // override, so the absence is detected here rather than by the
                // elementary op.
                if !data
                    .get_data()
                    .get_inner_data()
                    .params
                    .settings
                    .students
                    .contains(student_id)
                {
                    return Err(RemoveStudentLimitsError::NoLimitsForStudent(*student_id).into());
                }

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Settings(
                            collomatique_state_colloscopes::SettingsOp::SetStudent(
                                *student_id,
                                None,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("SettingsOp::SetStudent should not fail on a checked student id");

                assert!(result.is_none());

                Ok(())
            }
        }
    }

    // Nothing outside the tests calls this yet: the `UpdateOp` dispatch that
    // does is the last commit of the family migration. Drop the attribute then.
    #[allow(dead_code)]
    pub(crate) fn apply_to_session<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<(), SettingsUpdateError> {
        match self {
            Self::UpdateGlobalLimits(limits) => {
                // The global limits name no entity, so this op can neither be
                // rejected nor make the cascade repair anything.
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Settings(
                            collomatique_state_colloscopes::SettingsOp::SetGlobal(limits.clone()),
                        ),
                        self.get_desc(),
                    )
                    .expect("SettingsOp::SetGlobal should never fail");

                assert!(result.is_none());

                Ok(())
            }
            Self::UpdateStudentLimits(student_id, limits) => {
                // An ops-level address check: it decides whether there is an op
                // to issue at all, so it stays here rather than being read back
                // out of the state layer's precheck error.
                if !session
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .student_map
                    .contains(student_id)
                {
                    return Err(UpdateStudentLimitsError::InvalidStudentId(*student_id).into());
                }

                // A per-student override on a live student breaks nothing
                // either: the settings table's only reference is the student
                // key, and it names the student just checked.
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Settings(
                            collomatique_state_colloscopes::SettingsOp::SetStudent(
                                *student_id,
                                Some(limits.clone()),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("SettingsOp::SetStudent should not fail on a checked student id");

                assert!(result.is_none());

                Ok(())
            }
            Self::RemoveStudentLimits(student_id) => {
                if !session
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .student_map
                    .contains(student_id)
                {
                    return Err(RemoveStudentLimitsError::InvalidStudentId(*student_id).into());
                }

                // `SetStudent(_, None)` is a no-op on a student without an
                // override, so the absence is detected here rather than by the
                // elementary op.
                if !session
                    .get_data()
                    .get_inner_data()
                    .params
                    .settings
                    .students
                    .contains(student_id)
                {
                    return Err(RemoveStudentLimitsError::NoLimitsForStudent(*student_id).into());
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Settings(
                            collomatique_state_colloscopes::SettingsOp::SetStudent(
                                *student_id,
                                None,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("SettingsOp::SetStudent should not fail on a checked student id");

                assert!(result.is_none());

                Ok(())
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Settings,
            match self {
                SettingsUpdateOp::UpdateGlobalLimits(_) => {
                    "Mettre à jour les paramètres généraux de limites".into()
                }
                SettingsUpdateOp::UpdateStudentLimits(_, _) => {
                    "Mettre à jour les paramètres de limites d'un élève".into()
                }
                SettingsUpdateOp::RemoveStudentLimits(_) => {
                    "Supprimer les paramètres de limites d'un élève".into()
                }
            },
        )
    }
}

#[cfg(test)]
mod tests {
    //! The settings reference exactly one thing — the student a per-student
    //! override is keyed by — so a document holding one student says everything
    //! this family has to say, and the frozen hogwarts base (`tests/fixtures/`)
    //! would only add noise.
    //!
    //! Two properties are worth the reading: the three ops-level prechecks (the
    //! whole error surface of the family: the state layer's own precheck is
    //! never reached, and the cascade never repairs anything), and the fact that
    //! those prechecks read the *session's* document rather than the state the
    //! composite started on.

    use super::*;
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::settings::{Limits, SoftParam};
    use collomatique_state_colloscopes::{
        NewId, Op, PersonWithContact, StudentOp,
        ids::{Id, StudentId},
        students::Student,
    };
    use std::num::NonZeroU32;

    /// A document with one student and no settings of any kind — the whole
    /// state these tests need to read.
    fn one_student() -> (AppState<Data, Desc>, StudentId) {
        let mut state = AppState::new(Data::default());
        let new_id = state
            .apply(
                Op::Student(StudentOp::Add(Student {
                    desc: PersonWithContact {
                        surname: "Granger".into(),
                        firstname: "Hermione".into(),
                        tel: None,
                        email: None,
                    },
                    excluded_periods: std::collections::BTreeSet::new(),
                })),
                (OpCategory::Students, "Ajouter une élève".into()),
            )
            .expect("a student attached to nothing breaks nothing");
        let Some(NewId::StudentId(student_id)) = new_id else {
            panic!("adding a student should return a student id, got {new_id:?}");
        };

        (state, student_id)
    }

    /// An id no document ever issued.
    fn dangling_student() -> StudentId {
        unsafe { StudentId::new(1u64 << 40) }
    }

    fn limits(max_per_week: u32) -> Limits {
        Limits {
            interrogations_per_week_min: None,
            interrogations_per_week_max: Some(SoftParam {
                soft: false,
                value: max_per_week,
            }),
            max_interrogations_per_day: Some(SoftParam {
                soft: true,
                value: NonZeroU32::new(2).unwrap(),
            }),
        }
    }

    fn settings_of<T: Manager<Data = Data, Desc = Desc>>(
        state: &T,
    ) -> collomatique_state_colloscopes::settings::Settings {
        state.get_data().get_inner_data().params.settings.clone()
    }

    /// The global limits name no entity: the op lands as issued, and there is
    /// nothing for the cascade to repair.
    #[test]
    fn global_limits_land_untouched_and_warn_about_nothing() {
        let (state, _student) = one_student();

        let mut session = CascadeSession::new(state);
        let op = SettingsUpdateOp::UpdateGlobalLimits(limits(3));
        op.apply_to_session(&mut session)
            .expect("global limits reference nothing, so nothing can reject them");
        let (state, warnings) = session.commit(op.get_desc());

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(settings_of(&state).global, limits(3));
        assert!(
            settings_of(&state).students.is_empty(),
            "the global limits are not a per-student override"
        );
    }

    /// A per-student override is set, then removed, on a live student. The
    /// override table is sparse: removing the override removes the entry
    /// itself, and the global limits are untouched throughout.
    #[test]
    fn a_student_override_is_set_then_removed() {
        let (state, student) = one_student();

        let mut session = CascadeSession::new(state);
        SettingsUpdateOp::UpdateGlobalLimits(limits(3))
            .apply_to_session(&mut session)
            .expect("global limits reference nothing");
        SettingsUpdateOp::UpdateStudentLimits(student, limits(5))
            .apply_to_session(&mut session)
            .expect("the student is live");

        assert_eq!(
            session
                .get_data()
                .get_inner_data()
                .params
                .settings
                .students
                .get(&student),
            Some(&limits(5)),
        );

        // The removal's *both* prechecks read the session: the student it needs
        // and the override it needs were put there by this very session, not by
        // the document it started on.
        SettingsUpdateOp::RemoveStudentLimits(student)
            .apply_to_session(&mut session)
            .expect("the override the previous op set is there to remove");
        let (state, warnings) = session.commit((
            OpCategory::Settings,
            "Régler puis retirer les limites d'une élève".into(),
        ));

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(settings_of(&state).global, limits(3));
        assert!(
            settings_of(&state).students.is_empty(),
            "removing the override should remove the entry, not blank it"
        );
    }

    /// The whole error surface of the family, all three of it ops-level: two
    /// address checks on the student, and the absent-override detection the
    /// elementary op cannot make (`SetStudent(_, None)` on a student without an
    /// override is a silent no-op there).
    #[test]
    fn the_three_prechecks_reject_and_change_nothing() {
        let (state, student) = one_student();
        let dangling = dangling_student();

        let mut session = CascadeSession::new(state);
        let before = session.get_data().clone();

        assert_eq!(
            SettingsUpdateOp::UpdateStudentLimits(dangling, limits(5))
                .apply_to_session(&mut session)
                .unwrap_err(),
            SettingsUpdateError::UpdateStudentLimits(UpdateStudentLimitsError::InvalidStudentId(
                dangling
            )),
        );
        assert_eq!(
            SettingsUpdateOp::RemoveStudentLimits(dangling)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SettingsUpdateError::RemoveStudentLimits(RemoveStudentLimitsError::InvalidStudentId(
                dangling
            )),
        );
        assert_eq!(
            SettingsUpdateOp::RemoveStudentLimits(student)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SettingsUpdateError::RemoveStudentLimits(RemoveStudentLimitsError::NoLimitsForStudent(
                student
            )),
        );

        // A rejection is decided before any elementary op is issued, so the
        // document — and the warning log — are exactly as they were.
        assert_eq!(session.get_data(), &before);
        let (_state, warnings) = session.commit((OpCategory::Settings, "Rien".into()));
        assert!(warnings.is_empty(), "nothing was applied: {warnings:?}");
    }
}
