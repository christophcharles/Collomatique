//! Group lists submodule
//!
//! This module defines the relevant types to describes the lists of groups

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use std::ops::RangeInclusive;

use crate::ids::{
    ColloscopeGroupListId, ColloscopePeriodId, ColloscopeStudentId, ColloscopeSubjectId, Id,
};

/// Description of the group lists
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupLists<GroupListId: Id, PeriodId: Id, SubjectId: Id, StudentId: Id> {
    /// Group lists
    ///
    /// Each item associates a group list id to an actual group list
    pub group_list_map: BTreeMap<GroupListId, GroupList<StudentId>>,

    /// Associations between subjects and group lists
    ///
    /// If a subject does not appear no group list has been associated to it
    pub subjects_associations: BTreeMap<PeriodId, BTreeMap<SubjectId, GroupListId>>,
}

impl<GroupListId: Id, PeriodId: Id, SubjectId: Id, StudentId: Id> Default
    for GroupLists<GroupListId, PeriodId, SubjectId, StudentId>
{
    fn default() -> Self {
        GroupLists {
            group_list_map: BTreeMap::new(),
            subjects_associations: BTreeMap::new(),
        }
    }
}

/// Description of a single group list
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupList<StudentId: Id> {
    /// parameters for the group list
    pub params: GroupListParameters<StudentId>,
    /// Prefilled groups
    pub prefilled_groups: GroupListPrefilledGroups<StudentId>,
}

/// Prefilled groups for a single group list
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupListPrefilledGroups<StudentId: Id> {
    /// group list
    pub groups: Vec<PrefilledGroup<StudentId>>,
}

impl<StudentId: Id> Default for GroupListPrefilledGroups<StudentId> {
    fn default() -> Self {
        GroupListPrefilledGroups { groups: vec![] }
    }
}

/// Prefilled groups for a single group list
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrefilledGroup<StudentId: Id> {
    /// Optional name for the group
    pub name: Option<non_empty_string::NonEmptyString>,
    /// Students set
    ///
    /// Set of students that are in the group
    pub students: BTreeSet<StudentId>,
    /// Sealed switch
    ///
    /// If `true`, the group is sealed
    /// and no other students should be added.
    ///
    /// This can also be used to force a group to be empty
    pub sealed: bool,
}

impl<StudentId: Id> Default for PrefilledGroup<StudentId> {
    fn default() -> Self {
        PrefilledGroup {
            name: None,
            students: BTreeSet::new(),
            sealed: false,
        }
    }
}

impl<StudentId: Id> GroupListPrefilledGroups<StudentId> {
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
        for group in &self.groups {
            if group.students.contains(&student_id) {
                return true;
            }
        }
        false
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}

impl<StudentId: Id> PrefilledGroup<StudentId> {
    /// Builds a single prefilled group from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [PrefilledGroupExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: PrefilledGroupExternalData) -> Self {
        PrefilledGroup {
            name: external_data.name,
            students: external_data
                .students
                .into_iter()
                .map(|x| unsafe { StudentId::new(x) })
                .collect(),
            sealed: external_data.sealed,
        }
    }

    pub(crate) fn duplicate_with_id_maps(
        &self,
        students_map: &BTreeMap<StudentId, ColloscopeStudentId>,
    ) -> Option<PrefilledGroup<ColloscopeStudentId>> {
        let mut students = BTreeSet::new();

        for student_id in &self.students {
            let new_id = students_map.get(student_id)?;
            students.insert(*new_id);
        }

        Some(PrefilledGroup {
            name: self.name.clone(),
            students,
            sealed: self.sealed,
        })
    }
}

impl<StudentId: Id> GroupListPrefilledGroups<StudentId> {
    /// Builds prefilled groups from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [GroupListPrefilledGroupsExternalData::validate].
    pub(crate) unsafe fn from_external_data(
        external_data: GroupListPrefilledGroupsExternalData,
    ) -> Self {
        GroupListPrefilledGroups {
            groups: external_data
                .groups
                .into_iter()
                .map(|g| unsafe { PrefilledGroup::from_external_data(g) })
                .collect(),
        }
    }

    pub(crate) fn duplicate_with_id_maps(
        &self,
        students_map: &BTreeMap<StudentId, ColloscopeStudentId>,
    ) -> Option<GroupListPrefilledGroups<ColloscopeStudentId>> {
        let mut groups = vec![];

        for group in &self.groups {
            let new_group = group.duplicate_with_id_maps(students_map)?;
            groups.push(new_group);
        }

        Some(GroupListPrefilledGroups { groups })
    }
}

/// Parameters for a single group list
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupListParameters<StudentId: Id> {
    /// Name for the list
    pub name: String,
    /// Range of possible count of students per group
    pub students_per_group: RangeInclusive<NonZeroU32>,
    /// Range of possible number of groups in the list
    pub group_count: RangeInclusive<u32>,
    /// Students set that are not covered by the group list
    pub excluded_students: BTreeSet<StudentId>,
}

impl<StudentId: Id> GroupListParameters<StudentId> {
    /// Builds an interrogation slot from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [GroupListsExternalData::validate_all], [GroupListExternalData::validate] and
    /// [GroupListParametersExternalData::validate].
    pub(crate) unsafe fn from_external_data(
        external_data: GroupListParametersExternalData,
    ) -> Self {
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

    pub(crate) fn duplicate_with_id_maps(
        &self,
        students_map: &BTreeMap<StudentId, ColloscopeStudentId>,
    ) -> Option<GroupListParameters<ColloscopeStudentId>> {
        let mut excluded_students = BTreeSet::new();

        for student_id in &self.excluded_students {
            let new_id = students_map.get(student_id)?;
            excluded_students.insert(*new_id);
        }

        Some(GroupListParameters {
            name: self.name.clone(),
            students_per_group: self.students_per_group.clone(),
            group_count: self.group_count.clone(),
            excluded_students,
        })
    }
}

impl<StudentId: Id> GroupList<StudentId> {
    /// Checks whether the group list is sealed
    ///
    /// This means each prefilled group is sealed and there is no room for another
    /// group
    pub fn is_sealed(&self) -> bool {
        if !self
            .prefilled_groups
            .groups
            .iter()
            .all(|group| group.sealed)
        {
            return false;
        }
        let max_group_count = *self.params.group_count.end();
        self.prefilled_groups.groups.len() != (max_group_count as usize)
    }

    /// Returns the set of students that are not already in a prefilled group
    pub fn remaining_students_to_dispatch(
        &self,
        students: &BTreeSet<StudentId>,
    ) -> BTreeSet<StudentId> {
        let mut output: BTreeSet<_> = students
            .difference(&self.params.excluded_students)
            .copied()
            .collect();

        for group in &self.prefilled_groups.groups {
            for student_id in &group.students {
                output.remove(student_id);
            }
        }

        output
    }
}

impl<StudentId: Id> GroupList<StudentId> {
    /// Builds an interrogation slot from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [GroupListsExternalData::validate_all], [GroupListExternalData::validate] and
    /// [GroupListParametersExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: GroupListExternalData) -> Self {
        GroupList {
            params: unsafe { GroupListParameters::from_external_data(external_data.params) },
            prefilled_groups: unsafe {
                GroupListPrefilledGroups::from_external_data(external_data.prefilled_groups)
            },
        }
    }

    pub(crate) fn duplicate_with_id_maps(
        &self,
        students_map: &BTreeMap<StudentId, ColloscopeStudentId>,
    ) -> Option<GroupList<ColloscopeStudentId>> {
        Some(GroupList {
            params: self.params.duplicate_with_id_maps(students_map)?,
            prefilled_groups: self.prefilled_groups.duplicate_with_id_maps(students_map)?,
        })
    }
}

impl<GroupListId: Id, PeriodId: Id, SubjectId: Id, StudentId: Id>
    GroupLists<GroupListId, PeriodId, SubjectId, StudentId>
{
    /// Builds interrogation slots from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [GroupListsExternalData::validate_all], [GroupListExternalData::validate] and
    /// [GroupListParametersExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: GroupListsExternalData) -> Self {
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

    pub(crate) fn duplicate_with_id_maps(
        &self,
        group_lists_map: &BTreeMap<GroupListId, ColloscopeGroupListId>,
        periods_map: &BTreeMap<PeriodId, ColloscopePeriodId>,
        subjects_map: &BTreeMap<SubjectId, ColloscopeSubjectId>,
        students_map: &BTreeMap<StudentId, ColloscopeStudentId>,
    ) -> Option<
        GroupLists<
            ColloscopeGroupListId,
            ColloscopePeriodId,
            ColloscopeSubjectId,
            ColloscopeStudentId,
        >,
    > {
        let mut group_list_map = BTreeMap::new();

        for (group_list_id, group_list) in &self.group_list_map {
            let new_id = group_lists_map.get(group_list_id)?;
            let new_group_list = group_list.duplicate_with_id_maps(students_map)?;
            group_list_map.insert(*new_id, new_group_list);
        }

        let mut subjects_associations = BTreeMap::new();

        for (period_id, subject_map) in &self.subjects_associations {
            let new_period_id = periods_map.get(period_id)?;
            let mut new_subject_map = BTreeMap::new();

            for (subject_id, group_list_id) in subject_map {
                let new_subject_id = subjects_map.get(subject_id)?;
                let new_group_list_id = group_lists_map.get(group_list_id)?;
                new_subject_map.insert(*new_subject_id, *new_group_list_id);
            }

            subjects_associations.insert(*new_period_id, new_subject_map);
        }

        Some(GroupLists {
            group_list_map,
            subjects_associations,
        })
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

/// Prefilled groups for a single group list but unchecked
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupListPrefilledGroupsExternalData {
    /// group list
    pub groups: Vec<PrefilledGroupExternalData>,
}

/// A single prefilled group but unchecked
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PrefilledGroupExternalData {
    /// Optional name for the group
    pub name: Option<non_empty_string::NonEmptyString>,
    /// Students set
    ///
    /// Set of students that are in the group
    pub students: BTreeSet<u64>,
    /// Sealed switch
    ///
    /// If `true`, the group is sealed
    /// and no other students should be added.
    ///
    /// This can also be used to force a group to be empty
    pub sealed: bool,
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

impl<StudentId: Id> From<GroupList<StudentId>> for GroupListExternalData {
    fn from(value: GroupList<StudentId>) -> Self {
        GroupListExternalData {
            params: value.params.into(),
            prefilled_groups: value.prefilled_groups.into(),
        }
    }
}

impl PrefilledGroupExternalData {
    /// Checks the validity of a [PrefilledGroupExternalData]
    pub fn validate(&self, student_ids: &BTreeSet<u64>, excluded_students: &BTreeSet<u64>) -> bool {
        self.students.iter().all(|id| {
            if !student_ids.contains(id) {
                return false;
            }
            if excluded_students.contains(id) {
                return false;
            }
            true
        })
    }
}

impl<StudentId: Id> From<PrefilledGroup<StudentId>> for PrefilledGroupExternalData {
    fn from(value: PrefilledGroup<StudentId>) -> Self {
        PrefilledGroupExternalData {
            name: value.name,
            students: value.students.into_iter().map(|x| x.inner()).collect(),
            sealed: value.sealed,
        }
    }
}

impl GroupListPrefilledGroupsExternalData {
    /// Checks the validity of a [GroupListPrefilledGroupsExternalData]
    pub fn validate(&self, student_ids: &BTreeSet<u64>, excluded_students: &BTreeSet<u64>) -> bool {
        let mut students_so_far = BTreeSet::new();
        for group in &self.groups {
            if !group.validate(student_ids, excluded_students) {
                return false;
            }
            for student in &group.students {
                if !students_so_far.insert(*student) {
                    return false;
                }
            }
        }
        true
    }
}

impl<StudentId: Id> From<GroupListPrefilledGroups<StudentId>>
    for GroupListPrefilledGroupsExternalData
{
    fn from(value: GroupListPrefilledGroups<StudentId>) -> Self {
        GroupListPrefilledGroupsExternalData {
            groups: value.groups.into_iter().map(|g| g.into()).collect(),
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

impl<StudentId: Id> From<GroupListParameters<StudentId>> for GroupListParametersExternalData {
    fn from(value: GroupListParameters<StudentId>) -> Self {
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
