//! Incompat list submodule
//!
//! This module contains the code for decoding IncompatList entries

use super::*;

pub fn decode_entry(
    incompat_list: json::incompat_list::List,
    pre_data: &mut PreData,
) -> Result<(), DecodeError> {
    let mut ids = BTreeSet::new();

    for (incompat_id, incompat) in &incompat_list.incompat_map {
        let pre_incompat = match incompat.clone().try_into() {
            Ok(i) => i,
            Err(json::incompat_list::IncompatDecodeError::SlotOverlapsWithNextDay) => {
                return Err(DecodeError::IllformedSlotInSubjectIncompatibilities);
            }
        };

        if !ids.insert(*incompat_id) {
            return Err(DecodeError::DuplicatedID);
        }
        pre_data
            .incompats
            .incompat_map
            .insert(*incompat_id, pre_incompat);
    }

    Ok(())
}
