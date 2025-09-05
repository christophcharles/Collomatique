//! group list submodule
//!
//! This module contains the code for decoding GroupListList entries

use super::*;

pub fn decode_entry(
    group_lists: json::group_list_list::List,
    pre_data: &mut PreData,
) -> Result<(), DecodeError> {
    if !pre_data.main_params.group_lists.group_list_map.is_empty() {
        return Err(DecodeError::GroupListsAlreadyDecoded);
    }

    let mut ids = BTreeSet::new();

    for (group_list_id, group_list) in group_lists.group_list_map {
        if !ids.insert(group_list_id) {
            return Err(DecodeError::DuplicatedID);
        }
        let pre_group_list = group_list.into();
        pre_data
            .main_params
            .group_lists
            .group_list_map
            .insert(group_list_id, pre_group_list);
    }

    for (period_id, subject_map) in group_lists.subjects_associations {
        let Some(pre_subject_map) = pre_data
            .main_params
            .group_lists
            .subjects_associations
            .get_mut(&period_id)
        else {
            return Err(DecodeError::InvalidId);
        };
        *pre_subject_map = subject_map;
    }

    Ok(())
}
