use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BalancingUpdateOp {
    UpdateGlobalOptions(collomatique_state_colloscopes::balancing::BalancingOptions),
    UpdateSubjectOptions(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::balancing::BalancingOptions,
    ),
    RemoveSubjectOptions(collomatique_state_colloscopes::SubjectId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum BalancingUpdateError {
    #[error(transparent)]
    UpdateSubjectOptions(#[from] UpdateSubjectOptionsError),
    #[error(transparent)]
    RemoveSubjectOptions(#[from] RemoveSubjectOptionsError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateSubjectOptionsError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    /// The subject exists but runs no interrogations, and only an interrogated
    /// subject may carry balancing options (the checker's
    /// `BalancingForSubjectWithoutInterrogations`).
    #[error("Subject {0:?} has interrogations disabled: it cannot have balancing options")]
    SubjectHasNoInterrogation(collomatique_state_colloscopes::SubjectId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemoveSubjectOptionsError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("No options defined for subject {0:?}")]
    NoOptionsForSubject(collomatique_state_colloscopes::SubjectId),
}

impl BalancingUpdateOp {
    pub(crate) fn apply_to_session<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<(), BalancingUpdateError> {
        match self {
            Self::UpdateGlobalOptions(options) => {
                // The global options name no entity, so this op can neither be
                // rejected nor make the cascade repair anything.
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Balancing(
                            collomatique_state_colloscopes::BalancingOp::SetGlobal(options.clone()),
                        ),
                        self.get_desc(),
                    )
                    .expect("BalancingOp::SetGlobal should never fail");

                assert!(result.is_none());

                Ok(())
            }
            Self::UpdateSubjectOptions(subject_id, options) => {
                // An ops-level address check: it decides whether there is an op
                // to issue at all, so it stays here rather than being read back
                // out of the state layer's precheck error.
                let interrogated = match session
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                {
                    Some(subject) => subject.parameters.interrogation_parameters.is_some(),
                    None => {
                        return Err(UpdateSubjectOptionsError::InvalidSubjectId(*subject_id).into());
                    }
                };

                // And a content check on top of it, because a live subject is
                // not enough: only an interrogated subject may carry balancing
                // options (the checker's
                // `BalancingForSubjectWithoutInterrogations`). Without this the
                // op reaches the state layer, the map answers `None` for the
                // break — the rolled-back entry is not in the state, so there
                // is nothing to repair — the target is convicted, and the
                // `.expect` below kills the process on data-dependent input.
                if !interrogated {
                    return Err(
                        UpdateSubjectOptionsError::SubjectHasNoInterrogation(*subject_id).into(),
                    );
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Balancing(
                            collomatique_state_colloscopes::BalancingOp::SetSubject(
                                *subject_id,
                                Some(options.clone()),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("BalancingOp::SetSubject should not fail on a checked subject id");

                assert!(result.is_none());

                Ok(())
            }
            Self::RemoveSubjectOptions(subject_id) => {
                if session
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                    .is_none()
                {
                    return Err(RemoveSubjectOptionsError::InvalidSubjectId(*subject_id).into());
                }

                // `SetSubject(_, None)` is a no-op on a subject without an
                // override, so the absence is detected here rather than by the
                // elementary op.
                if !session
                    .get_data()
                    .get_inner_data()
                    .params
                    .balancing
                    .subjects
                    .contains(subject_id)
                {
                    return Err(RemoveSubjectOptionsError::NoOptionsForSubject(*subject_id).into());
                }

                // Dropping an override cannot break anything either: the entry
                // it removes is the balancing table's only reference.
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Balancing(
                            collomatique_state_colloscopes::BalancingOp::SetSubject(
                                *subject_id,
                                None,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("BalancingOp::SetSubject should not fail on a checked subject id");

                assert!(result.is_none());

                Ok(())
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Balancing,
            match self {
                BalancingUpdateOp::UpdateGlobalOptions(_) => {
                    "Mettre à jour les paramètres généraux d'équilibrage".into()
                }
                BalancingUpdateOp::UpdateSubjectOptions(_, _) => {
                    "Mettre à jour les paramètres d'équilibrage d'une matière".into()
                }
                BalancingUpdateOp::RemoveSubjectOptions(_) => {
                    "Supprimer les paramètres d'équilibrage d'une matière".into()
                }
            },
        )
    }
}

#[cfg(test)]
mod tests {
    //! The balancing table references exactly one thing — the subject a
    //! per-subject override is keyed by — so a document holding one subject
    //! says everything this family has to say, and the frozen hogwarts base
    //! (`tests/fixtures/`) would only add noise.
    //!
    //! Two properties are worth the reading: the four ops-level prechecks (the
    //! whole error surface of the family — the state layer's own
    //! `BalancingPrecheckError::InvalidSubjectId` is unreachable behind the
    //! ops-level subject check, and the cascade never repairs anything), and
    //! the fact that those prechecks read the *session's* document rather than
    //! the state the composite started on.
    //!
    //! The subject the happy paths use runs interrogations, because that is
    //! what an override needs: the checker's
    //! `BalancingForSubjectWithoutInterrogations` says a subject with
    //! interrogations disabled may not carry one. The fourth precheck is what
    //! turns that corner into a rejection instead of a dead process, and
    //! `an_override_on_a_subject_without_interrogations_is_rejected` pins it.

    use super::*;
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::balancing::BalancingOptions;
    use collomatique_state_colloscopes::soft_param::SoftParam;
    use collomatique_state_colloscopes::{
        NewId, NonEmptyRangeInclusive, Op, Subject, SubjectInterrogationParameters, SubjectOp,
        SubjectParameters, SubjectPeriodicity,
        ids::{Id, SubjectId},
    };
    use std::num::NonZeroU32;

    /// A document with one subject and no balancing entry of any kind — the
    /// whole state these tests need to read. Whether that subject runs
    /// interrogations is the caller's choice, because it is the difference
    /// between an override the checker accepts and one it forbids.
    fn one_subject_with(
        name: &str,
        interrogation_parameters: Option<SubjectInterrogationParameters>,
    ) -> (AppState<Data, Desc>, SubjectId) {
        let mut state = AppState::new(Data::default());
        let new_id = state
            .apply(
                Op::Subject(SubjectOp::AddAfter(
                    None,
                    Subject {
                        parameters: SubjectParameters {
                            name: name.into(),
                            interrogation_parameters,
                        },
                        excluded_periods: std::collections::BTreeSet::new(),
                        week_pattern: None,
                    },
                )),
                (OpCategory::Subjects, "Ajouter une matière".into()),
            )
            .expect("a subject attached to nothing breaks nothing");
        let Some(NewId::SubjectId(subject_id)) = new_id else {
            panic!("adding a subject should return a subject id, got {new_id:?}");
        };

        (state, subject_id)
    }

    /// The document the happy paths run on: its subject runs interrogations, so
    /// it may carry a balancing override.
    fn one_subject() -> (AppState<Data, Desc>, SubjectId) {
        one_subject_with(
            "Potions",
            Some(SubjectInterrogationParameters {
                students_per_group: NonEmptyRangeInclusive::new(
                    NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
                )
                .expect("statically non-empty"),
                groups_per_interrogation: NonEmptyRangeInclusive::new(
                    NonZeroU32::new(1).unwrap()..=NonZeroU32::new(1).unwrap(),
                )
                .expect("statically non-empty"),
                duration: collomatique_time::NonZeroMinutes::new(60).unwrap(),
                take_duration_into_account: true,
                periodicity: SubjectPeriodicity::ExactlyPeriodic {
                    periodicity_in_weeks: NonZeroU32::new(2).unwrap(),
                },
            }),
        )
    }

    /// A subject with interrogations *disabled* — live, so the address check
    /// passes, but forbidden to carry a balancing override.
    fn one_subject_without_interrogations() -> (AppState<Data, Desc>, SubjectId) {
        one_subject_with("Vol sur balai", None)
    }

    /// An id no document ever issued.
    fn dangling_subject() -> SubjectId {
        unsafe { SubjectId::new(1u64 << 40) }
    }

    /// Options distinguishable from the default ones, and from each other.
    fn options(avoid_twice_in_a_row: bool) -> BalancingOptions {
        BalancingOptions {
            teacher_rotation: Some(SoftParam {
                soft: false,
                value: (),
            }),
            slot_rotation: None,
            avoid_twice_in_a_row: avoid_twice_in_a_row.then_some(SoftParam {
                soft: true,
                value: (),
            }),
            year_teacher_rotation: true,
            period_teacher_rotation: false,
        }
    }

    fn balancing_of<T: Manager<Data = Data, Desc = Desc>>(
        state: &T,
    ) -> collomatique_state_colloscopes::balancing::Balancing {
        state.get_data().get_inner_data().params.balancing.clone()
    }

    /// The global options name no entity: the op lands as issued, and there is
    /// nothing for the cascade to repair.
    #[test]
    fn global_options_land_untouched_and_warn_about_nothing() {
        let (state, _subject) = one_subject();

        let mut session = CascadeSession::new(state);
        let op = BalancingUpdateOp::UpdateGlobalOptions(options(false));
        op.apply_to_session(&mut session)
            .expect("global options reference nothing, so nothing can reject them");
        let (state, warnings) = session.commit(op.get_desc());

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(balancing_of(&state).global, options(false));
        assert!(
            balancing_of(&state).subjects.is_empty(),
            "the global options are not a per-subject override"
        );
    }

    /// A per-subject override is set, then removed, on a live subject. The
    /// override table is sparse: removing the override removes the entry
    /// itself, and the global options are untouched throughout.
    #[test]
    fn a_subject_override_is_set_then_removed() {
        let (state, subject) = one_subject();

        let mut session = CascadeSession::new(state);
        BalancingUpdateOp::UpdateGlobalOptions(options(false))
            .apply_to_session(&mut session)
            .expect("global options reference nothing");
        BalancingUpdateOp::UpdateSubjectOptions(subject, options(true))
            .apply_to_session(&mut session)
            .expect("the subject is live and runs interrogations");

        assert_eq!(
            session
                .get_data()
                .get_inner_data()
                .params
                .balancing
                .subjects
                .get(&subject),
            Some(&options(true)),
        );

        // The removal's *both* prechecks read the session: the subject it needs
        // and the override it needs were put there by this very session, not by
        // the document it started on.
        BalancingUpdateOp::RemoveSubjectOptions(subject)
            .apply_to_session(&mut session)
            .expect("the override the previous op set is there to remove");
        let (state, warnings) = session.commit((
            OpCategory::Balancing,
            "Régler puis retirer l'équilibrage d'une matière".into(),
        ));

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(balancing_of(&state).global, options(false));
        assert!(
            balancing_of(&state).subjects.is_empty(),
            "removing the override should remove the entry, not blank it"
        );
    }

    /// The whole error surface of the family, all three of it ops-level: two
    /// address checks on the subject, and the absent-override detection the
    /// elementary op cannot make (`SetSubject(_, None)` on a subject without an
    /// override is a silent no-op there).
    #[test]
    fn the_three_prechecks_reject_and_change_nothing() {
        let (state, subject) = one_subject();
        let dangling = dangling_subject();

        let mut session = CascadeSession::new(state);
        let before = session.get_data().clone();

        assert_eq!(
            BalancingUpdateOp::UpdateSubjectOptions(dangling, options(true))
                .apply_to_session(&mut session)
                .unwrap_err(),
            BalancingUpdateError::UpdateSubjectOptions(
                UpdateSubjectOptionsError::InvalidSubjectId(dangling)
            ),
        );
        assert_eq!(
            BalancingUpdateOp::RemoveSubjectOptions(dangling)
                .apply_to_session(&mut session)
                .unwrap_err(),
            BalancingUpdateError::RemoveSubjectOptions(
                RemoveSubjectOptionsError::InvalidSubjectId(dangling)
            ),
        );
        assert_eq!(
            BalancingUpdateOp::RemoveSubjectOptions(subject)
                .apply_to_session(&mut session)
                .unwrap_err(),
            BalancingUpdateError::RemoveSubjectOptions(
                RemoveSubjectOptionsError::NoOptionsForSubject(subject)
            ),
        );

        // A rejection is decided before any elementary op is issued, so the
        // document — and the warning log — are exactly as they were.
        assert_eq!(session.get_data(), &before);
        let (_state, warnings) = session.commit((OpCategory::Balancing, "Rien".into()));
        assert!(warnings.is_empty(), "nothing was applied: {warnings:?}");
    }

    /// A balancing override on a subject whose interrogations are *disabled* is
    /// forbidden by the checker (`BalancingForSubjectWithoutInterrogations`).
    /// The subject is live, so the address check passes and the op reaches the
    /// state layer, which convicts it — the user must get a typed rejection
    /// back, not a dead process.
    #[test]
    fn an_override_on_a_subject_without_interrogations_is_rejected() {
        let (state, subject) = one_subject_without_interrogations();

        let mut session = CascadeSession::new(state);
        let before = session.get_data().clone();

        assert_eq!(
            BalancingUpdateOp::UpdateSubjectOptions(subject, options(true))
                .apply_to_session(&mut session)
                .unwrap_err(),
            BalancingUpdateError::UpdateSubjectOptions(
                UpdateSubjectOptionsError::SubjectHasNoInterrogation(subject)
            ),
        );

        // Rejected before any elementary op is issued, like every other
        // precheck of the family: the document is exactly as it was.
        assert_eq!(session.get_data(), &before);
    }
}
