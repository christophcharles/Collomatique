//! group list submodule
//!
//! This module contains the code for decoding GroupListList entries

use super::*;

pub fn decode_entry(
    group_lists: json::group_list_list::List,
    pre_data: &mut PreData,
) -> Result<(), DecodeError> {
    if !pre_data.group_lists.group_list_map.is_empty() {
        return Err(DecodeError::GroupListsAlreadyDecoded);
    }
    if !pre_data.group_lists.subjects_associations.is_empty() {
        return Err(DecodeError::GroupListsAlreadyDecoded);
    }

    let mut ids = BTreeSet::new();

    for (group_list_id, group_list) in group_lists.group_list_map {
        if !ids.insert(group_list_id) {
            return Err(DecodeError::DuplicatedID);
        }
        let pre_group_list = group_list.into();
        pre_data
            .group_lists
            .group_list_map
            .insert(group_list_id, pre_group_list);
    }

    pre_data.group_lists.subjects_associations = group_lists.subjects_associations;

    Ok(())
}
