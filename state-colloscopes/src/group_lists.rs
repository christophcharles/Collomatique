//! Group lists submodule
//!
//! This module defines the relevant types to describes the lists of groups

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{GroupListId, PeriodId, SlotId, StudentId, SubjectId};

/// Description of the group lists
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupLists {
    /// Group lists
    ///
    /// Each item associates a group list id to an actual group list
    pub group_list_map: BTreeMap<GroupListId, GroupList>,

    /// Associations between subjects and group lists
    ///
    /// If a subject does not appear no group list has been associated to it
    pub subjects_associations: BTreeMap<PeriodId, BTreeMap<SubjectId, GroupListId>>,
}

/// Description of a single group list
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GroupList {
    /// parameters for the group list
    pub params: GroupListParameters,
    /// Filling strategy for the group list
    pub filling: GroupListFilling,
}

/// Filling strategy for a group list
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupListFilling {
    /// Groups are filled manually with prefilled students
    Prefilled { groups: Vec<PrefilledGroup> },
    /// Groups are filled automatically, except for excluded students
    Automatic {
        excluded_students: BTreeSet<StudentId>,
    },
}

impl Default for GroupListFilling {
    fn default() -> Self {
        GroupListFilling::Automatic {
            excluded_students: BTreeSet::new(),
        }
    }
}

impl GroupListFilling {
    /// Returns true if the filling is prefilled
    pub fn is_prefilled(&self) -> bool {
        matches!(self, GroupListFilling::Prefilled { .. })
    }

    /// Returns the excluded students (empty set for Prefilled variant)
    pub fn excluded_students(&self) -> &BTreeSet<StudentId> {
        match self {
            GroupListFilling::Automatic { excluded_students } => excluded_students,
            GroupListFilling::Prefilled { .. } => {
                static EMPTY: std::sync::LazyLock<BTreeSet<StudentId>> =
                    std::sync::LazyLock::new(BTreeSet::new);
                &EMPTY
            }
        }
    }

    /// Checks that no student appears twice in the groups (for Prefilled variant)
    pub fn check_duplicated_student(&self) -> bool {
        match self {
            GroupListFilling::Prefilled { groups } => {
                let mut students_so_far = BTreeSet::new();
                for group in groups {
                    for student in &group.students {
                        if !students_so_far.insert(*student) {
                            return false;
                        }
                    }
                }
                true
            }
            GroupListFilling::Automatic { .. } => true,
        }
    }

    /// Iterates over all students in prefilled groups (empty for Automatic)
    pub fn iter_students(&self) -> impl Iterator<Item = StudentId> + '_ {
        match self {
            GroupListFilling::Prefilled { groups } => {
                Some(groups.iter().flat_map(|g| g.students.iter().copied()))
            }
            GroupListFilling::Automatic { .. } => None,
        }
        .into_iter()
        .flatten()
    }

    /// Removes a student from prefilled groups (returns true if found)
    pub fn remove_student(&mut self, student_id: StudentId) -> bool {
        match self {
            GroupListFilling::Prefilled { groups } => {
                for group in groups {
                    if group.students.remove(&student_id) {
                        return true;
                    }
                }
                false
            }
            GroupListFilling::Automatic { .. } => false,
        }
    }

    /// Returns true if the student is in a prefilled group
    pub fn contains_student(&self, student_id: StudentId) -> bool {
        self.find_student_group(student_id).is_some()
    }

    /// Finds the group number of a student (for Prefilled variant)
    pub fn find_student_group(&self, student_id: StudentId) -> Option<usize> {
        match self {
            GroupListFilling::Prefilled { groups } => {
                for (num, group) in groups.iter().enumerate() {
                    if group.students.contains(&student_id) {
                        return Some(num);
                    }
                }
                None
            }
            GroupListFilling::Automatic { .. } => None,
        }
    }

    /// Returns the number of groups (for Prefilled variant, 0 for Automatic)
    pub fn groups_len(&self) -> usize {
        match self {
            GroupListFilling::Prefilled { groups } => groups.len(),
            GroupListFilling::Automatic { .. } => 0,
        }
    }
}

/// Prefilled groups for a single group list
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrefilledGroup {
    /// Students set
    ///
    /// Set of students that are in the group
    pub students: BTreeSet<StudentId>,
}

/// Parameters for a single group list
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupListParameters {
    /// Name for the list
    pub name: String,
    /// Range of possible count of students per group
    pub students_per_group: RangeInclusive<NonZeroU32>,
    /// Group names (length determines max group count, None = unnamed group)
    pub group_names: Vec<Option<non_empty_string::NonEmptyString>>,
}

impl Default for GroupListParameters {
    fn default() -> Self {
        GroupListParameters {
            name: "Liste".into(),
            students_per_group: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            group_names: vec![None; 16], // 16 unnamed groups (typical for a class of 48 with 3 students per group)
        }
    }
}

impl GroupList {
    /// Checks whether the group list is prefilled
    ///
    /// Returns true if filling is Prefilled variant
    pub fn is_prefilled(&self) -> bool {
        self.filling.is_prefilled()
    }

    /// Returns the set of students that are not already in a prefilled group
    pub fn students(&self, students: &BTreeSet<StudentId>) -> BTreeSet<StudentId> {
        match &self.filling {
            GroupListFilling::Automatic { excluded_students } => {
                students.difference(excluded_students).copied().collect()
            }
            GroupListFilling::Prefilled { groups } => groups
                .iter()
                .flat_map(|g| g.students.iter().copied())
                .collect(),
        }
    }
}

/// Errors for group list operations
///
/// These errors can be returned when trying to modify [crate::Data] with a group list op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum GroupListError {
    /// group list id is invalid
    #[error("invalid group list id ({0:?})")]
    InvalidGroupListId(GroupListId),

    /// The group list id already exists
    #[error("group list id ({0:?}) already exists")]
    GroupListIdAlreadyExists(GroupListId),

    /// student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),

    /// subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// subject does not have interrogations
    #[error("subject id ({0:?}) has no interrogations")]
    SubjectHasNoInterrogation(SubjectId),

    /// period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// Subject does not run on given period
    #[error("invalid subject id {0:?} for period {1:?}")]
    SubjectDoesNotRunOnPeriod(SubjectId, PeriodId),

    /// students per group range is empty
    #[error("students_per_group range is empty")]
    StudentsPerGroupRangeIsEmpty,

    /// cannot remove group list as it still has a filling (prefilled or automatic with exclusions)
    #[error("Group list still has a filling and cannot be removed")]
    RemainingFilling,

    /// students appear multiple times in prefilled groups
    #[error("Some students appear multiple times in prefilled groups")]
    DuplicatedStudentInPrefilledGroups,

    /// cannot remove group list as there are still associated subjects
    #[error("Group list still is associated to subjects and cannot be removed")]
    RemainingAssociatedSubjects,

    /// Group list is not empty in colloscope
    #[error("group list id {0:?} in colloscope is not empty")]
    NotEmptyGroupListInColloscope(GroupListId),

    /// Group list in colloscope not compatible with new parameters
    #[error("group list id {0:?} in colloscope is not compatible with the given parameters")]
    NotCompatibleGroupListInColloscope(GroupListId),

    /// The subject has non-empty slots associated to the old group list with invalid numbers
    #[error(
        "subject {0:?} in colloscope has non-empty slots (slot {2:?}) in period {1:?} with invalid group number"
    )]
    InvalidGroupInSubjectSlotInColloscope(SubjectId, PeriodId, SlotId),

    /// Prefilled groups count does not match group_names count
    #[error("prefilled groups count ({actual}) does not match group names count ({expected})")]
    PrefillGroupCountMismatch { expected: usize, actual: usize },

    /// Cannot reduce group count when last groups have students
    #[error(
        "cannot reduce group count: groups to be removed still have students (ops layer should clean first)"
    )]
    NonEmptyGroupsWhenReducing,

    /// Cannot set prefilling: colloscope group list has students assigned
    #[error("Cannot set prefilling: colloscope group list {0:?} has students assigned")]
    NonEmptyColloscopeGroupListWhenPrefilling(GroupListId),
}
