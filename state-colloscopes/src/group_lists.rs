//! Group lists submodule
//!
//! This module defines the relevant types to describes the lists of groups

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use std::ops::RangeInclusive;

use crate::ids::{GroupListId, PeriodId, StudentId, SubjectId};

/// Description of the group lists
#[derive(Clone, Debug, PartialEq, Eq, Default)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupList {
    /// parameters for the group list
    pub params: GroupListParameters,
    /// Prefilled groups
    pub prefilled_groups: GroupListPrefilledGroups,
}

/// Parameters for a single group list
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct GroupListPrefilledGroups {
    /// student map
    /// To each student, associate a group number
    /// A student that appears in [GroupListParameters::excluded_students]
    /// should not appear here
    ///
    /// If a student appears in neither there is no prefilled association
    pub student_map: BTreeMap<StudentId, u32>,
}

impl GroupListPrefilledGroups {
    pub fn contains_student(&self, student_id: StudentId) -> bool {
        self.student_map.contains_key(&student_id)
    }

    pub fn is_empty(&self) -> bool {
        self.student_map.is_empty()
    }
}

impl GroupListPrefilledGroups {
    /// Builds an interrogation slot from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [GroupListPrefilledGroupsExternalData::validate].
    pub(crate) unsafe fn from_external_data(
        external_data: GroupListPrefilledGroupsExternalData,
    ) -> GroupListPrefilledGroups {
        GroupListPrefilledGroups {
            student_map: external_data
                .student_map
                .into_iter()
                .map(|(id, group_num)| (unsafe { StudentId::new(id) }, group_num))
                .collect(),
        }
    }
}

/// Parameters for a single group list
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupListParameters {
    /// Name for the list
    pub name: String,
    /// Range of possible count of students per group
    pub students_per_group: RangeInclusive<NonZeroU32>,
    /// Range of possible number of groups in the list
    pub group_count: RangeInclusive<u32>,
    /// Students set that are not covered by the group list
    pub excluded_students: BTreeSet<StudentId>,
}

impl GroupListParameters {
    /// Builds an interrogation slot from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [GroupListsExternalData::validate_all], [GroupListExternalData::validate] and
    /// [GroupListParametersExternalData::validate].
    pub(crate) unsafe fn from_external_data(
        external_data: GroupListParametersExternalData,
    ) -> GroupListParameters {
        GroupListParameters {
            name: external_data.name,
            students_per_group: external_data.students_per_group,
            group_count: external_data.group_count,
            excluded_students: external_data
                .excluded_students
                .into_iter()
                .map(|x| unsafe { StudentId::new(x) })
                .collect(),
        }
    }
}

impl GroupList {
    /// Builds an interrogation slot from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [GroupListsExternalData::validate_all], [GroupListExternalData::validate] and
    /// [GroupListParametersExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: GroupListExternalData) -> GroupList {
        GroupList {
            params: unsafe { GroupListParameters::from_external_data(external_data.params) },
            prefilled_groups: unsafe {
                GroupListPrefilledGroups::from_external_data(external_data.prefilled_groups)
            },
        }
    }
}

impl GroupLists {
    /// Builds interrogation slots from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [GroupListsExternalData::validate_all], [GroupListExternalData::validate] and
    /// [GroupListParametersExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: GroupListsExternalData) -> GroupLists {
        GroupLists {
            group_list_map: external_data
                .group_list_map
                .into_iter()
                .map(|(id, group_list)| {
                    (unsafe { GroupListId::new(id) }, unsafe {
                        GroupList::from_external_data(group_list)
                    })
                })
                .collect(),
            subjects_associations: external_data
                .subjects_associations
                .into_iter()
                .map(|(period_id, subject_map)| {
                    (
                        unsafe { PeriodId::new(period_id) },
                        subject_map
                            .into_iter()
                            .map(|(subject_id, group_list_id)| {
                                (unsafe { SubjectId::new(subject_id) }, unsafe {
                                    GroupListId::new(group_list_id)
                                })
                            })
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}

/// Description of the group lists but unchecked
///
/// This structure is an unchecked equivalent of [GroupLists].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct GroupListsExternalData {
    /// Group lists
    ///
    /// Each item associates a group list id to an actual group list
    pub group_list_map: BTreeMap<u64, GroupListExternalData>,

    /// Associations between subjects and group lists
    ///
    /// If a subject does not appear no group list has been associated to it
    pub subjects_associations: BTreeMap<u64, BTreeMap<u64, u64>>,
}

impl GroupListsExternalData {
    /// Checks the validity [GroupListsExternalData]
    pub fn validate_all(
        &self,
        subjects: &super::SubjectsExternalData,
        student_ids: &BTreeSet<u64>,
        period_ids: &BTreeSet<u64>,
    ) -> bool {
        if period_ids.len() != self.subjects_associations.len() {
            return false;
        }
        for (period_id, subject_map) in &self.subjects_associations {
            if !period_ids.contains(period_id) {
                return false;
            }
            for (subject_id, group_list_id) in subject_map {
                let Some(subject) = subjects.find_subject(*subject_id) else {
                    return false;
                };

                if subject.parameters.interrogation_parameters.is_none() {
                    return false;
                }

                if subject.excluded_periods.contains(period_id) {
                    return false;
                }

                if !self.group_list_map.contains_key(group_list_id) {
                    return false;
                }
            }
        }
        self.group_list_map
            .iter()
            .all(|(_group_list_id, data)| data.validate(student_ids))
    }
}

/// Description of a single group list but unchecked
///
/// This structure is an unchecked equivalent of [GroupList].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupListExternalData {
    /// Parameters for the group list
    pub params: GroupListParametersExternalData,
    /// Prefilled groups
    pub prefilled_groups: GroupListPrefilledGroupsExternalData,
}

/// Parameters for a single group list but unchecked
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupListPrefilledGroupsExternalData {
    /// student map
    /// To each student, associate a group number
    /// A student that appears in [GroupListParameters::excluded_students]
    /// should not appear here
    ///
    /// If a student appears in neither there is no prefilled association
    pub student_map: BTreeMap<u64, u32>,
}

/// Parameters of a single group list but unchecked
///
/// This structure is an unchecked equivalent of [GroupListParameters].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupListParametersExternalData {
    /// Name for the list
    pub name: String,
    /// Range of possible count of students per group
    pub students_per_group: RangeInclusive<NonZeroU32>,
    /// Range of possible number of groups in the list
    pub group_count: RangeInclusive<u32>,
    /// Students set that are not covered by the group list
    pub excluded_students: BTreeSet<u64>,
}

impl GroupListExternalData {
    /// Checks the validity of a [GroupListExternalData]
    pub fn validate(&self, student_ids: &BTreeSet<u64>) -> bool {
        if !self.params.validate(student_ids) {
            return false;
        }
        if !self
            .prefilled_groups
            .validate(student_ids, &self.params.excluded_students)
        {
            return false;
        }
        true
    }
}

impl From<GroupList> for GroupListExternalData {
    fn from(value: GroupList) -> Self {
        GroupListExternalData {
            params: value.params.into(),
            prefilled_groups: value.prefilled_groups.into(),
        }
    }
}

impl GroupListPrefilledGroupsExternalData {
    /// Checks the validity of a [GroupListPrefilledGroupsExternalData]
    pub fn validate(&self, student_ids: &BTreeSet<u64>, excluded_students: &BTreeSet<u64>) -> bool {
        if !self.student_map.iter().all(|(id, _group_num)| {
            if !student_ids.contains(id) {
                return false;
            }
            if excluded_students.contains(id) {
                return false;
            }
            true
        }) {
            return false;
        }
        true
    }
}

impl From<GroupListPrefilledGroups> for GroupListPrefilledGroupsExternalData {
    fn from(value: GroupListPrefilledGroups) -> Self {
        GroupListPrefilledGroupsExternalData {
            student_map: value
                .student_map
                .into_iter()
                .map(|(id, group)| (id.inner(), group))
                .collect(),
        }
    }
}

impl GroupListParametersExternalData {
    /// Checks the validity of a [GroupListExternalData]
    pub fn validate(&self, student_ids: &BTreeSet<u64>) -> bool {
        if self.students_per_group.is_empty() {
            return false;
        }
        if self.group_count.is_empty() {
            return false;
        }
        if !self
            .excluded_students
            .iter()
            .all(|x| student_ids.contains(x))
        {
            return false;
        }
        true
    }
}

impl From<GroupListParameters> for GroupListParametersExternalData {
    fn from(value: GroupListParameters) -> Self {
        GroupListParametersExternalData {
            name: value.name,
            students_per_group: value.students_per_group,
            group_count: value.group_count,
            excluded_students: value
                .excluded_students
                .into_iter()
                .map(|x| x.inner())
                .collect(),
        }
    }
}
