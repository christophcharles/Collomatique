use std::collections::BTreeSet;

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubjectsUpdateOp {
    AddNewSubject(collomatique_state_colloscopes::subjects::SubjectParameters),
    UpdateSubject(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::subjects::SubjectParameters,
    ),
    DeleteSubject(collomatique_state_colloscopes::SubjectId),
    MoveSubjectUp(collomatique_state_colloscopes::SubjectId),
    MoveSubjectDown(collomatique_state_colloscopes::SubjectId),
    UpdatePeriodStatus(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
        bool,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubjectsUpdateError {
    #[error(transparent)]
    UpdateSubject(#[from] UpdateSubjectError),
    #[error(transparent)]
    DeleteSubject(#[from] DeleteSubjectError),
    #[error(transparent)]
    MoveSubjectUp(#[from] MoveSubjectUpError),
    #[error(transparent)]
    MoveSubjectDown(#[from] MoveSubjectDownError),
    #[error(transparent)]
    UpdatePeriodStatus(#[from] UpdatePeriodStatusError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateSubjectError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeleteSubjectError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum MoveSubjectUpError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Subject is already the first subject")]
    NoUpperPosition,
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum MoveSubjectDownError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Subject is already the last subject")]
    NoLowerPosition,
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdatePeriodStatusError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

impl SubjectsUpdateOp {
    pub(crate) fn apply_to_session<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<Option<collomatique_state_colloscopes::SubjectId>, SubjectsUpdateError> {
        match self {
            Self::AddNewSubject(params) => {
                let last_subject = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .ordered_subject_list
                    .iter()
                    .last()
                    .map(|(id, _)| id);

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::AddAfter(
                                last_subject,
                                collomatique_state_colloscopes::Subject {
                                    parameters: params.clone(),
                                    excluded_periods: BTreeSet::new(),
                                },
                            ),
                        ),
                        self.get_desc(),
                    )
                    // A brand new subject runs on every period, so it names no
                    // period; and nothing in the document can name *it* yet. The
                    // anchor is the list's own last subject, read one line above.
                    .expect("a subject nothing names yet contradicts nothing");
                let Some(collomatique_state_colloscopes::NewId::SubjectId(new_id)) = result else {
                    panic!("Unexpected result from SubjectOp::AddAfter");
                };
                Ok(Some(new_id))
            }
            Self::UpdateSubject(subject_id, params) => {
                // An address check, not a cleaning guard: the op carries the
                // subject's *parameters* only, so the excluded-period set has to
                // be read off the live subject to be written back unchanged.
                let current_subject = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                    .ok_or(UpdateSubjectError::InvalidSubjectId(*subject_id))?;

                let excluded_periods = current_subject.excluded_periods.clone();

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::Update(
                                *subject_id,
                                collomatique_state_colloscopes::Subject {
                                    parameters: params.clone(),
                                    excluded_periods,
                                },
                            ),
                        ),
                        self.get_desc(),
                    )
                    // Everything new parameters can contradict is material that
                    // was already there, so the cascade repairs all of it: the
                    // teachers declared on a subject that no longer holds colles
                    // lose it, its slots go, its group-list associations, its
                    // balancing options and the pairing rules naming it are
                    // dropped. That includes the case the
                    // old body could not survive — a longer interrogation that
                    // would push a late slot past midnight kills the slot now
                    // (`Fix::DeleteOverflowingSlot`) instead of the process.
                    .expect("the cascade repairs whatever new parameters contradict");

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteSubject(subject_id) => {
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::Remove(*subject_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, InvalidOp, PrecheckError, SubjectPrecheckError,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Subject(se))) => {
                                match se {
                                    SubjectPrecheckError::InvalidSubjectId(id) => {
                                        DeleteSubjectError::InvalidSubjectId(*id)
                                    }
                                    SubjectPrecheckError::SubjectIdAlreadyExists(_)
                                    | SubjectPrecheckError::PositionOutOfBounds { .. } => panic!(
                                        "Unexpected SubjectPrecheckError during DeleteSubject: {e:?}"
                                    ),
                                }
                            }
                            // The old body listed here the seven cleaning phases
                            // that had to empty the way before the removal could
                            // land. All seven are the cascade's business now —
                            // teacher subject lists, slots, incompatibilities,
                            // pairing rules, balancing options, assignments rows
                            // and group-list associations — and each repair is
                            // logged as a warning.
                            _ => panic!("Unexpected error during DeleteSubject: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::MoveSubjectUp(subject_id) => {
                let current_position = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject_position(*subject_id)
                    .ok_or(MoveSubjectUpError::InvalidSubjectId(*subject_id))?;

                if current_position == 0 {
                    Err(MoveSubjectUpError::NoUpperPosition)?;
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::ChangePosition(
                                *subject_id,
                                current_position - 1,
                            ),
                        ),
                        self.get_desc(),
                    )
                    // The list order is presentation: nothing in the document
                    // depends on where a subject sits in it. Both the id and the
                    // target position are checked just above.
                    .expect("moving a subject in the list contradicts nothing");

                assert!(result.is_none());

                Ok(None)
            }
            Self::MoveSubjectDown(subject_id) => {
                let current_position = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject_position(*subject_id)
                    .ok_or(MoveSubjectDownError::InvalidSubjectId(*subject_id))?;

                if current_position
                    == session
                        .get_data()
                        .get_inner_data()
                        .params
                        .subjects
                        .ordered_subject_list
                        .len()
                        - 1
                {
                    Err(MoveSubjectDownError::NoLowerPosition)?;
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::ChangePosition(
                                *subject_id,
                                current_position + 1,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("moving a subject in the list contradicts nothing");

                assert!(result.is_none());

                Ok(None)
            }
            Self::UpdatePeriodStatus(subject_id, period_id, new_status) => {
                // Both are address checks the composite needs before it can
                // build its op, and their order is the surface: a call naming
                // two dead ids reports the period, as it always did.
                if session
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_none()
                {
                    Err(UpdatePeriodStatusError::InvalidPeriodId(*period_id))?;
                }

                let mut subject = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                    .ok_or(UpdatePeriodStatusError::InvalidSubjectId(*subject_id))?
                    .clone();

                if *new_status {
                    subject.excluded_periods.remove(period_id);
                } else {
                    subject.excluded_periods.insert(*period_id);
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::Update(*subject_id, subject),
                        ),
                        self.get_desc(),
                    )
                    // Taking the subject off a period cannot be refused: the
                    // enrolments, the colles and the group-list association it
                    // holds there are all pre-existing material, and the cascade
                    // drops exactly them. Putting it back on contradicts nothing
                    // at all.
                    .expect("the cascade repairs whatever a period exclusion contradicts");

                assert!(result.is_none());

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Subjects,
            match self {
                SubjectsUpdateOp::AddNewSubject(_desc) => "Ajouter une matière".into(),
                SubjectsUpdateOp::UpdateSubject(_id, _desc) => "Modifier une matière".into(),
                SubjectsUpdateOp::DeleteSubject(_id) => "Supprimer une matière".into(),
                SubjectsUpdateOp::MoveSubjectUp(_id) => "Remonter une matière".into(),
                SubjectsUpdateOp::MoveSubjectDown(_id) => "Descendre une matière".into(),
                Self::UpdatePeriodStatus(_subject_id, _period_id, status) => {
                    if *status {
                        "Dispenser une matière sur une période".into()
                    } else {
                        "Ne pas dispenser une matière sur une période".into()
                    }
                }
            },
        )
    }
}

#[cfg(test)]
mod tests {
    //! Nearly everything in the document points at a subject: the teachers who
    //! hold its colles, its slots, the incompatibilities built on its schedule,
    //! the pairing rules relating it to another subject, its balancing
    //! override, its enrolment rows and its group-list associations. Eight
    //! reference sites, more than any other entity has — and the subject itself
    //! points only at the periods it skips.
    //!
    //! That is why this module carried the heaviest cleaning phase of the
    //! fifteen: seven phases before a removal could land, four before
    //! interrogations could be switched off, three before a period could be
    //! dropped. It is also the family where the old cleaning and the cascade
    //! line up almost one for one — the fixtures below are what says so, effect
    //! by effect.
    //!
    //! What they also say, twice, is that the *order* of the repairs is not the
    //! canonical order of the invariants they answer. The engine is depth-first
    //! and rolls the failing op back while it looks for a fix, so a repair that
    //! cannot land yet has its own repairs land before it: striking a subject
    //! off a teacher's list is refused while that teacher still holds the
    //! subject's slots, so the slots go first. Both the removal fixture and the
    //! interrogation one pin that inversion.
    //!
    //! One thing genuinely changes here: lengthening an interrogation over a
    //! slot too late in the day used to kill the process on
    //! `.expect("All data should be valid at this point")`. The slot is removed
    //! and reported now.
    //!
    //! The frozen hogwarts base carries seven of the eight reference sites out
    //! of the box; it holds no pairing rule at all, so the removal fixture adds
    //! two of them — one naming Potions on each side — and an incompatibility,
    //! in plain sight at its top.

    use super::*;
    use crate::test_utils::{fixes, hogwarts};
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::{
        AssignmentOp, BalancingOp, ColloscopeOp, Fix, GroupListOp, IncompatOp, NewId, Op,
        PairingOp, SlotOp, SubjectOp, TeacherOp,
        ids::{Id, IncompatId, PeriodId, SlotId, SubjectId, TeacherId, WeekId},
        pairings::{PairingRule, RulePart},
        subjects::{Subject, SubjectParameters},
        teachers::Teacher,
    };

    fn subject_by_name(data: &Data, name: &str) -> SubjectId {
        data.get_inner_data()
            .params
            .subjects
            .ordered_subject_list
            .iter()
            .find(|(_id, subject)| subject.parameters.name == name)
            .map(|(id, _subject)| id)
            .unwrap_or_else(|| panic!("the fixture should have a subject named {name}"))
    }

    fn subject_of(data: &Data, subject: SubjectId) -> Subject {
        data.get_inner_data()
            .params
            .subjects
            .find_subject(subject)
            .expect("the fixture's subject should be live")
            .clone()
    }

    fn params_of(data: &Data, subject: SubjectId) -> SubjectParameters {
        subject_of(data, subject).parameters
    }

    /// The subject list in display order — what the two move ops shuffle.
    fn subject_order(data: &Data) -> Vec<SubjectId> {
        data.get_inner_data()
            .params
            .subjects
            .ordered_subject_list
            .iter()
            .map(|(id, _subject)| id)
            .collect()
    }

    fn teacher_of(data: &Data, teacher: TeacherId) -> Teacher {
        data.get_inner_data()
            .params
            .teachers
            .teacher_map
            .get(&teacher)
            .expect("the fixture's teacher should be live")
            .clone()
    }

    /// The teachers declared on `subject`, in id order — the order the
    /// reference site carries.
    fn teachers_of_subject(data: &Data, subject: SubjectId) -> Vec<TeacherId> {
        data.get_inner_data()
            .params
            .teachers
            .teacher_map
            .iter()
            .filter(|(_id, teacher)| teacher.subjects.contains(&subject))
            .map(|(id, _teacher)| id)
            .collect()
    }

    /// `teacher` with `subject` struck off their list — the value the cascade
    /// is expected to write back. The teacher stays: a subject leaving the set
    /// is removing content, not removing the teacher.
    fn teacher_without(data: &Data, teacher: TeacherId, subject: SubjectId) -> Teacher {
        let mut rebuilt = teacher_of(data, teacher);
        rebuilt.subjects.remove(&subject);

        rebuilt
    }

    /// The subject's slots **in id order**, which is deliberately not their
    /// display order (hogwarts lists the Potions slots 148, 145, 146, 147): the
    /// reference site carries the slot id, so this is the order the cascade
    /// meets them in.
    fn slots_of_subject(data: &Data, subject: SubjectId) -> Vec<SlotId> {
        let mut slots: Vec<_> = data
            .get_inner_data()
            .params
            .slots
            .slots_for_subject(subject)
            .into_iter()
            .flatten()
            .map(|(id, _slot)| *id)
            .collect();
        slots.sort();

        slots
    }

    /// The slots of `subject` that an interrogation of `duration` minutes would
    /// push past the end of their day — the checker's
    /// `Convergence::SlotOverflowsDay`, derived here rather than read off a
    /// timetable by hand.
    fn slots_overflowing_with(
        data: &Data,
        subject: SubjectId,
        duration: collomatique_time::NonZeroMinutes,
    ) -> Vec<SlotId> {
        slots_of_subject(data, subject)
            .into_iter()
            .filter(|slot| {
                let start = data
                    .get_inner_data()
                    .params
                    .slots
                    .find_slot(*slot)
                    .expect("just listed among the subject's slots")
                    .start_time
                    .clone();
                collomatique_time::SlotWithDuration::new(start, duration).is_none()
            })
            .collect()
    }

    /// The incompatibilities built on `subject`, in id order.
    fn incompats_of_subject(data: &Data, subject: SubjectId) -> Vec<IncompatId> {
        data.get_inner_data()
            .params
            .incompats
            .incompat_map
            .iter()
            .filter(|(_id, incompat)| incompat.subject_id == subject)
            .map(|(id, _incompat)| id)
            .collect()
    }

    /// The periods `subject` has an enrolment row on, in key order — the order
    /// the reference site carries.
    fn rows_of_subject(data: &Data, subject: SubjectId) -> Vec<PeriodId> {
        data.get_inner_data()
            .params
            .assignments
            .iter()
            .filter(|(_period, row_subject, _students)| *row_subject == subject)
            .map(|(period, _subject, _students)| period)
            .collect()
    }

    /// The periods `subject` uses a group list on, in key order.
    fn associations_of_subject(data: &Data, subject: SubjectId) -> Vec<PeriodId> {
        data.get_inner_data()
            .params
            .group_lists
            .subjects_associations
            .iter()
            .filter(|((_period, assoc_subject), _group_list)| *assoc_subject == subject)
            .map(|((period, _subject), _group_list)| period)
            .collect()
    }

    /// The `n`-th period in display order.
    fn period_at(data: &Data, index: usize) -> PeriodId {
        data.get_inner_data()
            .params
            .periods
            .period_ids()
            .nth(index)
            .unwrap_or_else(|| panic!("the fixture should have at least {} periods", index + 1))
    }

    /// The weeks of `period` a colle may be written on for `slot`, in id order.
    fn writable_weeks(data: &Data, slot: SlotId, period: PeriodId) -> Vec<WeekId> {
        let params = &data.get_inner_data().params;
        let mut weeks: Vec<_> = params
            .week_ids()
            .filter(|week| {
                params.weeks.week_position(*week).map(|(p, _pos)| p) == Some(period)
                    && params.is_interrogation_possible(slot, *week)
            })
            .collect();
        weeks.sort();

        weeks
    }

    /// A pairing rule between two subjects, excluding no period.
    fn rule(antecedent: SubjectId, consequent: SubjectId) -> PairingRule {
        PairingRule::new(
            RulePart {
                subject_id: antecedent,
                should_have: true,
            },
            RulePart {
                subject_id: consequent,
                should_have: true,
            },
            BTreeSet::new(),
            false,
        )
        .expect("the fixtures never name one subject in both parts")
    }

    /// Ids no document ever issued.
    fn dangling_subject() -> SubjectId {
        unsafe { SubjectId::new(1u64 << 40) }
    }

    fn dangling_period() -> PeriodId {
        unsafe { PeriodId::new(1u64 << 40) }
    }

    /// Replays `ops` on a clone of `base`: the document a fixture expects,
    /// written as the elementary ops it expects the cascade to have landed —
    /// each of them valid in that order, exactly as the cascade lands them.
    fn expected_document(base: &AppState<Data, Desc>, ops: Vec<Op>) -> AppState<Data, Desc> {
        let mut expected = base.clone();
        for op in ops {
            expected
                .apply(op, (OpCategory::Subjects, "Expected".into()))
                .expect("each expected op lands in the order the cascade landed it");
        }

        expected
    }

    /// Runs one op alone on `base` and hands back what the document became and
    /// what the cascade had to repair on the way.
    fn apply_alone(
        base: &AppState<Data, Desc>,
        op: &SubjectsUpdateOp,
    ) -> (AppState<Data, Desc>, Vec<CascadeWarning>) {
        let mut session = CascadeSession::new(base.clone());
        op.apply_to_session(&mut session)
            .unwrap_or_else(|e| panic!("{op:?} should land, got {e:?}"));

        session.commit(op.get_desc())
    }

    /// A new subject runs on every period and is named by nobody, so it can
    /// cost nothing: the id comes back, the log stays empty, and the subject
    /// lands at the end of the list.
    #[test]
    fn adding_a_subject_creates_it_at_the_end_and_warns_about_nothing() {
        let base = hogwarts();
        let mut added = params_of(base.get_data(), subject_by_name(base.get_data(), "Potions"));
        added.name = "Botanique".into();

        let mut session = CascadeSession::new(base.clone());
        let op = SubjectsUpdateOp::AddNewSubject(added.clone());
        let new_id = op
            .apply_to_session(&mut session)
            .expect("a fresh subject names nothing");
        let (state, warnings) = session.commit(op.get_desc());

        let new_id = new_id.expect("adding a subject returns the id it issued");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            subject_of(state.get_data(), new_id),
            Subject {
                parameters: added,
                excluded_periods: BTreeSet::new(),
            },
        );
        assert_eq!(
            subject_order(state.get_data()).last(),
            Some(&new_id),
            "a new subject goes after the list's last one",
        );
    }

    /// Renaming touches nothing anything else depends on. Worth its own
    /// fixture because the op replaces the *whole* subject: the excluded-period
    /// set the payload does not carry has to be read off the live subject and
    /// written back untouched.
    #[test]
    fn renaming_a_subject_warns_about_nothing() {
        let mut base = hogwarts();
        let potions = subject_by_name(base.get_data(), "Potions");
        let first_period = period_at(base.get_data(), 0);

        // A subject that skips a period, so the fixture can see the set survive
        // an update that says nothing about it. Emptying the enrolment row and
        // dropping the association first is what makes the exclusion legal.
        base.apply(
            Op::Assignment(AssignmentOp::SetRow(first_period, potions, BTreeSet::new())),
            (OpCategory::Assignments, "Préparation".into()),
        )
        .expect("emptying an enrolment row breaks nothing");
        base.apply(
            Op::GroupList(GroupListOp::AssignToSubject(first_period, potions, None)),
            (OpCategory::GroupLists, "Préparation".into()),
        )
        .expect("dropping an association breaks nothing");
        base.apply(
            Op::Subject(SubjectOp::Update(
                potions,
                Subject {
                    parameters: params_of(base.get_data(), potions),
                    excluded_periods: BTreeSet::from([first_period]),
                },
            )),
            (OpCategory::Subjects, "Préparation".into()),
        )
        .expect("the period was emptied of everything the subject had on it");

        let mut renamed = params_of(base.get_data(), potions);
        renamed.name = "Potions et poisons".into();

        let op = SubjectsUpdateOp::UpdateSubject(potions, renamed.clone());
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::Subject(SubjectOp::Update(
                    potions,
                    Subject {
                        parameters: renamed,
                        excluded_periods: BTreeSet::from([first_period]),
                    },
                ))],
            )
            .get_data(),
        );
    }

    /// Switching a subject's interrogations off is the update that costs the
    /// most: a subject without colles may not be taught by anyone, may not hold
    /// slots, may not use a group list and may not carry balancing options of
    /// its own. The old body cleaned those four in that order, one scan each;
    /// the cascade answers the same four effects.
    ///
    /// The order it reports them in is **not** the canonical order of the four
    /// convergences, and this fixture is where that is worth reading. The
    /// checker names the teacher first, so the engine tries to strike Potions
    /// off Rogue's list — but the failing target is rolled back while a fix is
    /// tried, so Potions still holds interrogations at that moment, and Rogue
    /// still holds its four slots: the teacher update breaks
    /// `SlotTeacherDoesNotTeachSubject` and cannot land. The engine is
    /// depth-first, so the slots go first, as that fix's own sub-fixes, and the
    /// teacher update lands on the retry. Then the target is retried, and what
    /// is left of the four — the associations, then the balancing override —
    /// answers in canonical order.
    ///
    /// A pairing rule naming the subject goes the same way, and lands **last**:
    /// its two convergences are declared after the balancing one, so the
    /// canonical pick reaches them once everything else is repaired. The
    /// fixture holds no rule of its own, so this one is seeded in plain sight
    /// before the target is applied.
    ///
    /// The enrolments deliberately survive: being registered in a subject says
    /// nothing about having colles in it, and no convergence relates the two.
    #[test]
    fn disabling_interrogations_dismantles_everything_that_needed_them() {
        let mut base = hogwarts();
        let potions = subject_by_name(base.get_data(), "Potions");
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let NewId::PairingRuleId(pairing) = base
            .apply(
                Op::Pairing(PairingOp::Add(rule(potions, metamorphose))),
                (OpCategory::Pairings, "Amorce".into()),
            )
            .expect("both subjects are live and run interrogations")
            .expect("adding a pairing rule issues an id")
        else {
            panic!("PairingOp::Add should issue a pairing rule id");
        };
        let teachers = teachers_of_subject(base.get_data(), potions);
        let slots = slots_of_subject(base.get_data(), potions);
        let associations = associations_of_subject(base.get_data(), potions);
        assert_eq!(teachers.len(), 1, "only Rogue holds the Potions colles");
        assert_eq!(slots.len(), 4);
        assert_eq!(associations.len(), 3, "one per period");
        assert!(
            base.get_data()
                .get_inner_data()
                .params
                .balancing
                .subjects
                .contains(&potions),
            "the fixture's Potions should carry balancing options of its own"
        );

        let mut without_colles = params_of(base.get_data(), potions);
        without_colles.interrogation_parameters = None;

        let op = SubjectsUpdateOp::UpdateSubject(potions, without_colles.clone());
        let (state, warnings) = apply_alone(&base, &op);

        let mut expected_fixes: Vec<_> = slots
            .iter()
            .map(|slot| Fix::DeleteSlot { slot: *slot })
            .collect();
        expected_fixes.push(Fix::RemoveTeacherSubject {
            teacher: teachers[0],
            subject: potions,
            rebuilt: teacher_without(base.get_data(), teachers[0], potions),
        });
        expected_fixes.extend(associations.iter().map(|period| Fix::UnassignGroupList {
            period: *period,
            subject: potions,
        }));
        expected_fixes.push(Fix::ClearSubjectBalancing { subject: potions });
        expected_fixes.push(Fix::DeletePairingRule { rule: pairing });
        assert_eq!(fixes(&warnings), expected_fixes);

        let mut expected_ops: Vec<_> = slots
            .iter()
            .map(|slot| Op::Slot(SlotOp::Remove(*slot)))
            .collect();
        expected_ops.push(Op::Teacher(TeacherOp::Update(
            teachers[0],
            teacher_without(base.get_data(), teachers[0], potions),
        )));
        expected_ops.extend(
            associations
                .iter()
                .map(|period| Op::GroupList(GroupListOp::AssignToSubject(*period, potions, None))),
        );
        expected_ops.push(Op::Balancing(BalancingOp::SetSubject(potions, None)));
        expected_ops.push(Op::Pairing(PairingOp::Remove(pairing)));
        expected_ops.push(Op::Subject(SubjectOp::Update(
            potions,
            Subject {
                parameters: without_colles,
                excluded_periods: BTreeSet::new(),
            },
        )));
        assert_eq!(
            state.get_data(),
            expected_document(&base, expected_ops).get_data(),
        );
        assert_eq!(
            rows_of_subject(state.get_data(), potions).len(),
            3,
            "the enrolments have nothing to do with the colles",
        );
    }

    /// A divergence from the old world. A Potions colle lasts an hour today;
    /// stretched to five and a half, the one starting at 19:00 would run past
    /// midnight — `Convergence::SlotOverflowsDay`. The slot is pre-existing
    /// material, so the map answers with [Fix::DeleteOverflowingSlot] (its own
    /// meaning: « il déborderait sur le jour suivant », not a plain deletion)
    /// and the update lands. The old body had no answer for it at all and died
    /// on `.expect("All data should be valid at this point")`.
    ///
    /// The three other Potions slots start early enough to fit, which is what
    /// makes this a choice rather than a sweep.
    #[test]
    fn lengthening_an_interrogation_over_a_late_slot_removes_that_slot() {
        let base = hogwarts();
        let potions = subject_by_name(base.get_data(), "Potions");
        let long = collomatique_time::NonZeroMinutes::new(330).expect("330 minutes is not zero");
        let overflowing = slots_overflowing_with(base.get_data(), potions, long);
        assert_eq!(
            overflowing.len(),
            1,
            "only the 19:00 Potions slot should overflow a 5h30 colle",
        );

        let mut stretched = params_of(base.get_data(), potions);
        stretched
            .interrogation_parameters
            .as_mut()
            .expect("the fixture's Potions runs interrogations")
            .duration = long;

        let op = SubjectsUpdateOp::UpdateSubject(potions, stretched.clone());
        let (state, warnings) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![Fix::DeleteOverflowingSlot {
                slot: overflowing[0],
            }],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![
                    Op::Slot(SlotOp::Remove(overflowing[0])),
                    Op::Subject(SubjectOp::Update(
                        potions,
                        Subject {
                            parameters: stretched,
                            excluded_periods: BTreeSet::new(),
                        },
                    )),
                ],
            )
            .get_data(),
        );
    }

    /// Taking a subject off a period drops the three things it held there: the
    /// enrolments, the group list it used, and the colles already written on
    /// the period's weeks. The old body cleaned exactly those three, one scan
    /// each, in that order.
    ///
    /// The order is the canonical one, and the colles sit in the middle of it
    /// on purpose: the checker declares the interrogation-row predicates ahead
    /// of the association ones precisely so this fixture reads the way it does.
    /// Were the association unassigned first, the group bound at that
    /// coordinate would fall to zero, every group of every cell would become
    /// its own out-of-bounds break, and the cells would die one group at a time
    /// — « le groupe 0 sera retiré » instead of « les colles seront supprimées
    /// ». The document would be identical; the sentence would not. That is what
    /// the declaration order in `invariants.rs` buys, and this is the fixture
    /// that would notice losing it.
    ///
    /// Hogwarts carries no colloscope, so the setup writes the two colles this
    /// fixture is about on the first Potions slot.
    #[test]
    fn taking_a_subject_off_a_period_drops_what_it_held_there() {
        let mut base = hogwarts();
        let potions = subject_by_name(base.get_data(), "Potions");
        let first_period = period_at(base.get_data(), 0);
        let slot = slots_of_subject(base.get_data(), potions)[0];
        let weeks = writable_weeks(base.get_data(), slot, first_period);
        assert_eq!(
            weeks.len(),
            2,
            "the first period opens with two weeks without colles",
        );
        for week in &weeks {
            base.apply(
                Op::Colloscope(ColloscopeOp::SetInterrogation(
                    slot,
                    *week,
                    BTreeSet::from([0]),
                )),
                (OpCategory::Colloscope, "Préparation".into()),
            )
            .expect("a group of the associated list may be placed on an active week");
        }

        let op = SubjectsUpdateOp::UpdatePeriodStatus(potions, first_period, false);
        let (state, warnings) = apply_alone(&base, &op);

        let mut expected_fixes = vec![Fix::ClearAssignmentRow {
            period: first_period,
            subject: potions,
        }];
        expected_fixes.extend(
            weeks
                .iter()
                .map(|week| Fix::ClearInterrogationCell { slot, week: *week }),
        );
        expected_fixes.push(Fix::UnassignGroupList {
            period: first_period,
            subject: potions,
        });
        assert_eq!(fixes(&warnings), expected_fixes);

        let mut expected_ops = vec![Op::Assignment(AssignmentOp::SetRow(
            first_period,
            potions,
            BTreeSet::new(),
        ))];
        expected_ops.extend(weeks.iter().map(|week| {
            Op::Colloscope(ColloscopeOp::SetInterrogation(slot, *week, BTreeSet::new()))
        }));
        expected_ops.push(Op::GroupList(GroupListOp::AssignToSubject(
            first_period,
            potions,
            None,
        )));
        expected_ops.push(Op::Subject(SubjectOp::Update(
            potions,
            Subject {
                parameters: params_of(base.get_data(), potions),
                excluded_periods: BTreeSet::from([first_period]),
            },
        )));
        assert_eq!(
            state.get_data(),
            expected_document(&base, expected_ops).get_data(),
        );
    }

    /// Putting a subject back on a period only ever *widens* what the document
    /// allows, so there is nothing to repair.
    #[test]
    fn putting_a_subject_back_on_a_period_warns_about_nothing() {
        let base = hogwarts();
        let potions = subject_by_name(base.get_data(), "Potions");
        let first_period = period_at(base.get_data(), 0);

        let mut session = CascadeSession::new(base.clone());
        SubjectsUpdateOp::UpdatePeriodStatus(potions, first_period, false)
            .apply_to_session(&mut session)
            .expect("the cascade drops what the subject held on the period");
        SubjectsUpdateOp::UpdatePeriodStatus(potions, first_period, true)
            .apply_to_session(&mut session)
            .expect("re-including a period contradicts nothing");
        let (state, warnings) = session.commit((OpCategory::Subjects, "Aller-retour".into()));

        assert_eq!(
            fixes(&warnings),
            vec![
                Fix::ClearAssignmentRow {
                    period: first_period,
                    subject: potions,
                },
                Fix::UnassignGroupList {
                    period: first_period,
                    subject: potions,
                },
            ],
            "only the first op repairs anything",
        );
        assert_eq!(
            subject_of(state.get_data(), potions).excluded_periods,
            BTreeSet::new(),
            "the subject runs on every period again",
        );
    }

    /// The removal, with all eight reference sites at once. Hogwarts gives
    /// seven of them — Rogue teaches Potions, it has four slots, its own
    /// balancing options, an enrolment row and a group list on each of the
    /// three periods — and the setup adds the eighth and its twin: an
    /// incompatibility built on Potions, and two pairing rules naming it, one
    /// on each side of the implication.
    ///
    /// Where the old body demanded that seven cleaning phases empty the way
    /// first — and panicked « Unexpected error during DeleteSubject » if one of
    /// them had missed something — the single elementary removal goes out and
    /// the cascade reports what it cost.
    ///
    /// One of the eight sites is never actually consulted here, and it is worth
    /// knowing which: the slots go as sub-fixes of the teacher's update, so the
    /// `Subject@SlotSubject` dangle arm is structurally shadowed. It always is
    /// — a slot names a teacher, and a teacher holding a slot in a subject must
    /// declare that subject, so the teacher site is present whenever the slot
    /// site is, and it is declared first. The arm answers the same
    /// [Fix::DeleteSlot] anyway, and its own pin lives in the map's unit tests.
    #[test]
    fn deleting_a_subject_takes_every_reference_to_it_with_it() {
        let mut base = hogwarts();
        let potions = subject_by_name(base.get_data(), "Potions");
        let divination = subject_by_name(base.get_data(), "Divination");

        let mut incompat = base
            .get_data()
            .get_inner_data()
            .params
            .incompats
            .incompat_map
            .values()
            .next()
            .expect("the fixture should hold at least one incompatibility")
            .clone();
        incompat.name = "Préparation des chaudrons".into();
        incompat.subject_id = potions;
        let NewId::IncompatId(incompat) = base
            .apply(
                Op::Incompat(IncompatOp::Add(incompat)),
                (OpCategory::Incompatibilities, "Préparation".into()),
            )
            .expect("an incompatibility may be built on any live subject")
            .expect("adding an incompatibility issues an id")
        else {
            panic!("IncompatOp::Add should issue an incompatibility id");
        };
        let NewId::PairingRuleId(as_antecedent) = base
            .apply(
                Op::Pairing(PairingOp::Add(rule(potions, divination))),
                (OpCategory::Pairings, "Préparation".into()),
            )
            .expect("both subjects are live")
            .expect("adding a pairing rule issues an id")
        else {
            panic!("PairingOp::Add should issue a pairing rule id");
        };
        let NewId::PairingRuleId(as_consequent) = base
            .apply(
                Op::Pairing(PairingOp::Add(rule(divination, potions))),
                (OpCategory::Pairings, "Préparation".into()),
            )
            .expect("both subjects are live")
            .expect("adding a pairing rule issues an id")
        else {
            panic!("PairingOp::Add should issue a pairing rule id");
        };

        let teachers = teachers_of_subject(base.get_data(), potions);
        let slots = slots_of_subject(base.get_data(), potions);
        let rows = rows_of_subject(base.get_data(), potions);
        let associations = associations_of_subject(base.get_data(), potions);
        assert_eq!(
            incompats_of_subject(base.get_data(), potions),
            vec![incompat],
        );
        assert_eq!(rows.len(), 3);
        assert_eq!(associations.len(), 3);

        let op = SubjectsUpdateOp::DeleteSubject(potions);
        let (state, warnings) = apply_alone(&base, &op);

        // The reference sites' declaration order — teacher, slots,
        // incompatibility, pairing rule as antecedent, pairing rule as
        // consequent, balancing options, enrolment rows, group-list
        // associations — with the same depth-first inversion of its first two
        // as the interrogation fixture: striking Potions off Rogue's list
        // cannot land while he still holds its slots, so the slots go first as
        // that fix's own sub-fixes.
        let mut expected_fixes: Vec<_> = slots
            .iter()
            .map(|slot| Fix::DeleteSlot { slot: *slot })
            .collect();
        expected_fixes.push(Fix::RemoveTeacherSubject {
            teacher: teachers[0],
            subject: potions,
            rebuilt: teacher_without(base.get_data(), teachers[0], potions),
        });
        expected_fixes.push(Fix::DeleteIncompat { incompat });
        expected_fixes.push(Fix::DeletePairingRule {
            rule: as_antecedent,
        });
        expected_fixes.push(Fix::DeletePairingRule {
            rule: as_consequent,
        });
        expected_fixes.push(Fix::ClearSubjectBalancing { subject: potions });
        expected_fixes.extend(rows.iter().map(|period| Fix::ClearAssignmentRow {
            period: *period,
            subject: potions,
        }));
        expected_fixes.extend(associations.iter().map(|period| Fix::UnassignGroupList {
            period: *period,
            subject: potions,
        }));
        assert_eq!(fixes(&warnings), expected_fixes);

        let mut expected_ops: Vec<_> = slots
            .iter()
            .map(|slot| Op::Slot(SlotOp::Remove(*slot)))
            .collect();
        expected_ops.push(Op::Teacher(TeacherOp::Update(
            teachers[0],
            teacher_without(base.get_data(), teachers[0], potions),
        )));
        expected_ops.push(Op::Incompat(IncompatOp::Remove(incompat)));
        expected_ops.push(Op::Pairing(PairingOp::Remove(as_antecedent)));
        expected_ops.push(Op::Pairing(PairingOp::Remove(as_consequent)));
        expected_ops.push(Op::Balancing(BalancingOp::SetSubject(potions, None)));
        expected_ops.extend(
            rows.iter().map(|period| {
                Op::Assignment(AssignmentOp::SetRow(*period, potions, BTreeSet::new()))
            }),
        );
        expected_ops.extend(
            associations
                .iter()
                .map(|period| Op::GroupList(GroupListOp::AssignToSubject(*period, potions, None))),
        );
        expected_ops.push(Op::Subject(SubjectOp::Remove(potions)));
        assert_eq!(
            state.get_data(),
            expected_document(&base, expected_ops).get_data(),
        );
    }

    /// The list order is presentation: moving a subject through it repairs
    /// nothing, and the two ops are exact inverses.
    #[test]
    fn moving_a_subject_reorders_the_list_and_warns_about_nothing() {
        let base = hogwarts();
        let order = subject_order(base.get_data());
        let second = order[1];

        let up = SubjectsUpdateOp::MoveSubjectUp(second);
        let (state, warnings) = apply_alone(&base, &up);
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(subject_order(state.get_data())[0], second);

        let down = SubjectsUpdateOp::MoveSubjectDown(second);
        let (state, warnings) = apply_alone(&state, &down);
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(subject_order(state.get_data()), order);
    }

    /// The two ends of the list, which the family checks itself rather than
    /// letting the state layer answer `PositionOutOfBounds`.
    #[test]
    fn moving_a_subject_past_the_ends_of_the_list_is_refused() {
        let base = hogwarts();
        let order = subject_order(base.get_data());

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            SubjectsUpdateOp::MoveSubjectUp(order[0])
                .apply_to_session(&mut session)
                .unwrap_err(),
            SubjectsUpdateError::MoveSubjectUp(MoveSubjectUpError::NoUpperPosition),
        );
        assert_eq!(
            SubjectsUpdateOp::MoveSubjectDown(*order.last().expect("the list is not empty"))
                .apply_to_session(&mut session)
                .unwrap_err(),
            SubjectsUpdateError::MoveSubjectDown(MoveSubjectDownError::NoLowerPosition),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// Every op that names an existing subject, on an id no document issued.
    /// Four of the five answer from the family's own address check; the
    /// removal is the one that lets the state layer's precheck answer and
    /// translates it. A rejected op changes nothing and logs nothing.
    #[test]
    fn a_dead_subject_id_is_rejected_by_every_op_that_names_one() {
        let base = hogwarts();
        let dangling = dangling_subject();
        let first_period = period_at(base.get_data(), 0);
        let params = params_of(base.get_data(), subject_by_name(base.get_data(), "Potions"));

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            SubjectsUpdateOp::UpdateSubject(dangling, params)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SubjectsUpdateError::UpdateSubject(UpdateSubjectError::InvalidSubjectId(dangling)),
        );
        assert_eq!(
            SubjectsUpdateOp::DeleteSubject(dangling)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SubjectsUpdateError::DeleteSubject(DeleteSubjectError::InvalidSubjectId(dangling)),
        );
        assert_eq!(
            SubjectsUpdateOp::MoveSubjectUp(dangling)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SubjectsUpdateError::MoveSubjectUp(MoveSubjectUpError::InvalidSubjectId(dangling)),
        );
        assert_eq!(
            SubjectsUpdateOp::MoveSubjectDown(dangling)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SubjectsUpdateError::MoveSubjectDown(MoveSubjectDownError::InvalidSubjectId(dangling)),
        );
        assert_eq!(
            SubjectsUpdateOp::UpdatePeriodStatus(dangling, first_period, false)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SubjectsUpdateError::UpdatePeriodStatus(UpdatePeriodStatusError::InvalidSubjectId(
                dangling
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
        let (_state, warnings) = session.commit((OpCategory::Subjects, "Rien".into()));
        assert!(warnings.is_empty(), "nothing was applied: {warnings:?}");
    }

    /// The period-status op takes two ids, and its two address checks run in a
    /// documented order: the period first. A call naming two ghosts reports the
    /// period, as it always did.
    #[test]
    fn the_period_status_op_reports_a_dead_period_before_a_dead_subject() {
        let base = hogwarts();
        let potions = subject_by_name(base.get_data(), "Potions");

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            SubjectsUpdateOp::UpdatePeriodStatus(potions, dangling_period(), false)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SubjectsUpdateError::UpdatePeriodStatus(UpdatePeriodStatusError::InvalidPeriodId(
                dangling_period()
            )),
        );
        assert_eq!(
            SubjectsUpdateOp::UpdatePeriodStatus(dangling_subject(), dangling_period(), false)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SubjectsUpdateError::UpdatePeriodStatus(UpdatePeriodStatusError::InvalidPeriodId(
                dangling_period()
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
    }
}
