//! Solution submodule of [crate::base].
//!
//! This submodule defines the various types to describe a colloscope.
//!
//! The main such structure is [Colloscope] which describes
//! a (partially completed or not) colloscope.
use std::collections::{BTreeMap, BTreeSet};

use super::Identifier;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupList<StudentId: Identifier> {
    pub groups_for_remaining_students: BTreeMap<StudentId, Option<u32>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interrogation<GroupListId: Identifier> {
    pub group_list_id: GroupListId,
    pub assigned_groups: BTreeSet<u32>,
    pub unassigned_groups: BTreeSet<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectInterrogations<SlotId: Identifier, GroupListId: Identifier> {
    pub slots: BTreeMap<SlotId, Vec<Option<Interrogation<GroupListId>>>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Colloscope<
    SubjectId: Identifier,
    SlotId: Identifier,
    GroupListId: Identifier,
    StudentId: Identifier,
> {
    pub subject_map: BTreeMap<SubjectId, SubjectInterrogations<SlotId, GroupListId>>,
    pub group_lists: BTreeMap<GroupListId, GroupList<StudentId>>,
}

use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum ValidationError {
    #[error("The number of weeks varies from slot to slot")]
    InconsistentWeekCount,
    #[error("Invalid group list id in group assignment")]
    InvalidGroupListId,
    #[error("Group is both assigned and not unassigned in slot")]
    InconsistentGroupStatusInSlot,
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    Colloscope<SubjectId, SlotId, GroupListId, StudentId>
{
    pub fn validate(
        self,
    ) -> Result<ValidatedColloscope<SubjectId, SlotId, GroupListId, StudentId>, ValidationError>
    {
        let mut group_list_groups = BTreeMap::new();

        for (group_list_id, group_list) in &self.group_lists {
            let mut groups = BTreeSet::new();
            for (_student_id, group_opt) in &group_list.groups_for_remaining_students {
                if let Some(group) = group_opt {
                    groups.insert(*group);
                }
            }
            group_list_groups.insert(*group_list_id, groups);
        }

        let mut week_count = None;
        for (_subject_id, subject_slots) in &self.subject_map {
            for (_slot_id, slot) in &subject_slots.slots {
                match week_count {
                    Some(count) => {
                        if count != slot.len() {
                            return Err(ValidationError::InconsistentWeekCount);
                        }
                    }
                    None => week_count = Some(slot.len()),
                }

                for week_interrogation_opt in slot {
                    let Some(week_interrogation) = week_interrogation_opt else {
                        continue;
                    };

                    if !self
                        .group_lists
                        .contains_key(&week_interrogation.group_list_id)
                    {
                        return Err(ValidationError::InvalidGroupListId);
                    }

                    for group in &week_interrogation.unassigned_groups {
                        if week_interrogation.assigned_groups.contains(group) {
                            return Err(ValidationError::InconsistentGroupStatusInSlot);
                        }
                    }
                }
            }
        }

        Ok(ValidatedColloscope { internal: self })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedColloscope<
    SubjectId: Identifier,
    SlotId: Identifier,
    GroupListId: Identifier,
    StudentId: Identifier,
> {
    internal: Colloscope<SubjectId, SlotId, GroupListId, StudentId>,
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    ValidatedColloscope<SubjectId, SlotId, GroupListId, StudentId>
{
    pub fn inner(&self) -> &Colloscope<SubjectId, SlotId, GroupListId, StudentId> {
        &self.internal
    }

    pub fn into_inner(self) -> Colloscope<SubjectId, SlotId, GroupListId, StudentId> {
        self.internal
    }
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    std::ops::Deref for ValidatedColloscope<SubjectId, SlotId, GroupListId, StudentId>
{
    type Target = Colloscope<SubjectId, SlotId, GroupListId, StudentId>;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}
