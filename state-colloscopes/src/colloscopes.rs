//! Colloscopes submodule
//!
//! This module defines the relevant types to describes the colloscopes

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use collomatique_state::ContentOrd;

use crate::Table;
use crate::ids::{GroupListId, SlotId, StudentId, WeekId};
use crate::ops::AnnotatedColloscopeOp;

/// Description of a colloscope
///
/// The colloscope is stored sparsely, in canonical form: a row exists in either
/// table *iff* it is non-empty. The two tables are the only representation —
/// there is no dense skeleton, no per-period/per-slot scaffolding and no
/// `None` cells. The ids in a row should be valid with respect to the
/// corresponding params.
#[derive(Clone, Debug, Default, PartialEq, Eq, ContentOrd)]
pub struct Colloscope {
    /// Assigned groups per `(slot, week)`. A row is present exactly when the
    /// group set is non-empty.
    interrogations: Table<(SlotId, WeekId), BTreeSet<u32>>,
    /// Student→group placements per non-prefilled group list. A row is present
    /// exactly when the placement map is non-empty.
    group_lists: Table<GroupListId, BTreeMap<StudentId, u32>>,
}

impl Colloscope {
    /// Whether there are no interrogation rows. Reads the interrogations table
    /// only — its twin [Self::are_group_lists_empty] covers the other half of
    /// the struct.
    pub fn are_interrogations_empty(&self) -> bool {
        self.interrogations.is_empty()
    }

    /// Whether no group list has any placements. Reads the group-lists table
    /// only — its twin [Self::are_interrogations_empty] covers the other half
    /// of the struct.
    pub fn are_group_lists_empty(&self) -> bool {
        self.group_lists.is_empty()
    }
}

// The colloscope's half of the dense renumbering walk (see [crate::compact]).
// The two methods must visit exactly the same id occurrences: both components
// of an interrogation key, and both the key and the placed students of a
// group-list row. The group *numbers* in either table are not ids.
impl Colloscope {
    pub(crate) fn collect_ids(&self, ids: &mut BTreeSet<u64>) {
        use crate::ids::Id as _;
        for ((slot_id, week_id), _assigned_groups) in self.interrogations.iter() {
            ids.insert(slot_id.inner());
            ids.insert(week_id.inner());
        }
        for (group_list_id, groups_for_students) in self.group_lists.iter() {
            ids.insert(group_list_id.inner());
            for student_id in groups_for_students.keys() {
                ids.insert(student_id.inner());
            }
        }
    }

    pub(crate) fn remap_ids(self, map: &crate::compact::IdMap) -> Self {
        use crate::compact::remap;
        Colloscope {
            interrogations: self
                .interrogations
                .into_iter()
                .map(|((slot_id, week_id), assigned_groups)| {
                    ((remap(map, slot_id), remap(map, week_id)), assigned_groups)
                })
                .collect(),
            group_lists: self
                .group_lists
                .into_iter()
                .map(|(group_list_id, groups_for_students)| {
                    (
                        remap(map, group_list_id),
                        groups_for_students
                            .into_iter()
                            .map(|(student_id, group)| (remap(map, student_id), group))
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}

/// Sparse read/write surface (canonical view: a cell is a "row" iff it holds a
/// non-empty group set / non-empty placement map).
///
/// These are thin accessors over the two sparse tables. `None`/absent cells and
/// empty sets/maps all read as absent; the writers keep that canonical form (an
/// empty write clears the row).
impl Colloscope {
    /// The assigned groups on `(slot, week)`, or `None` when the cell is empty
    /// or absent.
    pub fn interrogation(&self, slot: SlotId, week: WeekId) -> Option<&BTreeSet<u32>> {
        self.interrogations.get(&(slot, week))
    }

    /// Non-empty rows for one slot, each with its week. Order unspecified.
    pub fn interrogations_for_slot(
        &self,
        slot: SlotId,
    ) -> impl Iterator<Item = (WeekId, &BTreeSet<u32>)> {
        self.interrogations
            .iter()
            .filter_map(move |((s, w), groups)| (s == slot).then_some((w, groups)))
    }

    /// Every non-empty interrogation row, keyed by `(slot, week)`. Iteration
    /// order is unspecified (currently `(slot, week)` id order).
    pub fn iter(&self) -> impl Iterator<Item = ((SlotId, WeekId), &BTreeSet<u32>)> {
        self.interrogations.iter()
    }

    /// The placements for a group list, or `None` when the list is empty or
    /// absent.
    pub fn group_list(&self, id: GroupListId) -> Option<&BTreeMap<StudentId, u32>> {
        self.group_lists.get(&id)
    }

    /// Every non-empty group list, keyed by its id.
    pub fn group_lists_iter(
        &self,
    ) -> impl Iterator<Item = (GroupListId, &BTreeMap<StudentId, u32>)> {
        self.group_lists.iter()
    }

    /// Sets the assigned groups on `(slot, week)`. An empty set clears the row
    /// (canonical form). Never panics — this is a plain table upsert.
    pub fn set_interrogation(&mut self, slot: SlotId, week: WeekId, groups: BTreeSet<u32>) {
        if groups.is_empty() {
            self.interrogations.remove(&(slot, week));
        } else {
            self.interrogations.insert((slot, week), groups);
        }
    }

    /// Sets the placements for a group list. An empty map clears the row
    /// (canonical form). Never panics — this is a plain table upsert.
    pub fn set_group_list(&mut self, id: GroupListId, placements: BTreeMap<StudentId, u32>) {
        if placements.is_empty() {
            self.group_lists.remove(&id);
        } else {
            self.group_lists.insert(id, placements);
        }
    }

    /// Test-only corruption: inserts an interrogation row verbatim, bypassing
    /// the canonicalizing [Self::set_interrogation] — a stored empty row is
    /// exactly the [crate::invariants::LogicError::EmptyInterrogationRow] (new
    /// checker) the invariant checker must detect, and no production surface can
    /// produce it.
    #[cfg(test)]
    pub(crate) fn forge_interrogation_row(
        &mut self,
        slot: SlotId,
        week: WeekId,
        groups: BTreeSet<u32>,
    ) {
        self.interrogations.insert((slot, week), groups);
    }

    /// Test-only corruption: inserts a group-list row verbatim, bypassing the
    /// canonicalizing [Self::set_group_list] — a stored empty row is exactly
    /// the [crate::invariants::LogicError::EmptyColloscopeGroupListRow] (new
    /// checker) the invariant checker must detect.
    #[cfg(test)]
    pub(crate) fn forge_group_list_row(
        &mut self,
        id: GroupListId,
        placements: BTreeMap<StudentId, u32>,
    ) {
        self.group_lists.insert(id, placements);
    }
}

/// Precondition errors of the forced colloscope ops — the carve-out subset
/// (step-3 survey Table 2). Kept: the `SetGroupList` target existence
/// ([Self::InvalidGroupListId]) and the `SetInterrogation` coordinate existence
/// ([Self::InvalidWeekId] / [Self::InvalidSlotId]). The `SetGroupList`
/// prefilled/placement guards and all three `SetInterrogation` semantic guards
/// are stripped.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ColloscopePrecheckError {
    /// Group list original id is invalid
    #[error("invalid group list id ({0:?})")]
    InvalidGroupListId(GroupListId),

    /// The week id in a colloscope op does not resolve to any period
    #[error("invalid week id ({0:?})")]
    InvalidWeekId(WeekId),

    /// Slot original id is invalid
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(SlotId),
}

impl crate::Data {
    /// Used internally by [crate::Data::force_apply]
    ///
    /// Force-applies a colloscope op: the coordinate-existence carve-outs
    /// are kept (returned as [ColloscopePrecheckError] — `SetGroupList` target,
    /// `SetInterrogation` week + slot), the `SetGroupList` prefilled/placement
    /// guards and the three `SetInterrogation` semantic guards are stripped
    /// (step-3 survey Table 1). Sparse writers copied verbatim. May leave the
    /// state invalid; the caller owns checking and rollback.
    pub(crate) fn force_apply_colloscope(
        &mut self,
        colloscope_op: &AnnotatedColloscopeOp,
    ) -> std::result::Result<AnnotatedColloscopeOp, ColloscopePrecheckError> {
        match colloscope_op {
            AnnotatedColloscopeOp::SetGroupList(group_list_id, placements) => {
                if self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                    .is_none()
                {
                    return Err(ColloscopePrecheckError::InvalidGroupListId(*group_list_id));
                }

                // stripped: is_prefilled guard + validate_group_list_placements

                // Read the prior placements for the reverse op, then write.
                let old_placements = self
                    .inner_data
                    .colloscope
                    .group_list(*group_list_id)
                    .cloned()
                    .unwrap_or_default();
                self.inner_data
                    .colloscope
                    .set_group_list(*group_list_id, placements.clone());

                Ok(AnnotatedColloscopeOp::SetGroupList(
                    *group_list_id,
                    old_placements,
                ))
            }
            AnnotatedColloscopeOp::SetInterrogation(slot_id, week_id, assigned_groups) => {
                // Resolve the week to its (period, position) coordinate (kept).
                if self
                    .inner_data
                    .params
                    .weeks
                    .week_position(*week_id)
                    .is_none()
                {
                    return Err(ColloscopePrecheckError::InvalidWeekId(*week_id));
                }

                // The slot must exist (kept).
                if self
                    .inner_data
                    .params
                    .slots
                    .find_slot_with_subject(*slot_id)
                    .is_none()
                {
                    return Err(ColloscopePrecheckError::InvalidSlotId(*slot_id));
                }

                // stripped: SlotNotRunningOnPeriod + InterrogationOnInactiveWeek
                // + InvalidGroupNumInInterrogation group-bound guard

                // Read the prior groups for the reverse op, then write.
                let old_groups = self
                    .inner_data
                    .colloscope
                    .interrogation(*slot_id, *week_id)
                    .cloned()
                    .unwrap_or_default();
                self.inner_data.colloscope.set_interrogation(
                    *slot_id,
                    *week_id,
                    assigned_groups.clone(),
                );

                Ok(AnnotatedColloscopeOp::SetInterrogation(
                    *slot_id, *week_id, old_groups,
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::Id;
    use crate::refs::{Reference, SlotRefSite, WeekRefSite};
    use crate::{FixableInvariant, InnerData, LogicError};

    /// Stage-6 backfill: a stored empty interrogation row — unreachable through
    /// any public path — is a tier-2 logic error.
    #[test]
    fn empty_interrogation_row_rejected() {
        let mut data = InnerData::default();
        let slot = unsafe { SlotId::new(1) };
        let week = unsafe { WeekId::new(2) };
        data.colloscope
            .forge_interrogation_row(slot, week, BTreeSet::new());
        assert_eq!(
            data.broken_invariants(),
            Err(BTreeSet::from([LogicError::EmptyInterrogationRow(
                slot, week
            )]))
        );
    }

    /// Stage-6 backfill: a stored empty group-list row is likewise a tier-2
    /// logic error.
    #[test]
    fn empty_group_list_row_rejected() {
        let mut data = InnerData::default();
        let group_list = unsafe { GroupListId::new(1) };
        data.colloscope
            .forge_group_list_row(group_list, BTreeMap::new());
        assert_eq!(
            data.broken_invariants(),
            Err(BTreeSet::from([LogicError::EmptyColloscopeGroupListRow(
                group_list
            )]))
        );
    }

    /// A non-empty forged row with dangling coordinates reports *both* dangling
    /// ids — and nothing else: the convergence checks on the cell all skip when
    /// the slot or week fails to resolve.
    #[test]
    fn non_empty_forged_row_reports_dangling_ids() {
        let mut data = InnerData::default();
        let slot = unsafe { SlotId::new(1) };
        let week = unsafe { WeekId::new(2) };
        data.colloscope
            .forge_interrogation_row(slot, week, BTreeSet::from([0]));
        assert_eq!(
            data.broken_invariants(),
            Ok(BTreeSet::from([
                FixableInvariant::DanglingFk(Reference::Week {
                    target: week,
                    site: WeekRefSite::ColloscopeInterrogation { slot },
                }),
                FixableInvariant::DanglingFk(Reference::Slot {
                    target: slot,
                    site: SlotRefSite::ColloscopeInterrogation { week },
                }),
            ]))
        );
    }
}
