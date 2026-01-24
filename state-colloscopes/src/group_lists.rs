//! Group lists submodule
//!
//! This module defines the relevant types to describes the lists of groups

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

use crate::ids::{GroupListId, PeriodId, StudentId, SubjectId};

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
    /// Prefilled groups (None = automatic filling)
    pub prefilled_groups: Option<GroupListPrefilledGroups>,
}

/// Prefilled groups for a single group list
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupListPrefilledGroups {
    /// group list
    pub groups: Vec<PrefilledGroup>,
}

/// Prefilled groups for a single group list
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrefilledGroup {
    /// Students set
    ///
    /// Set of students that are in the group
    pub students: BTreeSet<StudentId>,
}

impl GroupListPrefilledGroups {
    pub fn check_duplicated_student(&self) -> bool {
        let mut students_so_far = BTreeSet::new();
        for group in &self.groups {
            for student in &group.students {
                if !students_so_far.insert(*student) {
                    return false;
                }
            }
        }
        true
    }

    pub fn iter_students(&self) -> impl Iterator<Item = StudentId> {
        self.groups.iter().flat_map(|g| g.students.iter().copied())
    }

    pub fn remove_student(&mut self, student_id: StudentId) -> bool {
        for group in &mut self.groups {
            if group.students.remove(&student_id) {
                return true;
            }
        }
        false
    }

    pub fn contains_student(&self, student_id: StudentId) -> bool {
        self.find_student_group(student_id).is_some()
    }

    pub fn find_student_group(&self, student_id: StudentId) -> Option<usize> {
        for (num, group) in self.groups.iter().enumerate() {
            if group.students.contains(&student_id) {
                return Some(num);
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
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
    /// Students set that are not covered by the group list
    pub excluded_students: BTreeSet<StudentId>,
}

impl Default for GroupListParameters {
    fn default() -> Self {
        GroupListParameters {
            name: "Liste".into(),
            students_per_group: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            group_names: vec![None; 16], // 16 unnamed groups (typical for a class of 48 with 3 students per group)
            excluded_students: BTreeSet::new(),
        }
    }
}

impl GroupList {
    /// Checks whether the group list is prefilled
    ///
    /// Returns true if prefilled_groups is Some (i.e., groups are prefilled)
    pub fn is_prefilled(&self) -> bool {
        self.prefilled_groups.is_some()
    }

    /// Returns the set of students that are not already in a prefilled group
    pub fn remaining_students_to_dispatch(
        &self,
        students: &BTreeSet<StudentId>,
    ) -> BTreeSet<StudentId> {
        let non_excluded: BTreeSet<_> = students
            .difference(&self.params.excluded_students)
            .copied()
            .collect();

        match &self.prefilled_groups {
            None => non_excluded,
            Some(prefilled) => non_excluded
                .into_iter()
                .filter(|id| !prefilled.contains_student(*id))
                .collect(),
        }
    }
}
