use std::collections::{BTreeMap, BTreeSet};

use super::*;

/// The whole contents of a colloscope, in the same sparse shape as
/// [collomatique_state_colloscopes::colloscopes::Colloscope] but built from
/// plain maps: the state type is two private `Table`s and carries no serde,
/// and an [crate::UpdateOp] payload must serialize.
///
/// Canonical form is *not* required of a value built by hand: an empty group
/// set or an empty placement map means "no row", exactly as it does for
/// [ColloscopeUpdateOp::UpdateColloscopeInterrogation] and its twin, and
/// [ColloscopeUpdateOp::InstallColloscope] reads it that way.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ColloscopeContents {
    pub interrogations: BTreeMap<
        (
            collomatique_state_colloscopes::SlotId,
            collomatique_state_colloscopes::WeekId,
        ),
        BTreeSet<u32>,
    >,
    pub group_lists: BTreeMap<
        collomatique_state_colloscopes::GroupListId,
        BTreeMap<collomatique_state_colloscopes::StudentId, u32>,
    >,
}

impl From<&collomatique_state_colloscopes::colloscopes::Colloscope> for ColloscopeContents {
    fn from(colloscope: &collomatique_state_colloscopes::colloscopes::Colloscope) -> Self {
        ColloscopeContents {
            interrogations: colloscope
                .iter()
                .map(|(coord, groups)| (coord, groups.clone()))
                .collect(),
            group_lists: colloscope
                .group_lists_iter()
                .map(|(id, placements)| (id, placements.clone()))
                .collect(),
        }
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
    /// Replaces the colloscope wholesale: afterwards the document holds the
    /// payload's rows and no others. This is the solver's landing door and the
    /// scripting api's `install`, and it is the reason neither has to reach for
    /// the forced [collomatique_state_colloscopes::Op::GlobalUpdate].
    ///
    /// The payload is a whole colloscope, not a diff. It is *applied* as one
    /// — a row the document already holds correctly costs no elementary op —
    /// but that is this arm's business, not the caller's.
    InstallColloscope(ColloscopeContents),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColloscopeUpdateError {
    #[error(transparent)]
    UpdateColloscopeGroupList(#[from] UpdateColloscopeGroupListError),
    #[error(transparent)]
    UpdateColloscopeInterrogation(#[from] UpdateColloscopeInterrogationError),
    #[error(transparent)]
    InstallColloscope(#[from] InstallColloscopeError),
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
    /// write outright (as the overloaded [Self::InvalidGroupListId]) until the
    /// `force_apply` copies dropped the guard by design, after which the
    /// condition became a plain invariant break that nothing in
    /// `colloscopes/ops/` named — so the op panicked instead.
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

/// Every way a whole-colloscope payload can be wrong: the two single-row
/// vocabularies put together, since the composite writes both kinds of row.
///
/// Errors are per-op, so this is an enum of its own rather than a reuse of
/// [UpdateColloscopeGroupListError] and [UpdateColloscopeInterrogationError] —
/// the same choice [crate::AddGeneratedGroupListsError] makes, where the
/// variants it needs are re-declared instead of shared. Every variant carries
/// the ids that locate the offending row, so a bulk failure says which row
/// failed.
#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstallColloscopeError {
    #[error("invalid group list id ({0:?})")]
    InvalidGroupListId(collomatique_state_colloscopes::GroupListId),
    #[error("group list {0:?} is prefilled and has no colloscope row")]
    PrefilledGroupListInColloscope(collomatique_state_colloscopes::GroupListId),
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
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
                                // op died there; it is the guard
                                // `force_apply_colloscope` dropped, restored
                                // here under a name of its own.
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
                                        // the frozen `colloscopes/ops/` error payload.
                                        Convergence::ColloscopeStudentGroupOutOfBounds(group_list, student, _),
                                    ) = inv
                                    {
                                        return UpdateColloscopeGroupListError::InvalidGroupNumForStudentInGroupList(
                                            *group_list, *student,
                                        );
                                    }
                                }
                                // The four scans above cover every break a
                                // SetGroupList can cause, so this is an
                                // instrument rather than a hole: reaching it
                                // means the checker grew a case the vocabulary
                                // has no word for.
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
            Self::InstallColloscope(contents) => {
                // The payload's rows, canonicalized first: an empty group set
                // or an empty placement map means "no row", exactly as it does
                // for the two single-row variants. Everything below reads
                // these two maps rather than the payload's own.
                let new_group_lists: BTreeMap<_, _> = contents
                    .group_lists
                    .iter()
                    .filter(|(_id, placements)| !placements.is_empty())
                    .map(|(id, placements)| (*id, placements.clone()))
                    .collect();
                let new_interrogations: BTreeMap<_, _> = contents
                    .interrogations
                    .iter()
                    .filter(|(_coord, groups)| !groups.is_empty())
                    .map(|(coord, groups)| (*coord, groups.clone()))
                    .collect();

                // The document's rows, read once up front so the mutable
                // `session.apply` loops below do not overlap the shared read
                // borrow. The surface only yields non-empty rows, so these are
                // already in the same canonical shape as the two maps above.
                let (old_group_lists, old_interrogations) = {
                    let colloscope = &session.get_data().get_inner_data().colloscope;
                    let old_group_lists: BTreeMap<_, _> = colloscope
                        .group_lists_iter()
                        .map(|(id, placements)| (id, placements.clone()))
                        .collect();
                    let old_interrogations: BTreeMap<_, _> = colloscope
                        .iter()
                        .map(|(coord, groups)| (coord, groups.clone()))
                        .collect();
                    (old_group_lists, old_interrogations)
                };

                // The op carries a whole colloscope; it *lands* as a diff.
                // Every elementary op costs a document clone and a whole-model
                // invariant scan, so erase-then-rewrite — two ops per row,
                // always — is the wrong shape for the case this op is really
                // for: read a colloscope, change a handful of cells, install it
                // back.
                //
                // The four passes below could run in any order (the frame rule
                // holds trivially: every bound a colloscope row is checked
                // against comes from `params`, never from another colloscope
                // row), but the order is fixed so the fixtures can pin the
                // sequence exactly. Within a pass it is key order, the maps
                // being `BTreeMap`s.

                // 1. Group-list clears: the rows the payload drops.
                for group_list_id in old_group_lists.keys() {
                    if new_group_lists.contains_key(group_list_id) {
                        continue;
                    }
                    // Clearing only ever removes, so no repair can be needed
                    // and no check can fail: an absent row contradicts nothing,
                    // and the id resolves — a valid document holds no
                    // colloscope row for a list it does not have.
                    let result = session
                        .apply(
                            collomatique_state_colloscopes::Op::Colloscope(
                                collomatique_state_colloscopes::ColloscopeOp::SetGroupList(
                                    *group_list_id,
                                    BTreeMap::new(),
                                ),
                            ),
                            self.get_desc(),
                        )
                        .expect("No error possible for erasing");

                    assert!(result.is_none());
                }

                // 2. Interrogation clears: likewise for the cells it drops.
                for (slot_id, week_id) in old_interrogations.keys() {
                    if new_interrogations.contains_key(&(*slot_id, *week_id)) {
                        continue;
                    }
                    let result = session
                        .apply(
                            collomatique_state_colloscopes::Op::Colloscope(
                                collomatique_state_colloscopes::ColloscopeOp::SetInterrogation(
                                    *slot_id,
                                    *week_id,
                                    BTreeSet::new(),
                                ),
                            ),
                            self.get_desc(),
                        )
                        .expect("No error possible for erasing");

                    assert!(result.is_none());
                }

                // 3. Group-list writes: the rows the payload adds or changes.
                // A row the document already holds exactly costs nothing.
                for (group_list_id, placements) in &new_group_lists {
                    if old_group_lists.get(group_list_id) == Some(placements) {
                        continue;
                    }
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
                                        InstallColloscopeError::InvalidGroupListId(*id)
                                    }
                                    // A group-list row is addressed by the list
                                    // alone: the elementary op names neither a
                                    // week nor a slot, so the other two
                                    // prechecks cannot fire.
                                    ColloscopePrecheckError::InvalidWeekId(_)
                                    | ColloscopePrecheckError::InvalidSlotId(_) => panic!(
                                        "Unexpected colloscope precheck during InstallColloscope group-list write: {e:?}"
                                    ),
                                },
                                // The state this write lands on is valid — the
                                // document was, and every earlier op of this
                                // composite left it so — so every break in the
                                // set was introduced by this SetGroupList, and
                                // none of them is the cascade's to repair: the
                                // row went back to the one the document already
                                // held with the rolled-back op, and that row is
                                // innocent of the break. So every arm of the map
                                // that could name it answers nothing, the engine
                                // convicts the target, and the scans below turn
                                // the break back into the bad input it came
                                // from. Old validator order: the prefilled
                                // target first, then excluded student, invalid
                                // student id, group number out of bounds.
                                Error::BrokenInvariants(set) => {
                                    for inv in set {
                                        if let FixableInvariant::Convergence(
                                            Convergence::ColloscopeGroupListPrefilled(group_list),
                                        ) = inv
                                        {
                                            return InstallColloscopeError::PrefilledGroupListInColloscope(
                                                *group_list,
                                            );
                                        }
                                    }
                                    for inv in set {
                                        if let FixableInvariant::Convergence(
                                            Convergence::ColloscopeStudentExcluded(group_list, student),
                                        ) = inv
                                        {
                                            return InstallColloscopeError::ExcludedStudentInGroupList(
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
                                            return InstallColloscopeError::InvalidStudentId(*target);
                                        }
                                    }
                                    for inv in set {
                                        if let FixableInvariant::Convergence(
                                            // The offending group number is not part of
                                            // the frozen `colloscopes/ops/` error payload.
                                            Convergence::ColloscopeStudentGroupOutOfBounds(group_list, student, _),
                                        ) = inv
                                        {
                                            return InstallColloscopeError::InvalidGroupNumForStudentInGroupList(
                                                *group_list, *student,
                                            );
                                        }
                                    }
                                    // The four scans above cover every break a
                                    // SetGroupList can cause, so this is an
                                    // instrument rather than a hole: reaching it
                                    // means the checker grew a case the
                                    // vocabulary has no word for.
                                    panic!("Unexpected invariant breaks during InstallColloscope group-list write: {set:?}");
                                }
                                _ => panic!("Unexpected error during InstallColloscope group-list write: {e:?}"),
                            }
                        })?;

                    assert!(result.is_none());
                }

                // 4. Interrogation writes: the cells the payload adds or
                // changes, on the same terms.
                for ((slot_id, week_id), assigned_groups) in &new_interrogations {
                    if old_interrogations.get(&(*slot_id, *week_id)) == Some(assigned_groups) {
                        continue;
                    }
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
                                        InstallColloscopeError::InvalidWeekId(*id)
                                    }
                                    ColloscopePrecheckError::InvalidSlotId(id) => {
                                        InstallColloscopeError::InvalidSlotId(*id)
                                    }
                                    // SetInterrogation carries no group list id,
                                    // so this precheck variant cannot arise here.
                                    ColloscopePrecheckError::InvalidGroupListId(id) => panic!(
                                        "Unexpected InvalidGroupListId during InstallColloscope interrogation write: {id:?}"
                                    ),
                                },
                                // Same reasoning as the group-list row above: the
                                // state this write lands on is valid, so this
                                // SetInterrogation introduced every break, and
                                // the rolled-back cell — empty, or holding the
                                // groups it held before — matches none of the
                                // coordinates the breaks name. The map answers
                                // nothing, the engine convicts the target, and
                                // these scans report the bad input in the old
                                // validator's order: slot-not-running, then
                                // inactive week, then group number out of bounds.
                                Error::BrokenInvariants(set) => {
                                    for inv in set {
                                        if let FixableInvariant::Convergence(
                                            Convergence::InterrogationSlotNotRunningOnPeriod(slot, week),
                                        ) = inv
                                        {
                                            return InstallColloscopeError::SlotNotRunningOnPeriod(*slot, *week);
                                        }
                                    }
                                    for inv in set {
                                        if let FixableInvariant::Convergence(
                                            Convergence::InterrogationOnInactiveWeek(slot, week),
                                        ) = inv
                                        {
                                            return InstallColloscopeError::InterrogationOnInactiveWeek(*slot, *week);
                                        }
                                    }
                                    for inv in set {
                                        if let FixableInvariant::Convergence(
                                            Convergence::InterrogationGroupsOutOfBounds(slot, week, _),
                                        ) = inv
                                        {
                                            return InstallColloscopeError::InvalidGroupNumInInterrogation(*slot, *week);
                                        }
                                    }
                                    panic!("Unexpected invariant breaks during InstallColloscope interrogation write: {set:?}");
                                }
                                _ => panic!("Unexpected error during InstallColloscope interrogation write: {e:?}"),
                            }
                        })?;

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
                ColloscopeUpdateOp::InstallColloscope(_contents) => {
                    "Mettre à jour le colloscope".into()
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
        AnnotatedOp, AssignmentOp, ColloscopeOp, GroupListOp, NewId, NonEmptyRangeInclusive, Op,
        SubjectOp,
        group_lists::{GroupList, GroupListFilling, GroupListParameters},
        ids::{GroupListId, Id, PeriodId, SlotId, StudentId, SubjectId, WeekId},
        ops::AnnotatedColloscopeOp,
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

    /// Hogwarts plus `count` automatic group lists of [AUTOMATIC_GROUPS]
    /// unnamed groups each, all excluding Drago Malefoy. The base's own two
    /// lists are prefilled, and a prefilled list may hold no colloscope row at
    /// all, so this is the one shape the group-list writing op can legitimately
    /// aim at.
    ///
    /// The ids come back in ascending order, which is the order the
    /// [ColloscopeUpdateOp::InstallColloscope] diff visits them in (its payload
    /// is a `BTreeMap`).
    fn hogwarts_with_automatic_lists(count: usize) -> (AppState<Data, Desc>, Vec<GroupListId>) {
        let mut base = hogwarts();
        let malefoy = student_by_name(base.get_data(), "Malefoy", "Drago");
        let mut ids = Vec::with_capacity(count);
        for i in 0..count {
            let list = GroupList::new(
                GroupListParameters {
                    name: format!("Liste automatique {i}"),
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
            ids.push(id);
        }
        ids.sort();

        (base, ids)
    }

    /// The one-list case, which most of the fixtures below want.
    fn hogwarts_with_an_automatic_list() -> (AppState<Data, Desc>, GroupListId) {
        let (base, lists) = hogwarts_with_automatic_lists(1);
        let [list] = lists[..] else {
            unreachable!("one list was asked for")
        };

        (base, list)
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

    /// The elementary ops a committed composite actually landed, in
    /// application order.
    ///
    /// `commit` collapses them into one history slot and the slot keeps them,
    /// which is the only way a fixture can tell "the row was rewritten with
    /// what it already held" from "the row was left alone": both leave the same
    /// document behind. That distinction is the whole point of the
    /// [ColloscopeUpdateOp::InstallColloscope] diff, so its fixtures read the
    /// op list rather than the document alone.
    fn landed_ops(state: &AppState<Data, Desc>) -> Vec<AnnotatedOp> {
        state
            .get_last_op()
            .expect("a committed composite always leaves its history slot")
            .inner()
            .iter()
            .map(|reversible| reversible.inner().clone())
            .collect()
    }

    /// The two elementary ops this family writes, as the annotated form
    /// [landed_ops] hands back.
    fn set_group_list(
        group_list: GroupListId,
        placements: BTreeMap<StudentId, u32>,
    ) -> AnnotatedOp {
        AnnotatedOp::Colloscope(AnnotatedColloscopeOp::SetGroupList(group_list, placements))
    }

    fn set_interrogation(slot: SlotId, week: WeekId, groups: BTreeSet<u32>) -> AnnotatedOp {
        AnnotatedOp::Colloscope(AnnotatedColloscopeOp::SetInterrogation(slot, week, groups))
    }

    /// The document's colloscope, read back in the payload's own shape.
    fn contents_of(state: &AppState<Data, Desc>) -> ColloscopeContents {
        (&state.get_data().get_inner_data().colloscope).into()
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

    /// Writes `contents` onto a clone of `base` as plain preparation ops: the
    /// pre-state the install fixtures below diff against.
    fn prepare(base: &AppState<Data, Desc>, contents: &ColloscopeContents) -> AppState<Data, Desc> {
        let mut prepared = base.clone();
        for (group_list, placements) in &contents.group_lists {
            prepared
                .apply(
                    Op::Colloscope(ColloscopeOp::SetGroupList(*group_list, placements.clone())),
                    (OpCategory::Colloscope, "Préparation".into()),
                )
                .expect("the fixture's pre-state rows are all legitimate");
        }
        for ((slot, week), groups) in &contents.interrogations {
            prepared
                .apply(
                    Op::Colloscope(ColloscopeOp::SetInterrogation(*slot, *week, groups.clone())),
                    (OpCategory::Colloscope, "Préparation".into()),
                )
                .expect("the fixture's pre-state rows are all legitimate");
        }

        prepared
    }

    /// Métamorphose's slots in id order — the order the interrogation passes
    /// visit them in, their coordinates living in a `BTreeMap`. The subject is
    /// the one with enough slots for a four-way diff, and « Liste principale »
    /// is associated with it on every period, so its eight groups are the bound
    /// every cell below is checked against.
    fn install_slots(data: &Data) -> Vec<SlotId> {
        let metamorphose = subject_by_name(data, "Métamorphose");
        let mut slots = slots_of_subject(data, metamorphose);
        slots.sort();
        assert!(
            slots.len() >= 4,
            "the fixture should have at least four Métamorphose slots"
        );

        slots
    }

    /// Onto an empty colloscope the install is all writes: the payload's rows
    /// in pass order — every group-list row, then every cell — and not a single
    /// clear.
    #[test]
    fn installing_onto_an_empty_colloscope_writes_every_row_and_clears_nothing() {
        let (base, lists) = hogwarts_with_automatic_lists(2);
        let slots = install_slots(base.get_data());
        let week = week_at(base.get_data(), 2);
        let harry = student_by_name(base.get_data(), "Potter", "Harry");
        let ron = student_by_name(base.get_data(), "Weasley", "Ron");

        let contents = ColloscopeContents {
            group_lists: BTreeMap::from([
                (lists[0], BTreeMap::from([(harry, 0)])),
                (lists[1], BTreeMap::from([(ron, 1)])),
            ]),
            interrogations: BTreeMap::from([
                ((slots[0], week), BTreeSet::from([0])),
                ((slots[1], week), BTreeSet::from([1])),
            ]),
        };

        let op = ColloscopeUpdateOp::InstallColloscope(contents.clone());
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            landed_ops(&state),
            vec![
                set_group_list(lists[0], BTreeMap::from([(harry, 0)])),
                set_group_list(lists[1], BTreeMap::from([(ron, 1)])),
                set_interrogation(slots[0], week, BTreeSet::from([0])),
                set_interrogation(slots[1], week, BTreeSet::from([1])),
            ],
        );
        assert_eq!(
            contents_of(&state),
            contents,
            "the document ends holding exactly the payload",
        );
    }

    /// The pin on the whole diff, both tables at once: a row the payload
    /// repeats unchanged, a row it changes, a row it drops and a row it adds.
    ///
    /// What the composite lands is the clears first (group lists, then cells),
    /// then the writes — and nothing at all for the unchanged rows. Reading the
    /// document alone could not say that last part: a body that rewrote every
    /// payload row would leave exactly the same colloscope behind, and only
    /// [landed_ops] tells the two apart.
    #[test]
    fn the_install_diff_clears_what_is_dropped_writes_what_moved_and_skips_what_already_matches() {
        let (base, lists) = hogwarts_with_automatic_lists(4);
        let slots = install_slots(base.get_data());
        let week = week_at(base.get_data(), 2);
        let harry = student_by_name(base.get_data(), "Potter", "Harry");

        // lists[0] / slots[0]: unchanged. lists[1] / slots[1]: changed.
        // lists[2] / slots[2]: dropped. lists[3] / slots[3]: added.
        let base = prepare(
            &base,
            &ColloscopeContents {
                group_lists: BTreeMap::from([
                    (lists[0], BTreeMap::from([(harry, 0)])),
                    (lists[1], BTreeMap::from([(harry, 0)])),
                    (lists[2], BTreeMap::from([(harry, 0)])),
                ]),
                interrogations: BTreeMap::from([
                    ((slots[0], week), BTreeSet::from([0])),
                    ((slots[1], week), BTreeSet::from([0])),
                    ((slots[2], week), BTreeSet::from([0])),
                ]),
            },
        );

        let contents = ColloscopeContents {
            group_lists: BTreeMap::from([
                (lists[0], BTreeMap::from([(harry, 0)])),
                (lists[1], BTreeMap::from([(harry, 1)])),
                (lists[3], BTreeMap::from([(harry, 2)])),
            ]),
            interrogations: BTreeMap::from([
                ((slots[0], week), BTreeSet::from([0])),
                ((slots[1], week), BTreeSet::from([1])),
                ((slots[3], week), BTreeSet::from([2])),
            ]),
        };

        let op = ColloscopeUpdateOp::InstallColloscope(contents.clone());
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            landed_ops(&state),
            vec![
                set_group_list(lists[2], BTreeMap::new()),
                set_interrogation(slots[2], week, BTreeSet::new()),
                set_group_list(lists[1], BTreeMap::from([(harry, 1)])),
                set_group_list(lists[3], BTreeMap::from([(harry, 2)])),
                set_interrogation(slots[1], week, BTreeSet::from([1])),
                set_interrogation(slots[3], week, BTreeSet::from([2])),
            ],
        );
        assert_eq!(
            contents_of(&state),
            contents,
            "the document ends holding exactly the payload",
        );
    }

    /// Installing what the document already holds costs nothing: no elementary
    /// op, and the state comes out bit-identical to the one the op found.
    #[test]
    fn installing_an_identical_payload_lands_no_elementary_op_at_all() {
        let (base, lists) = hogwarts_with_automatic_lists(2);
        let slots = install_slots(base.get_data());
        let week = week_at(base.get_data(), 2);
        let harry = student_by_name(base.get_data(), "Potter", "Harry");

        let contents = ColloscopeContents {
            group_lists: BTreeMap::from([
                (lists[0], BTreeMap::from([(harry, 0)])),
                (lists[1], BTreeMap::from([(harry, 1)])),
            ]),
            interrogations: BTreeMap::from([
                ((slots[0], week), BTreeSet::from([0])),
                ((slots[1], week), BTreeSet::from([1])),
            ]),
        };
        let base = prepare(&base, &contents);

        let op = ColloscopeUpdateOp::InstallColloscope(contents);
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(landed_ops(&state), vec![]);
        assert_eq!(state.get_data(), base.get_data());
    }

    /// The empty payload is the other extreme: every row cleared, nothing
    /// written, and the document left where `EraseColloscope` followed by
    /// `EraseGroupLists` would have left it.
    #[test]
    fn installing_an_empty_payload_clears_the_whole_colloscope() {
        let (base, lists) = hogwarts_with_automatic_lists(2);
        let slots = install_slots(base.get_data());
        let week = week_at(base.get_data(), 2);
        let harry = student_by_name(base.get_data(), "Potter", "Harry");

        let base = prepare(
            &base,
            &ColloscopeContents {
                group_lists: BTreeMap::from([
                    (lists[0], BTreeMap::from([(harry, 0)])),
                    (lists[1], BTreeMap::from([(harry, 1)])),
                ]),
                interrogations: BTreeMap::from([
                    ((slots[0], week), BTreeSet::from([0])),
                    ((slots[1], week), BTreeSet::from([1])),
                ]),
            },
        );

        let op = ColloscopeUpdateOp::InstallColloscope(ColloscopeContents::default());
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            landed_ops(&state),
            vec![
                set_group_list(lists[0], BTreeMap::new()),
                set_group_list(lists[1], BTreeMap::new()),
                set_interrogation(slots[0], week, BTreeSet::new()),
                set_interrogation(slots[1], week, BTreeSet::new()),
            ],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![
                    Op::Colloscope(ColloscopeOp::SetGroupList(lists[0], BTreeMap::new())),
                    Op::Colloscope(ColloscopeOp::SetGroupList(lists[1], BTreeMap::new())),
                    Op::Colloscope(ColloscopeOp::SetInterrogation(
                        slots[0],
                        week,
                        BTreeSet::new()
                    )),
                    Op::Colloscope(ColloscopeOp::SetInterrogation(
                        slots[1],
                        week,
                        BTreeSet::new()
                    )),
                ],
            )
            .get_data(),
        );
    }

    /// A payload built by hand need not be canonical: an empty group set and an
    /// empty placement map mean "no row", exactly as they do for the two
    /// single-row variants. So they clear a coordinate the document holds and
    /// cost nothing at all for one it does not.
    #[test]
    fn an_empty_row_in_the_payload_means_no_row() {
        let (base, lists) = hogwarts_with_automatic_lists(2);
        let slots = install_slots(base.get_data());
        let week = week_at(base.get_data(), 2);
        let harry = student_by_name(base.get_data(), "Potter", "Harry");

        // Only the first list and the first cell are held by the document.
        let base = prepare(
            &base,
            &ColloscopeContents {
                group_lists: BTreeMap::from([(lists[0], BTreeMap::from([(harry, 0)]))]),
                interrogations: BTreeMap::from([((slots[0], week), BTreeSet::from([0]))]),
            },
        );

        let op = ColloscopeUpdateOp::InstallColloscope(ColloscopeContents {
            group_lists: BTreeMap::from([(lists[0], BTreeMap::new()), (lists[1], BTreeMap::new())]),
            interrogations: BTreeMap::from([
                ((slots[0], week), BTreeSet::new()),
                ((slots[1], week), BTreeSet::new()),
            ]),
        });
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            landed_ops(&state),
            vec![
                set_group_list(lists[0], BTreeMap::new()),
                set_interrogation(slots[0], week, BTreeSet::new()),
            ],
            "the rows the document held are cleared, the absent ones cost nothing",
        );
        assert_eq!(contents_of(&state), ColloscopeContents::default());
    }

    /// The install's own error vocabulary, group-list half: the state layer's
    /// precheck on a dead list id, then the four payload scans in the old
    /// validator's order — the prefilled target, the excluded student, the
    /// student who does not exist, the group number past the end of the list.
    ///
    /// Each payload carries the offending row *alone*, so nothing of the
    /// composite has landed when it fails: the shared session below is still
    /// showing the base at the end.
    #[test]
    fn the_install_reports_every_way_a_group_list_row_can_be_wrong() {
        let (base, list) = hogwarts_with_an_automatic_list();
        let prefilled = group_list_by_name(base.get_data(), "Liste principale");
        let harry = student_by_name(base.get_data(), "Potter", "Harry");
        let malefoy = student_by_name(base.get_data(), "Malefoy", "Drago");
        let past_the_end = AUTOMATIC_GROUPS as u32;

        let install_group_list = |group_list, placements| {
            ColloscopeUpdateOp::InstallColloscope(ColloscopeContents {
                group_lists: BTreeMap::from([(group_list, placements)]),
                interrogations: BTreeMap::new(),
            })
        };

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            install_group_list(dangling_group_list(), BTreeMap::from([(harry, 0)]))
                .apply_to_session(&mut session)
                .unwrap_err(),
            ColloscopeUpdateError::InstallColloscope(InstallColloscopeError::InvalidGroupListId(
                dangling_group_list()
            )),
        );
        assert_eq!(
            install_group_list(prefilled, BTreeMap::from([(harry, 0)]))
                .apply_to_session(&mut session)
                .unwrap_err(),
            ColloscopeUpdateError::InstallColloscope(
                InstallColloscopeError::PrefilledGroupListInColloscope(prefilled),
            ),
        );
        assert_eq!(
            install_group_list(list, BTreeMap::from([(malefoy, 0)]))
                .apply_to_session(&mut session)
                .unwrap_err(),
            ColloscopeUpdateError::InstallColloscope(
                InstallColloscopeError::ExcludedStudentInGroupList(list, malefoy),
            ),
        );
        assert_eq!(
            install_group_list(list, BTreeMap::from([(dangling_student(), 0)]))
                .apply_to_session(&mut session)
                .unwrap_err(),
            ColloscopeUpdateError::InstallColloscope(InstallColloscopeError::InvalidStudentId(
                dangling_student()
            )),
        );
        assert_eq!(
            install_group_list(list, BTreeMap::from([(harry, past_the_end)]))
                .apply_to_session(&mut session)
                .unwrap_err(),
            ColloscopeUpdateError::InstallColloscope(
                InstallColloscopeError::InvalidGroupNumForStudentInGroupList(list, harry),
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// The same for the cell half: the two prechecks (week first, then slot),
    /// then the inactive week and the group number past the end of the
    /// associated list. The slot whose subject skips the week's period needs a
    /// document of its own and has its own fixture below.
    #[test]
    fn the_install_reports_every_way_an_interrogation_can_be_wrong() {
        let base = hogwarts();
        let slots = install_slots(base.get_data());
        let active = week_at(base.get_data(), 2);
        // The first two weeks of the school year run no interrogations at all.
        let inactive = week_at(base.get_data(), 0);
        // « Liste principale », associated with Métamorphose, offers eight.
        let past_the_end = 8u32;

        let install_interrogation = |slot, week, groups| {
            ColloscopeUpdateOp::InstallColloscope(ColloscopeContents {
                group_lists: BTreeMap::new(),
                interrogations: BTreeMap::from([((slot, week), groups)]),
            })
        };

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            install_interrogation(slots[0], dangling_week(), BTreeSet::from([0]))
                .apply_to_session(&mut session)
                .unwrap_err(),
            ColloscopeUpdateError::InstallColloscope(InstallColloscopeError::InvalidWeekId(
                dangling_week()
            )),
        );
        assert_eq!(
            install_interrogation(dangling_slot(), active, BTreeSet::from([0]))
                .apply_to_session(&mut session)
                .unwrap_err(),
            ColloscopeUpdateError::InstallColloscope(InstallColloscopeError::InvalidSlotId(
                dangling_slot()
            )),
        );
        assert_eq!(
            install_interrogation(slots[0], inactive, BTreeSet::from([0]))
                .apply_to_session(&mut session)
                .unwrap_err(),
            ColloscopeUpdateError::InstallColloscope(
                InstallColloscopeError::InterrogationOnInactiveWeek(slots[0], inactive),
            ),
        );
        assert_eq!(
            install_interrogation(slots[0], active, BTreeSet::from([past_the_end]))
                .apply_to_session(&mut session)
                .unwrap_err(),
            ColloscopeUpdateError::InstallColloscope(
                InstallColloscopeError::InvalidGroupNumInInterrogation(slots[0], active),
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// The install's last scan, on the document the single-row fixture builds
    /// for the same purpose: Divination taken out of the first period, which
    /// means emptying its assignments row there and dropping its group-list
    /// association first.
    #[test]
    fn the_install_rejects_an_interrogation_on_a_period_the_subject_skips() {
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
            ColloscopeUpdateOp::InstallColloscope(ColloscopeContents {
                group_lists: BTreeMap::new(),
                interrogations: BTreeMap::from([((slot, week), BTreeSet::from([0]))]),
            })
            .apply_to_session(&mut session)
            .unwrap_err(),
            ColloscopeUpdateError::InstallColloscope(
                InstallColloscopeError::SlotNotRunningOnPeriod(slot, week),
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// A payload whose good rows come first and whose bad row comes last still
    /// lands nothing: `dry_apply` works on a clone of the document and drops it
    /// with the session, so the caller's state is bit-identical afterwards.
    #[test]
    fn a_failing_install_leaves_the_document_untouched() {
        let (base, lists) = hogwarts_with_automatic_lists(2);
        let slots = install_slots(base.get_data());
        let week = week_at(base.get_data(), 2);
        let harry = student_by_name(base.get_data(), "Potter", "Harry");

        let base = prepare(
            &base,
            &ColloscopeContents {
                group_lists: BTreeMap::from([(lists[0], BTreeMap::from([(harry, 0)]))]),
                interrogations: BTreeMap::from([((slots[0], week), BTreeSet::from([0]))]),
            },
        );
        let before = base.clone();

        let op = UpdateOp::Colloscope(ColloscopeUpdateOp::InstallColloscope(ColloscopeContents {
            group_lists: BTreeMap::from([(lists[1], BTreeMap::from([(harry, 1)]))]),
            interrogations: BTreeMap::from([
                ((slots[1], week), BTreeSet::from([1])),
                // Past the end of « Liste principale »: the last write of the
                // last pass, so everything else has landed by the time it
                // fails.
                ((slots[2], week), BTreeSet::from([8])),
            ]),
        }));

        // `CascadeResult` carries a `Manager` and is not `Debug`, so the
        // outcome is read by hand rather than through `unwrap_err`.
        let Err(error) = op.dry_apply(&base) else {
            panic!("an out-of-bounds group in the payload should be refused");
        };
        assert_eq!(
            error,
            UpdateError::Colloscope(ColloscopeUpdateError::InstallColloscope(
                InstallColloscopeError::InvalidGroupNumInInterrogation(slots[2], week),
            )),
        );
        assert_eq!(base.get_data(), before.get_data());
    }
}
