//! Colloscopes submodule
//!
//! This module defines the relevant types to describes the colloscopes

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Colloscope {
    /// Assigned groups per `(slot, week)`. A row is present exactly when the
    /// group set is non-empty.
    interrogations: Table<(SlotId, WeekId), BTreeSet<u32>>,
    /// Student→group placements per non-prefilled group list. A row is present
    /// exactly when the placement map is non-empty.
    group_lists: Table<GroupListId, BTreeMap<StudentId, u32>>,
}

impl Colloscope {
    pub fn is_empty(&self) -> bool {
        self.interrogations.is_empty()
    }

    pub fn are_group_lists_empty(&self) -> bool {
        self.group_lists.is_empty()
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
    /// checker) / [ColloscopeError::EmptyInterrogationRow] (old checker) the
    /// invariant checkers must detect, and no production surface can produce it.
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
    /// checker) / [ColloscopeError::EmptyGroupListRow] (old checker) the
    /// invariant checkers must detect.
    #[cfg(test)]
    pub(crate) fn forge_group_list_row(
        &mut self,
        id: GroupListId,
        placements: BTreeMap<StudentId, u32>,
    ) {
        self.group_lists.insert(id, placements);
    }
}

impl Colloscope {
    /// Validates every stored row against the parameters, row by row (there is
    /// no skeleton to reconstruct or count against — mirrors
    /// `check_assignments_data_consistency`). On canonical, validated data
    /// every row resolves; a surviving invalid row is a bug.
    pub(crate) fn validate_against_params(
        &self,
        params: &super::colloscope_params::Parameters,
    ) -> Result<(), ColloscopeError> {
        // Interrogation rows: the row must be canonically non-empty, the
        // coordinate must be a possible interrogation cell and the assigned
        // groups must fit the `(period, subject)` association bound.
        for ((slot_id, week_id), assigned_groups) in self.interrogations.iter() {
            if assigned_groups.is_empty() {
                return Err(ColloscopeError::EmptyInterrogationRow(slot_id, week_id));
            }
            let Some((period_id, _)) = params.periods.week_position(week_id) else {
                return Err(ColloscopeError::InvalidWeekId(week_id));
            };

            let Some((subject_id, slot)) = params.slots.find_slot_with_subject(slot_id) else {
                return Err(ColloscopeError::InvalidSlotId(slot_id));
            };
            let subject = params
                .subjects
                .find_subject(subject_id)
                .expect("subject id from a live slot is valid");
            if subject.parameters.interrogation_parameters.is_none()
                || subject.excluded_periods.contains(&period_id)
            {
                return Err(ColloscopeError::SlotNotRunningOnPeriod(slot_id, week_id));
            }

            if !params.is_week_active(week_id, slot.week_pattern) {
                return Err(ColloscopeError::InterrogationOnInactiveWeek(
                    slot_id, week_id,
                ));
            }

            let first_forbidden_value: u32 = params
                .group_lists
                .subjects_associations
                .get(&(period_id, subject_id))
                .map(|group_list_id| {
                    params
                        .group_lists
                        .group_list_map
                        .get(group_list_id)
                        .expect("association references a live group list")
                        .params
                        .group_names
                        .len() as u32
                })
                .unwrap_or(0);
            for group_num in assigned_groups {
                if *group_num >= first_forbidden_value {
                    return Err(ColloscopeError::InvalidGroupNumInInterrogation(
                        slot_id, week_id,
                    ));
                }
            }
        }

        // Group-list rows: the row must be canonically non-empty, the id must
        // resolve to a non-prefilled group list and its placements must be
        // consistent with the params (excluded/invalid students, group numbers).
        for (group_list_id, placements) in self.group_lists.iter() {
            if placements.is_empty() {
                return Err(ColloscopeError::EmptyGroupListRow(group_list_id));
            }
            let Some(params_group_list) = params.group_lists.group_list_map.get(&group_list_id)
            else {
                return Err(ColloscopeError::InvalidGroupListId(group_list_id));
            };
            if params_group_list.is_prefilled() {
                return Err(ColloscopeError::PrefilledGroupListInColloscope(
                    group_list_id,
                ));
            }
            validate_group_list_placements(
                group_list_id,
                placements,
                &params_group_list.params,
                &params_group_list.filling,
                &params.students,
            )?;
        }

        Ok(())
    }
}

/// Validates a raw student→group placement map for one group list against its
/// params, filling and the students table. Operates on the sparse surface value
/// (`Colloscope::group_list`).
pub(crate) fn validate_group_list_placements(
    group_list_id: GroupListId,
    groups_for_students: &BTreeMap<StudentId, u32>,
    group_list_params: &super::group_lists::GroupListParameters,
    group_list_filling: &super::group_lists::GroupListFilling,
    students: &super::students::Students,
) -> Result<(), ColloscopeError> {
    let first_forbidden_value = group_list_params.group_names.len() as u32;
    let excluded_students = group_list_filling.excluded_students();

    for (student_id, group_num) in groups_for_students {
        if excluded_students.contains(student_id) {
            return Err(ColloscopeError::ExcludedStudentInGroupList(
                group_list_id,
                *student_id,
            ));
        }

        if !students.student_map.contains(student_id) {
            return Err(ColloscopeError::InvalidStudentId(*student_id));
        }

        if *group_num >= first_forbidden_value {
            return Err(ColloscopeError::InvalidGroupNumForStudentInGroupList(
                group_list_id,
                *student_id,
            ));
        }
    }

    Ok(())
}

/// Errors for colloscopes operations
///
/// These errors can be returned when trying to modify [crate::Data] with a colloscope op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ColloscopeError {
    /// Student original id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),

    /// Slot original id is invalid
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(SlotId),

    /// Group list original id is invalid
    #[error("invalid group list id ({0:?})")]
    InvalidGroupListId(GroupListId),

    /// A group number in an interrogation row is out of range for the slot's
    /// `(period, subject)` group-list association
    #[error("invalid group number in interrogation on slot {0:?}, week {1:?}")]
    InvalidGroupNumInInterrogation(SlotId, WeekId),

    #[error("excluded student in group list")]
    ExcludedStudentInGroupList(GroupListId, StudentId),

    #[error("Invalid group number for student")]
    InvalidGroupNumForStudentInGroupList(GroupListId, StudentId),

    #[error("Prefilled group list {0:?} should not be in colloscope")]
    PrefilledGroupListInColloscope(GroupListId),

    /// The week id in a colloscope op does not resolve to any period
    #[error("invalid week id ({0:?})")]
    InvalidWeekId(WeekId),

    /// The slot's subject does not run interrogations on the week's period
    #[error("slot {0:?} does not run on the period of week {1:?}")]
    SlotNotRunningOnPeriod(SlotId, WeekId),

    /// The week is inactive for the slot (excluded by pattern or not an
    /// interrogation week)
    #[error("interrogation on inactive week {1:?} for slot {0:?}")]
    InterrogationOnInactiveWeek(SlotId, WeekId),

    /// A stored interrogation row with an empty group set — canonically
    /// unrepresentable (the sparse surface drops empty writes); only in-crate
    /// corruption can produce it. Stage-6 backfill for old-checker completeness.
    #[error("empty interrogation row stored for slot {0:?}, week {1:?}")]
    EmptyInterrogationRow(SlotId, WeekId),

    /// A stored colloscope group-list row with an empty placement map — same
    /// canonical-absent contract as [Self::EmptyInterrogationRow].
    #[error("empty group-list row stored for group list {0:?}")]
    EmptyGroupListRow(GroupListId),
}

impl crate::Data {
    /// Used internally
    ///
    /// Apply colloscope operations
    pub(crate) fn apply_colloscope(
        &mut self,
        colloscope_op: &AnnotatedColloscopeOp,
    ) -> std::result::Result<AnnotatedColloscopeOp, ColloscopeError> {
        match colloscope_op {
            AnnotatedColloscopeOp::SetGroupList(group_list_id, placements) => {
                let Some(params_group_list) = self
                    .inner_data
                    .params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                else {
                    return Err(ColloscopeError::InvalidGroupListId(*group_list_id));
                };

                // Prefilled group lists have a params entry but no colloscope
                // row: the op targets them by mistake. Rejecting via the params
                // `is_prefilled` flag keeps this check meaningful under the
                // sparse tables.
                if params_group_list.is_prefilled() {
                    return Err(ColloscopeError::InvalidGroupListId(*group_list_id));
                }

                // Same validation the dense wrapper ran, against the raw payload.
                validate_group_list_placements(
                    *group_list_id,
                    placements,
                    &params_group_list.params,
                    &params_group_list.filling,
                    &self.inner_data.params.students,
                )?;

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
                let params = &self.inner_data.params;

                // Resolve the week to its (period, position) coordinate.
                let Some((period_id, _)) = params.periods.week_position(*week_id) else {
                    return Err(ColloscopeError::InvalidWeekId(*week_id));
                };

                // The slot must exist and its subject must run interrogations on
                // this period.
                let Some((subject_id, slot)) = params.slots.find_slot_with_subject(*slot_id) else {
                    return Err(ColloscopeError::InvalidSlotId(*slot_id));
                };
                let subject = params
                    .subjects
                    .find_subject(subject_id)
                    .expect("subject id from a live slot is valid");
                if subject.parameters.interrogation_parameters.is_none()
                    || subject.excluded_periods.contains(&period_id)
                {
                    return Err(ColloscopeError::SlotNotRunningOnPeriod(*slot_id, *week_id));
                }

                // The week must be active for the slot's pattern.
                if !params.is_week_active(*week_id, slot.week_pattern) {
                    return Err(ColloscopeError::InterrogationOnInactiveWeek(
                        *slot_id, *week_id,
                    ));
                }

                // Group numbers are bounded by the group list associated to the
                // slot's subject on this period (no association => no valid group).
                let first_forbidden_value: u32 = params
                    .group_lists
                    .subjects_associations
                    .get(&(period_id, subject_id))
                    .map(|group_list_id| {
                        params
                            .group_lists
                            .group_list_map
                            .get(group_list_id)
                            .expect("association references a live group list")
                            .params
                            .group_names
                            .len() as u32
                    })
                    .unwrap_or(0);
                for group_num in assigned_groups {
                    if *group_num >= first_forbidden_value {
                        return Err(ColloscopeError::InvalidGroupNumInInterrogation(
                            *slot_id, *week_id,
                        ));
                    }
                }

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
    use crate::{InnerData, InnerDataError};

    /// Stage-6 backfill: a stored empty interrogation row — unreachable through
    /// any public path — is rejected by the old checker.
    #[test]
    fn empty_interrogation_row_rejected() {
        let mut data = InnerData::default();
        let slot = unsafe { SlotId::new(1) };
        let week = unsafe { WeekId::new(2) };
        data.colloscope
            .forge_interrogation_row(slot, week, BTreeSet::new());
        assert_eq!(
            data.check_invariants(),
            Err(InnerDataError::ColloscopeError(
                ColloscopeError::EmptyInterrogationRow(slot, week)
            ))
        );
    }

    /// Stage-6 backfill: a stored empty group-list row is likewise rejected.
    #[test]
    fn empty_group_list_row_rejected() {
        let mut data = InnerData::default();
        let group_list = unsafe { GroupListId::new(1) };
        data.colloscope
            .forge_group_list_row(group_list, BTreeMap::new());
        assert_eq!(
            data.check_invariants(),
            Err(InnerDataError::ColloscopeError(
                ColloscopeError::EmptyGroupListRow(group_list)
            ))
        );
    }

    /// Precedence: emptiness fires before id resolution, but a non-empty row
    /// with dangling coordinates still reports the dangling id.
    #[test]
    fn non_empty_forged_row_reports_dangling_ids() {
        let mut data = InnerData::default();
        let slot = unsafe { SlotId::new(1) };
        let week = unsafe { WeekId::new(2) };
        data.colloscope
            .forge_interrogation_row(slot, week, BTreeSet::from([0]));
        assert_eq!(
            data.check_invariants(),
            Err(InnerDataError::ColloscopeError(
                ColloscopeError::InvalidWeekId(week)
            ))
        );
    }
}
