use collomatique_state_colloscopes::{
    PromoteGroupListParametersError, PromoteGroupListPrefilledGroupsError,
};

use crate::rpc::error_msg::group_lists::GroupListsError;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupListsCmdMsg {
    AddNewGroupList(GroupListParametersMsg),
    UpdateGroupList(MsgGroupListId, GroupListParametersMsg),
    DeleteGroupList(MsgGroupListId),
    PrefillGroupList(MsgGroupListId, GroupListPrefilledGroupsMsg),
    AssignGroupListToSubject(MsgPeriodId, MsgSubjectId, Option<MsgGroupListId>),
}

impl GroupListsCmdMsg {
    pub fn promote(
        self,
        data: &collomatique_state_colloscopes::Data,
    ) -> Result<crate::ops::GroupListsUpdateOp, GroupListsError> {
        use crate::ops::GroupListsUpdateOp;
        Ok(match self {
            GroupListsCmdMsg::AddNewGroupList(params) => {
                let new_params = match data.promote_group_list_params(params.into()) {
                    Ok(p) => p,
                    Err(PromoteGroupListParametersError::InvalidStudentId(id)) => {
                        return Err(error_msg::AddNewGroupListError::InvalidStudentId(
                            MsgStudentId(id),
                        )
                        .into());
                    }
                };
                GroupListsUpdateOp::AddNewGroupList(new_params)
            }
            GroupListsCmdMsg::UpdateGroupList(id, params) => {
                let Some(group_list_id) = data.validate_group_list_id(id.0) else {
                    return Err(error_msg::UpdateGroupListError::InvalidGroupListId(id).into());
                };
                let new_params = match data.promote_group_list_params(params.into()) {
                    Ok(p) => p,
                    Err(PromoteGroupListParametersError::InvalidStudentId(id)) => {
                        return Err(error_msg::AddNewGroupListError::InvalidStudentId(
                            MsgStudentId(id),
                        )
                        .into());
                    }
                };
                GroupListsUpdateOp::UpdateGroupList(group_list_id, new_params)
            }
            GroupListsCmdMsg::DeleteGroupList(id) => {
                let Some(group_list_id) = data.validate_group_list_id(id.0) else {
                    return Err(error_msg::DeleteGroupListError::InvalidGroupListId(id).into());
                };
                GroupListsUpdateOp::DeleteGroupList(group_list_id)
            }
            GroupListsCmdMsg::PrefillGroupList(id, prefilled_groups) => {
                let Some(group_list_id) = data.validate_group_list_id(id.0) else {
                    return Err(error_msg::PrefillGroupListError::InvalidGroupListId(id).into());
                };
                let new_prefilled_groups =
                    match data.promote_group_list_prefilled_groups(prefilled_groups.into()) {
                        Ok(pg) => pg,
                        Err(PromoteGroupListPrefilledGroupsError::InvalidStudentId(id)) => {
                            return Err(error_msg::AddNewGroupListError::InvalidStudentId(
                                MsgStudentId(id),
                            )
                            .into());
                        }
                    };
                GroupListsUpdateOp::PrefillGroupList(group_list_id, new_prefilled_groups)
            }
            GroupListsCmdMsg::AssignGroupListToSubject(period_id, subject_id, group_list_id) => {
                let Some(period_id) = data.validate_period_id(period_id.0) else {
                    return Err(error_msg::AssignGroupListToSubjectError::InvalidPeriodId(
                        period_id,
                    )
                    .into());
                };
                let Some(subject_id) = data.validate_subject_id(subject_id.0) else {
                    return Err(error_msg::AssignGroupListToSubjectError::InvalidSubjectId(
                        subject_id,
                    )
                    .into());
                };
                let group_list_id = match group_list_id {
                    Some(id) => {
                        let Some(new_id) = data.validate_group_list_id(id.0) else {
                            return Err(
                                error_msg::AssignGroupListToSubjectError::InvalidGroupListId(id)
                                    .into(),
                            );
                        };
                        Some(new_id)
                    }
                    None => None,
                };
                GroupListsUpdateOp::AssignGroupListToSubject(period_id, subject_id, group_list_id)
            }
        })
    }
}

use std::collections::BTreeSet;
use std::num::NonZeroU32;
use std::ops::RangeInclusive;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupListParametersMsg {
    pub name: String,
    pub students_per_group: RangeInclusive<NonZeroU32>,
    pub group_count: RangeInclusive<u32>,
    pub excluded_students: BTreeSet<MsgStudentId>,
}

impl From<GroupListParametersMsg>
    for collomatique_state_colloscopes::group_lists::GroupListParametersExternalData
{
    fn from(value: GroupListParametersMsg) -> Self {
        collomatique_state_colloscopes::group_lists::GroupListParametersExternalData {
            name: value.name,
            students_per_group: value.students_per_group,
            group_count: value.group_count,
            excluded_students: value.excluded_students.into_iter().map(|x| x.0).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupListPrefilledGroupsMsg {
    pub groups: Vec<PrefilledGroupMsg>,
}

impl From<GroupListPrefilledGroupsMsg>
    for collomatique_state_colloscopes::group_lists::GroupListPrefilledGroupsExternalData
{
    fn from(value: GroupListPrefilledGroupsMsg) -> Self {
        collomatique_state_colloscopes::group_lists::GroupListPrefilledGroupsExternalData {
            groups: value.groups.into_iter().map(|x| x.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrefilledGroupMsg {
    pub name: Option<non_empty_string::NonEmptyString>,
    pub students: BTreeSet<MsgStudentId>,
    pub sealed: bool,
}

impl From<PrefilledGroupMsg>
    for collomatique_state_colloscopes::group_lists::PrefilledGroupExternalData
{
    fn from(value: PrefilledGroupMsg) -> Self {
        collomatique_state_colloscopes::group_lists::PrefilledGroupExternalData {
            name: value.name,
            students: value.students.into_iter().map(|x| x.0).collect(),
            sealed: value.sealed,
        }
    }
}
