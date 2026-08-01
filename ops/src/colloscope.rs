use std::collections::{BTreeMap, BTreeSet};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColloscopeUpdateWarning {}

impl ColloscopeUpdateWarning {
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
pub enum ColloscopeUpdateOp {
    UpdateColloscopeGroupList(
        collomatique_state_colloscopes::GroupListId,
        BTreeMap<collomatique_state_colloscopes::StudentId, u32>,
    ),
    UpdateColloscopeInterrogation(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::WeekId,
        BTreeSet<u32>,
    ),
    EraseColloscope,
    EraseGroupLists,
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColloscopeUpdateError {
    #[error(transparent)]
    UpdateColloscopeGroupList(#[from] UpdateColloscopeGroupListError),
    #[error(transparent)]
    UpdateColloscopeInterrogation(#[from] UpdateColloscopeInterrogationError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateColloscopeGroupListError {
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
    #[error("invalid group list id ({0:?})")]
    InvalidGroupListId(collomatique_state_colloscopes::GroupListId),
    #[error("excluded student in group list")]
    ExcludedStudentInGroupList(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::StudentId,
    ),
    #[error("Invalid group number for student")]
    InvalidGroupNumForStudentInGroupList(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::StudentId,
    ),
    /// The target list fills its groups by hand, so the colloscope has no say
    /// in it and holds no row for it.
    ///
    /// A restored error, not a new one: the old checked apply rejected such a
    /// write outright (as the overloaded [Self::InvalidGroupListId]) until
    /// step 4's `force_apply` copies dropped the guard by design, after which
    /// the condition became a plain invariant break that nothing in `ops/`
    /// named — so the op panicked instead. Emitted by `apply_to_session` only;
    /// the old `apply_no_cleaning` keeps the panic and dies with it.
    #[error("group list {0:?} is prefilled and has no colloscope row")]
    PrefilledGroupListInColloscope(collomatique_state_colloscopes::GroupListId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateColloscopeInterrogationError {
    #[error("invalid week id ({0:?})")]
    InvalidWeekId(collomatique_state_colloscopes::WeekId),
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(collomatique_state_colloscopes::SlotId),
    #[error("slot {0:?} does not run on the period of week {1:?}")]
    SlotNotRunningOnPeriod(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::WeekId,
    ),
    #[error("interrogation on inactive week {1:?} for slot {0:?}")]
    InterrogationOnInactiveWeek(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::WeekId,
    ),
    #[error("Invalid group number in interrogation")]
    InvalidGroupNumInInterrogation(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::WeekId,
    ),
}

impl ColloscopeUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<CleaningOp<ColloscopeUpdateWarning>> {
        None
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<(), ColloscopeUpdateError> {
        match self {
            Self::UpdateColloscopeGroupList(group_list_id, placements) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Colloscope(
                            collomatique_state_colloscopes::ColloscopeOp::SetGroupList(
                                *group_list_id,
                                placements.clone(),
                            )
                        ),
                        self.get_desc()
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, ColloscopePrecheckError, Convergence, FixableInvariant,
                            InvalidOp, PrecheckError, Reference, StudentRefSite,
                        };
                        match e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Colloscope(
                                ColloscopePrecheckError::InvalidGroupListId(id),
                            ))) => UpdateColloscopeGroupListError::InvalidGroupListId(id),
                            // The pre-op state was valid, so every break in the set was
                            // introduced by this SetGroupList. Old validator order
                            // (validate_group_list_placements): excluded student, then
                            // invalid student id, then group number out of bounds.
                            Error::BrokenInvariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::ColloscopeStudentExcluded(group_list, student),
                                    ) = inv
                                    {
                                        return UpdateColloscopeGroupListError::ExcludedStudentInGroupList(
                                            *group_list, *student,
                                        );
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Student {
                                        target,
                                        site: StudentRefSite::ColloscopeGroupListStudent(_),
                                    }) = inv
                                    {
                                        return UpdateColloscopeGroupListError::InvalidStudentId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::Convergence(
                                        // The offending group number is not part of
                                        // the frozen `ops/` error payload.
                                        Convergence::ColloscopeStudentGroupOutOfBounds(group_list, student, _),
                                    ) = inv
                                    {
                                        return UpdateColloscopeGroupListError::InvalidGroupNumForStudentInGroupList(
                                            *group_list, *student,
                                        );
                                    }
                                }
                                panic!("Unexpected invariant breaks during UpdateColloscopeGroupList: {set:?}");
                            }
                            _ => panic!("Unexpected error during UpdateColloscopeGroupList: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(())
            }
            Self::UpdateColloscopeInterrogation(slot_id, week_id, assigned_groups) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Colloscope(
                            collomatique_state_colloscopes::ColloscopeOp::SetInterrogation(
                                *slot_id,
                                *week_id,
                                assigned_groups.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, ColloscopePrecheckError, Convergence, FixableInvariant,
                            InvalidOp, PrecheckError,
                        };
                        match e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Colloscope(pe))) => match pe {
                                ColloscopePrecheckError::InvalidWeekId(id) => {
                                    UpdateColloscopeInterrogationError::InvalidWeekId(id)
                                }
                                ColloscopePrecheckError::InvalidSlotId(id) => {
                                    UpdateColloscopeInterrogationError::InvalidSlotId(id)
                                }
                                // SetInterrogation carries no group list id, so this
                                // precheck variant cannot arise here.
                                ColloscopePrecheckError::InvalidGroupListId(id) => panic!(
                                    "Unexpected InvalidGroupListId during UpdateColloscopeInterrogation: {id:?}"
                                ),
                            },
                            // The pre-op state was valid, so every break in the set was
                            // introduced by this SetInterrogation. Old validator order
                            // (apply_colloscope SetInterrogation): slot-not-running,
                            // then inactive week, then group number out of bounds.
                            Error::BrokenInvariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::InterrogationSlotNotRunningOnPeriod(slot, week),
                                    ) = inv
                                    {
                                        return UpdateColloscopeInterrogationError::SlotNotRunningOnPeriod(*slot, *week);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::InterrogationOnInactiveWeek(slot, week),
                                    ) = inv
                                    {
                                        return UpdateColloscopeInterrogationError::InterrogationOnInactiveWeek(*slot, *week);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::InterrogationGroupsOutOfBounds(slot, week, _),
                                    ) = inv
                                    {
                                        return UpdateColloscopeInterrogationError::InvalidGroupNumInInterrogation(*slot, *week);
                                    }
                                }
                                panic!("Unexpected invariant breaks during UpdateColloscopeInterrogation: {set:?}");
                            }
                            _ => panic!("Unexpected error during UpdateColloscopeInterrogation: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(())
            }
            Self::EraseColloscope => {
                // Only non-empty rows need clearing; the surface yields exactly
                // those. Row coordinates are collected up front so the mutable
                // `data.apply` loop does not overlap the shared read borrow.
                let inner = data.get_data().get_inner_data();
                let coords: Vec<_> = inner
                    .colloscope
                    .iter()
                    .map(|((slot_id, week_id), _groups)| (slot_id, week_id))
                    .collect();
                for (slot_id, week_id) in coords {
                    let result = data
                        .apply(
                            collomatique_state_colloscopes::Op::Colloscope(
                                collomatique_state_colloscopes::ColloscopeOp::SetInterrogation(
                                    slot_id,
                                    week_id,
                                    BTreeSet::new(),
                                ),
                            ),
                            self.get_desc(),
                        )
                        .expect("No error possible for erasing");

                    assert!(result.is_none());
                }

                Ok(())
            }
            Self::EraseGroupLists => {
                // Only non-empty group lists need clearing.
                let group_list_ids: Vec<_> = data
                    .get_data()
                    .get_inner_data()
                    .colloscope
                    .group_lists_iter()
                    .map(|(group_list_id, _placements)| group_list_id)
                    .collect();
                for group_list_id in group_list_ids {
                    let result = data
                        .apply(
                            collomatique_state_colloscopes::Op::Colloscope(
                                collomatique_state_colloscopes::ColloscopeOp::SetGroupList(
                                    group_list_id,
                                    BTreeMap::new(),
                                ),
                            ),
                            self.get_desc(),
                        )
                        .expect("No error possible for erasing");

                    assert!(result.is_none());
                }

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
    ) -> Result<(), ColloscopeUpdateError> {
        match self {
            Self::UpdateColloscopeGroupList(group_list_id, placements) => {
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Colloscope(
                            collomatique_state_colloscopes::ColloscopeOp::SetGroupList(
                                *group_list_id,
                                placements.clone(),
                            )
                        ),
                        self.get_desc()
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, ColloscopePrecheckError, Convergence, FixableInvariant,
                            InvalidOp, PrecheckError, Reference, StudentRefSite,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Colloscope(pe))) => match pe {
                                ColloscopePrecheckError::InvalidGroupListId(id) => {
                                    UpdateColloscopeGroupListError::InvalidGroupListId(*id)
                                }
                                // A group-list row is addressed by the list
                                // alone: this op names neither a week nor a
                                // slot, so the other two prechecks cannot fire.
                                ColloscopePrecheckError::InvalidWeekId(_)
                                | ColloscopePrecheckError::InvalidSlotId(_) => panic!(
                                    "Unexpected colloscope precheck during UpdateColloscopeGroupList: {e:?}"
                                ),
                            },
                            // The pre-op state was valid, so every break in the set was
                            // introduced by this SetGroupList — and none of them is the
                            // cascade's to repair: the row went back to its old self
                            // with the rolled-back op, so every arm of the map that
                            // could name it looks for the offending placement, finds
                            // the old row innocent, and answers nothing. The engine
                            // convicts the target and the scans below turn the break
                            // back into the bad input it came from. Old validator order:
                            // the prefilled target first (the old bodies' guard ran
                            // ahead of validate_group_list_placements), then that
                            // validator's own order — excluded student, invalid student
                            // id, group number out of bounds.
                            Error::BrokenInvariants(set) => {
                                // A list that fills its groups by hand has no
                                // colloscope row, so the placements are beside
                                // the point: the target is the wrong kind of
                                // list. The map does know this break — it
                                // clears such a row — but a rolled-back write
                                // leaves no row to clear, so it answers nothing
                                // and the target is convicted. Until this scan
                                // the break reached the catch-all below and the
                                // op died there; it is the guard step 4 dropped
                                // from `force_apply_colloscope`, restored here
                                // under a name of its own (D5's growth rule).
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::ColloscopeGroupListPrefilled(group_list),
                                    ) = inv
                                    {
                                        return UpdateColloscopeGroupListError::PrefilledGroupListInColloscope(
                                            *group_list,
                                        );
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::ColloscopeStudentExcluded(group_list, student),
                                    ) = inv
                                    {
                                        return UpdateColloscopeGroupListError::ExcludedStudentInGroupList(
                                            *group_list, *student,
                                        );
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Student {
                                        target,
                                        site: StudentRefSite::ColloscopeGroupListStudent(_),
                                    }) = inv
                                    {
                                        return UpdateColloscopeGroupListError::InvalidStudentId(*target);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        // The offending group number is not part of
                                        // the frozen `ops/` error payload.
                                        Convergence::ColloscopeStudentGroupOutOfBounds(group_list, student, _),
                                    ) = inv
                                    {
                                        return UpdateColloscopeGroupListError::InvalidGroupNumForStudentInGroupList(
                                            *group_list, *student,
                                        );
                                    }
                                }
                                // The four scans above cover every break a
                                // SetGroupList can cause, so this is the
                                // instrument H.2 describes rather than a hole:
                                // reaching it means the checker grew a case the
                                // vocabulary has no word for.
                                panic!("Unexpected invariant breaks during UpdateColloscopeGroupList: {set:?}");
                            }
                            _ => panic!("Unexpected error during UpdateColloscopeGroupList: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(())
            }
            Self::UpdateColloscopeInterrogation(slot_id, week_id, assigned_groups) => {
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Colloscope(
                            collomatique_state_colloscopes::ColloscopeOp::SetInterrogation(
                                *slot_id,
                                *week_id,
                                assigned_groups.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, ColloscopePrecheckError, Convergence, FixableInvariant,
                            InvalidOp, PrecheckError,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Colloscope(pe))) => match pe {
                                ColloscopePrecheckError::InvalidWeekId(id) => {
                                    UpdateColloscopeInterrogationError::InvalidWeekId(*id)
                                }
                                ColloscopePrecheckError::InvalidSlotId(id) => {
                                    UpdateColloscopeInterrogationError::InvalidSlotId(*id)
                                }
                                // SetInterrogation carries no group list id, so this
                                // precheck variant cannot arise here.
                                ColloscopePrecheckError::InvalidGroupListId(id) => panic!(
                                    "Unexpected InvalidGroupListId during UpdateColloscopeInterrogation: {id:?}"
                                ),
                            },
                            // Same reasoning as the group-list row above: the pre-op
                            // state was valid, so this SetInterrogation introduced
                            // every break, and the rolled-back cell — empty, or
                            // holding the groups it held before — matches none of the
                            // coordinates the breaks name. The map answers nothing,
                            // the engine convicts the target, and these scans report
                            // the bad input in the old validator's order
                            // (apply_colloscope SetInterrogation): slot-not-running,
                            // then inactive week, then group number out of bounds.
                            Error::BrokenInvariants(set) => {
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::InterrogationSlotNotRunningOnPeriod(slot, week),
                                    ) = inv
                                    {
                                        return UpdateColloscopeInterrogationError::SlotNotRunningOnPeriod(*slot, *week);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::InterrogationOnInactiveWeek(slot, week),
                                    ) = inv
                                    {
                                        return UpdateColloscopeInterrogationError::InterrogationOnInactiveWeek(*slot, *week);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::InterrogationGroupsOutOfBounds(slot, week, _),
                                    ) = inv
                                    {
                                        return UpdateColloscopeInterrogationError::InvalidGroupNumInInterrogation(*slot, *week);
                                    }
                                }
                                panic!("Unexpected invariant breaks during UpdateColloscopeInterrogation: {set:?}");
                            }
                            _ => panic!("Unexpected error during UpdateColloscopeInterrogation: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(())
            }
            Self::EraseColloscope => {
                // Only non-empty rows need clearing; the surface yields exactly
                // those. Row coordinates are collected up front so the mutable
                // `session.apply` loop does not overlap the shared read borrow.
                let coords: Vec<_> = session
                    .get_data()
                    .get_inner_data()
                    .colloscope
                    .iter()
                    .map(|((slot_id, week_id), _groups)| (slot_id, week_id))
                    .collect();
                for (slot_id, week_id) in coords {
                    // Clearing only ever removes, so no repair can be needed:
                    // an empty row is absent, and an absent row contradicts
                    // nothing. The cascade stays quiet all the way through.
                    let result = session
                        .apply(
                            collomatique_state_colloscopes::Op::Colloscope(
                                collomatique_state_colloscopes::ColloscopeOp::SetInterrogation(
                                    slot_id,
                                    week_id,
                                    BTreeSet::new(),
                                ),
                            ),
                            self.get_desc(),
                        )
                        .expect("No error possible for erasing");

                    assert!(result.is_none());
                }

                Ok(())
            }
            Self::EraseGroupLists => {
                // Only non-empty group lists need clearing.
                let group_list_ids: Vec<_> = session
                    .get_data()
                    .get_inner_data()
                    .colloscope
                    .group_lists_iter()
                    .map(|(group_list_id, _placements)| group_list_id)
                    .collect();
                for group_list_id in group_list_ids {
                    let result = session
                        .apply(
                            collomatique_state_colloscopes::Op::Colloscope(
                                collomatique_state_colloscopes::ColloscopeOp::SetGroupList(
                                    group_list_id,
                                    BTreeMap::new(),
                                ),
                            ),
                            self.get_desc(),
                        )
                        .expect("No error possible for erasing");

                    assert!(result.is_none());
                }

                Ok(())
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Colloscope,
            match self {
                ColloscopeUpdateOp::UpdateColloscopeGroupList(_id, _placements) => {
                    "Mettre à jour une liste de groupe du colloscope".into()
                }
                ColloscopeUpdateOp::UpdateColloscopeInterrogation(_slot, _week, _groups) => {
                    "Mettre à jour une interrogation du colloscope".into()
                }
                ColloscopeUpdateOp::EraseColloscope => "Effacer le colloscope".into(),
                ColloscopeUpdateOp::EraseGroupLists => {
                    "Effacer les listes de groupes automatiques".into()
                }
            },
        )
    }
}

#[cfg(test)]
mod tests {
    //! The colloscope is where everything else ends up being *referenced*, and
    //! it references nothing back: no entity in the document points at a
    //! colloscope cell or at a group placement. That is why this family is the
    //! one with the richest error translation and the emptiest warning list —
    //! its two writing ops can be wrong in eight different ways between them,
    //! and none of the four can ever make the cascade repair anything.
    //!
    //! The reason the writes never cascade is the one the whole step turns on.
    //! Every break they can cause is caused by the payload itself, and the
    //! failing op is rolled back before the map is asked: the row goes back to
    //! what it held before — usually nothing at all — so the arm looking for
    //! the offending placement, the offending cell or the offending group
    //! finds it absent and answers nothing. The engine convicts the target and
    //! the scans below it report the bad input, in the old validator's order.
    //!
    //! The two erase composites cannot cascade for a different reason: they
    //! only ever *remove*, and an absent row contradicts nothing.
    //!
    //! The frozen hogwarts base carries no colloscope at all, and both of its
    //! group lists are prefilled — a shape that may hold no colloscope row
    //! either. So every fixture below writes its own corner on top of the
    //! base, in plain sight at its top: an automatic group list to place
    //! students in, and the cells the erase fixtures are about.

    use super::*;
    // No `fixes` helper here: this is the one family whose every fixture
    // asserts an *empty* warning log, so there is never a `Fix` list to read.
    use crate::test_utils::hogwarts;
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::{
        AssignmentOp, ColloscopeOp, GroupListOp, NewId, NonEmptyRangeInclusive, Op, SubjectOp,
        group_lists::{GroupList, GroupListFilling, GroupListParameters},
        ids::{GroupListId, Id, PeriodId, SlotId, StudentId, SubjectId, WeekId},
        subjects::Subject,
    };
    use std::num::NonZeroU32;

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

    fn student_by_name(data: &Data, surname: &str, firstname: &str) -> StudentId {
        data.get_inner_data()
            .params
            .students
            .student_map
            .iter()
            .find(|(_id, student)| {
                student.desc.surname == surname && student.desc.firstname == firstname
            })
            .map(|(id, _student)| id)
            .unwrap_or_else(|| {
                panic!("the fixture should have a student named {firstname} {surname}")
            })
    }

    fn group_list_by_name(data: &Data, name: &str) -> GroupListId {
        data.get_inner_data()
            .params
            .group_lists
            .group_list_map
            .iter()
            .find(|(_id, group_list)| group_list.params().name == name)
            .map(|(id, _group_list)| id)
            .unwrap_or_else(|| panic!("the fixture should have a group list named {name}"))
    }

    fn subject_of(data: &Data, subject: SubjectId) -> Subject {
        data.get_inner_data()
            .params
            .subjects
            .find_subject(subject)
            .expect("the fixture's subject should be live")
            .clone()
    }

    /// The subject's slots, in display order.
    fn slots_of_subject(data: &Data, subject: SubjectId) -> Vec<SlotId> {
        data.get_inner_data()
            .params
            .slots
            .slots_for_subject(subject)
            .into_iter()
            .flatten()
            .map(|(id, _slot)| *id)
            .collect()
    }

    /// The `n`-th week in global week order.
    fn week_at(data: &Data, index: usize) -> WeekId {
        data.get_inner_data()
            .params
            .week_ids()
            .nth(index)
            .unwrap_or_else(|| panic!("the fixture should have at least {} weeks", index + 1))
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

    /// Ids no document ever issued.
    fn dangling_group_list() -> GroupListId {
        unsafe { GroupListId::new(1u64 << 40) }
    }

    fn dangling_student() -> StudentId {
        unsafe { StudentId::new(1u64 << 40) }
    }

    fn dangling_slot() -> SlotId {
        unsafe { SlotId::new(1u64 << 40) }
    }

    fn dangling_week() -> WeekId {
        unsafe { WeekId::new(1u64 << 40) }
    }

    /// The number of groups the automatic list below offers — the bound every
    /// placement in it has to fit under.
    const AUTOMATIC_GROUPS: usize = 3;

    /// Hogwarts plus an automatic group list of [AUTOMATIC_GROUPS] unnamed
    /// groups, excluding Drago Malefoy. The base's own two lists are
    /// prefilled, and a prefilled list may hold no colloscope row at all, so
    /// this is the one shape the group-list writing op can legitimately aim
    /// at.
    fn hogwarts_with_an_automatic_list() -> (AppState<Data, Desc>, GroupListId) {
        let mut base = hogwarts();
        let malefoy = student_by_name(base.get_data(), "Malefoy", "Drago");
        let list = GroupList::new(
            GroupListParameters {
                name: "Liste automatique".into(),
                students_per_group: NonEmptyRangeInclusive::new(
                    NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
                )
                .expect("statically non-empty"),
                group_names: vec![None; AUTOMATIC_GROUPS],
            },
            GroupListFilling::Automatic {
                excluded_students: BTreeSet::from([malefoy]),
            },
        )
        .expect("an automatic filling never constrains the group count");

        let id = match base.apply(
            Op::GroupList(GroupListOp::Add(list)),
            (OpCategory::GroupLists, "Préparation".into()),
        ) {
            Ok(Some(NewId::GroupListId(id))) => id,
            other => panic!("adding a group list should hand back its id, got {other:?}"),
        };

        (base, id)
    }

    /// Replays `ops` on a clone of `base`: the document a fixture expects,
    /// written as the elementary ops it expects the composite to have landed.
    fn expected_document(base: &AppState<Data, Desc>, ops: Vec<Op>) -> AppState<Data, Desc> {
        let mut expected = base.clone();
        for op in ops {
            expected
                .apply(op, (OpCategory::Colloscope, "Expected".into()))
                .expect("each expected op lands in the order the composite landed it");
        }

        expected
    }

    /// Runs one op alone on `base` and hands back what the document became and
    /// what the cascade had to repair on the way.
    fn apply_alone(
        base: &AppState<Data, Desc>,
        op: &ColloscopeUpdateOp,
    ) -> (AppState<Data, Desc>, Vec<CascadeWarning>) {
        let mut session = CascadeSession::new(base.clone());
        op.apply_to_session(&mut session)
            .unwrap_or_else(|e| panic!("{op:?} should land, got {e:?}"));

        session.commit(op.get_desc())
    }

    /// Placing live, non-excluded students in in-bounds groups of an automatic
    /// list: the whole row goes in and nothing in the document has to move.
    #[test]
    fn writing_a_group_list_row_places_the_students_and_warns_about_nothing() {
        let (base, list) = hogwarts_with_an_automatic_list();
        let harry = student_by_name(base.get_data(), "Potter", "Harry");
        let ron = student_by_name(base.get_data(), "Weasley", "Ron");
        let placements = BTreeMap::from([(harry, 0), (ron, 1)]);

        let op = ColloscopeUpdateOp::UpdateColloscopeGroupList(list, placements.clone());
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::Colloscope(ColloscopeOp::SetGroupList(list, placements))],
            )
            .get_data(),
        );
    }

    /// The same for a cell: a group of the list associated with the slot's
    /// subject, on a week the slot runs on.
    #[test]
    fn writing_an_interrogation_places_the_groups_and_warns_about_nothing() {
        let base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let slot = slots_of_subject(base.get_data(), metamorphose)[0];
        // The first period opens with two weeks without interrogations; the
        // third is the first one a slot can be used on.
        let week = week_at(base.get_data(), 2);
        let groups = BTreeSet::from([0]);

        let op = ColloscopeUpdateOp::UpdateColloscopeInterrogation(slot, week, groups.clone());
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::Colloscope(ColloscopeOp::SetInterrogation(
                    slot, week, groups,
                ))],
            )
            .get_data(),
        );
    }

    /// The group-list row's whole surface: the state layer's own precheck on a
    /// dead list id, then the three payload scans in the old validator's order
    /// — the excluded student, the student who does not exist, the group
    /// number past the end of the list.
    ///
    /// Two of those three are breaks the map knows a repair for when they
    /// arise honestly (excluding a student, or shrinking a list, takes their
    /// placement out of the colloscope). Here they are convicted instead,
    /// because the rolled-back row never held the placement the break names.
    #[test]
    fn the_group_list_row_reports_every_way_its_placements_can_be_wrong() {
        let (base, list) = hogwarts_with_an_automatic_list();
        let harry = student_by_name(base.get_data(), "Potter", "Harry");
        let malefoy = student_by_name(base.get_data(), "Malefoy", "Drago");
        let past_the_end = AUTOMATIC_GROUPS as u32;

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            ColloscopeUpdateOp::UpdateColloscopeGroupList(
                dangling_group_list(),
                BTreeMap::from([(harry, 0)]),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            ColloscopeUpdateError::UpdateColloscopeGroupList(
                UpdateColloscopeGroupListError::InvalidGroupListId(dangling_group_list()),
            ),
        );
        assert_eq!(
            ColloscopeUpdateOp::UpdateColloscopeGroupList(list, BTreeMap::from([(malefoy, 0)]))
                .apply_to_session(&mut session)
                .unwrap_err(),
            ColloscopeUpdateError::UpdateColloscopeGroupList(
                UpdateColloscopeGroupListError::ExcludedStudentInGroupList(list, malefoy),
            ),
        );
        assert_eq!(
            ColloscopeUpdateOp::UpdateColloscopeGroupList(
                list,
                BTreeMap::from([(dangling_student(), 0)]),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            ColloscopeUpdateError::UpdateColloscopeGroupList(
                UpdateColloscopeGroupListError::InvalidStudentId(dangling_student()),
            ),
        );
        assert_eq!(
            ColloscopeUpdateOp::UpdateColloscopeGroupList(
                list,
                BTreeMap::from([(harry, past_the_end)]),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            ColloscopeUpdateError::UpdateColloscopeGroupList(
                UpdateColloscopeGroupListError::InvalidGroupNumForStudentInGroupList(list, harry),
            ),
        );

        // Which break wins when a payload carries several is public API, so
        // the order is pinned rather than left to the set's own: a row placing
        // an excluded student, a student who does not exist and a student past
        // the last group reports the excluded one.
        assert_eq!(
            ColloscopeUpdateOp::UpdateColloscopeGroupList(
                list,
                BTreeMap::from([(malefoy, 0), (dangling_student(), 0), (harry, past_the_end)]),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            ColloscopeUpdateError::UpdateColloscopeGroupList(
                UpdateColloscopeGroupListError::ExcludedStudentInGroupList(list, malefoy),
            ),
        );
        // And the student who does not exist is reported before the group
        // number, one rank down the same order.
        assert_eq!(
            ColloscopeUpdateOp::UpdateColloscopeGroupList(
                list,
                BTreeMap::from([(dangling_student(), 0), (harry, past_the_end)]),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            ColloscopeUpdateError::UpdateColloscopeGroupList(
                UpdateColloscopeGroupListError::InvalidStudentId(dangling_student()),
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// A group list that fills its groups by hand has nothing for the
    /// colloscope to store, so a row aimed at one is refused — and refused
    /// *first*, before any of the three placement scans, since the placements
    /// are beside the point once the target is the wrong kind of list.
    ///
    /// This is where the old world's guard sat, in both bodies that had one
    /// (`git show 56510199^:state-colloscopes/src/colloscopes.rs`, ~`:361` and
    /// `:206`), and `ops/` used to translate it — as the overloaded
    /// `InvalidGroupListId`, but translate it. Step 4's `force_apply` copies
    /// dropped the guard by design and the condition became a plain invariant
    /// break that no scan named, so the op has panicked ever since. The pin
    /// restores the answer with a name of its own.
    ///
    /// Hogwarts's own two lists are both prefilled, so no setup is needed:
    /// « Liste principale » is the shape this is about.
    #[test]
    fn a_row_aimed_at_a_prefilled_list_is_refused_before_its_placements_are_read() {
        let base = hogwarts();
        let prefilled = group_list_by_name(base.get_data(), "Liste principale");
        let harry = student_by_name(base.get_data(), "Potter", "Harry");

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            ColloscopeUpdateOp::UpdateColloscopeGroupList(prefilled, BTreeMap::from([(harry, 0)]),)
                .apply_to_session(&mut session)
                .unwrap_err(),
            ColloscopeUpdateError::UpdateColloscopeGroupList(
                UpdateColloscopeGroupListError::PrefilledGroupListInColloscope(prefilled),
            ),
        );
        // And it wins over the placement scans, which is the half that moves an
        // existing answer: before this pin the dangling student was reported.
        assert_eq!(
            ColloscopeUpdateOp::UpdateColloscopeGroupList(
                prefilled,
                BTreeMap::from([(dangling_student(), 0)]),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            ColloscopeUpdateError::UpdateColloscopeGroupList(
                UpdateColloscopeGroupListError::PrefilledGroupListInColloscope(prefilled),
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// The cell's surface, the same way: the two state-layer prechecks (the
    /// week first, then the slot), then the payload scans — the inactive week
    /// and the group number past the end of the associated list. The third
    /// scan, the slot whose subject skips the week's period, needs a document
    /// of its own and has its own fixture below.
    #[test]
    fn the_interrogation_reports_every_way_its_coordinates_can_be_wrong() {
        let base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let slot = slots_of_subject(base.get_data(), metamorphose)[0];
        let active = week_at(base.get_data(), 2);
        // The first two weeks of the school year run no interrogations at all.
        let inactive = week_at(base.get_data(), 0);
        // « Liste principale », associated with Métamorphose, offers eight.
        let past_the_end = 8u32;
        let group = BTreeSet::from([0]);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                slot,
                dangling_week(),
                group.clone(),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            ColloscopeUpdateError::UpdateColloscopeInterrogation(
                UpdateColloscopeInterrogationError::InvalidWeekId(dangling_week()),
            ),
        );
        assert_eq!(
            ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                dangling_slot(),
                active,
                group.clone(),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            ColloscopeUpdateError::UpdateColloscopeInterrogation(
                UpdateColloscopeInterrogationError::InvalidSlotId(dangling_slot()),
            ),
        );
        // Both coordinates dead: the week is checked first.
        assert_eq!(
            ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                dangling_slot(),
                dangling_week(),
                group.clone(),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            ColloscopeUpdateError::UpdateColloscopeInterrogation(
                UpdateColloscopeInterrogationError::InvalidWeekId(dangling_week()),
            ),
        );
        assert_eq!(
            ColloscopeUpdateOp::UpdateColloscopeInterrogation(slot, inactive, group.clone())
                .apply_to_session(&mut session)
                .unwrap_err(),
            ColloscopeUpdateError::UpdateColloscopeInterrogation(
                UpdateColloscopeInterrogationError::InterrogationOnInactiveWeek(slot, inactive),
            ),
        );
        assert_eq!(
            ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                slot,
                active,
                BTreeSet::from([past_the_end]),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            ColloscopeUpdateError::UpdateColloscopeInterrogation(
                UpdateColloscopeInterrogationError::InvalidGroupNumInInterrogation(slot, active),
            ),
        );
        // The inactive week is reported before the group number, one rank up
        // the same order.
        assert_eq!(
            ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                slot,
                inactive,
                BTreeSet::from([past_the_end]),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            ColloscopeUpdateError::UpdateColloscopeInterrogation(
                UpdateColloscopeInterrogationError::InterrogationOnInactiveWeek(slot, inactive),
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// The first of the cell's three payload scans, which the base cannot show
    /// as it stands: every one of its subjects runs on every period. So the
    /// fixture takes Divination out of the first period — which means emptying
    /// its assignments row there and dropping its group-list association
    /// first, since a subject that skips a period may have neither.
    ///
    /// Dropping the association is also what makes this an order pin: with no
    /// list associated, the group bound at that coordinate is zero, so group 0
    /// is *also* past the end. The slot-not-running scan is the one that
    /// answers, three ranks above.
    #[test]
    fn an_interrogation_on_a_period_the_subject_skips_is_rejected() {
        let mut base = hogwarts();
        let divination = subject_by_name(base.get_data(), "Divination");
        let first = period_at(base.get_data(), 0);
        let slot = slots_of_subject(base.get_data(), divination)[0];
        let week = week_at(base.get_data(), 2);

        base.apply(
            Op::Assignment(AssignmentOp::SetRow(first, divination, BTreeSet::new())),
            (OpCategory::Assignments, "Préparation".into()),
        )
        .expect("emptying an assignments row breaks nothing");
        base.apply(
            Op::GroupList(GroupListOp::AssignToSubject(first, divination, None)),
            (OpCategory::GroupLists, "Préparation".into()),
        )
        .expect("dropping an association breaks nothing");
        let mut skipping = subject_of(base.get_data(), divination);
        skipping.excluded_periods.insert(first);
        base.apply(
            Op::Subject(SubjectOp::Update(divination, skipping)),
            (OpCategory::Subjects, "Préparation".into()),
        )
        .expect("the period was emptied of everything the subject had on it");

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            ColloscopeUpdateOp::UpdateColloscopeInterrogation(slot, week, BTreeSet::from([0]),)
                .apply_to_session(&mut session)
                .unwrap_err(),
            ColloscopeUpdateError::UpdateColloscopeInterrogation(
                UpdateColloscopeInterrogationError::SlotNotRunningOnPeriod(slot, week),
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// The two erase composites, on a document carrying both kinds of content:
    /// two cells and one group-list row. Each erases its own half and leaves
    /// the other one standing, and neither can make the cascade repair
    /// anything — they only ever remove, and an absent row contradicts
    /// nothing.
    #[test]
    fn the_erasers_clear_their_own_half_and_warn_about_nothing() {
        let (mut base, list) = hogwarts_with_an_automatic_list();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let slots = slots_of_subject(base.get_data(), metamorphose);
        let harry = student_by_name(base.get_data(), "Potter", "Harry");
        let ron = student_by_name(base.get_data(), "Weasley", "Ron");
        let cells = [
            (slots[0], week_at(base.get_data(), 2), BTreeSet::from([0])),
            (slots[1], week_at(base.get_data(), 3), BTreeSet::from([1])),
        ];

        for (slot, week, groups) in cells.clone() {
            base.apply(
                Op::Colloscope(ColloscopeOp::SetInterrogation(slot, week, groups)),
                (OpCategory::Colloscope, "Préparation".into()),
            )
            .expect("a group of the associated list may be placed on an active week");
        }
        base.apply(
            Op::Colloscope(ColloscopeOp::SetGroupList(
                list,
                BTreeMap::from([(harry, 0), (ron, 1)]),
            )),
            (OpCategory::Colloscope, "Préparation".into()),
        )
        .expect("placing live students in a live automatic list breaks nothing");

        let op = ColloscopeUpdateOp::EraseColloscope;
        let (state, warnings) = apply_alone(&base, &op);
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                cells
                    .iter()
                    .map(
                        |(slot, week, _groups)| Op::Colloscope(ColloscopeOp::SetInterrogation(
                            *slot,
                            *week,
                            BTreeSet::new()
                        ))
                    )
                    .collect(),
            )
            .get_data(),
            "the cells go and the group-list row stays",
        );

        let op = ColloscopeUpdateOp::EraseGroupLists;
        let (state, warnings) = apply_alone(&base, &op);
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::Colloscope(ColloscopeOp::SetGroupList(
                    list,
                    BTreeMap::new(),
                ))],
            )
            .get_data(),
            "the group-list row goes and the cells stay",
        );
    }
}
