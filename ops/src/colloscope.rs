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
                                        Convergence::ColloscopeStudentGroupOutOfBounds(group_list, student),
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
                                        Convergence::InterrogationGroupOutOfBounds(slot, week),
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
