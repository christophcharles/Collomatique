//! group list submodule
//!
//! This module defines the group list entry for the JSON description
//!
use super::*;

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

/// JSON desc of group lists
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct List {
    /// map between group list ids and corresponding group lists
    #[serde(with = "serde_with::rust::maps_duplicate_key_is_error")]
    pub group_list_map: BTreeMap<u64, GroupList>,
    /// Associations between subjects and group lists
    #[serde(with = "serde_with::rust::maps_duplicate_key_is_error")]
    pub subjects_associations: BTreeMap<u64, u64>,
}

/// JSON desc of a single group list
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupList {
    pub params: GroupListParameters,
    pub prefilled_groups: GroupListPrefilledGroups,
}

/// JSON desc of a single group list prefilled groups
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupListPrefilledGroups {
    #[serde(with = "serde_with::rust::maps_duplicate_key_is_error")]
    pub student_map: BTreeMap<u64, u32>,
}

impl From<&collomatique_state_colloscopes::group_lists::GroupListPrefilledGroups>
    for GroupListPrefilledGroups
{
    fn from(value: &collomatique_state_colloscopes::group_lists::GroupListPrefilledGroups) -> Self {
        GroupListPrefilledGroups {
            student_map: value
                .student_map
                .iter()
                .map(|(student_id, group_num)| (student_id.inner(), *group_num))
                .collect(),
        }
    }
}

impl From<GroupListPrefilledGroups>
    for collomatique_state_colloscopes::group_lists::GroupListPrefilledGroupsExternalData
{
    fn from(value: GroupListPrefilledGroups) -> Self {
        collomatique_state_colloscopes::group_lists::GroupListPrefilledGroupsExternalData {
            student_map: value.student_map.clone(),
        }
    }
}

/// JSON desc of a single group list parameters
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupListParameters {
    pub name: String,
    pub students_per_group: std::ops::RangeInclusive<NonZeroU32>,
    pub group_count: std::ops::RangeInclusive<u32>,
    pub excluded_students: BTreeSet<u64>,
}

impl From<&collomatique_state_colloscopes::group_lists::GroupListParameters>
    for GroupListParameters
{
    fn from(value: &collomatique_state_colloscopes::group_lists::GroupListParameters) -> Self {
        GroupListParameters {
            name: value.name.clone(),
            students_per_group: value.students_per_group.clone(),
            group_count: value.group_count.clone(),
            excluded_students: value
                .excluded_students
                .iter()
                .map(|id| id.inner())
                .collect(),
        }
    }
}

impl From<GroupListParameters>
    for collomatique_state_colloscopes::group_lists::GroupListParametersExternalData
{
    fn from(value: GroupListParameters) -> Self {
        collomatique_state_colloscopes::group_lists::GroupListParametersExternalData {
            name: value.name,
            students_per_group: value.students_per_group,
            group_count: value.group_count,
            excluded_students: value.excluded_students,
        }
    }
}

impl From<&collomatique_state_colloscopes::group_lists::GroupList> for GroupList {
    fn from(value: &collomatique_state_colloscopes::group_lists::GroupList) -> Self {
        GroupList {
            params: (&value.params).into(),
            prefilled_groups: (&value.prefilled_groups).into(),
        }
    }
}

impl From<GroupList> for collomatique_state_colloscopes::group_lists::GroupListExternalData {
    fn from(value: GroupList) -> Self {
        collomatique_state_colloscopes::group_lists::GroupListExternalData {
            params: value.params.into(),
            prefilled_groups: value.prefilled_groups.into(),
        }
    }
}
