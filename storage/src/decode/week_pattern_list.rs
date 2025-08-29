//! Week pattern list submodule
//!
//! This module contains the code for decoding WeekPatternList entries

use super::*;

pub fn decode_entry(
    week_pattern_list: json::week_pattern_list::List,
    pre_data: &mut PreData,
) -> Result<(), DecodeError> {
    if !pre_data.week_patterns.week_pattern_map.is_empty() {
        return Err(DecodeError::WeekPatternsAlreadyDecoded);
    }

    let mut ids = BTreeSet::new();
    for (id, week_pattern) in week_pattern_list.week_pattern_map {
        if !ids.insert(id) {
            return Err(DecodeError::DuplicatedID);
        }
        pre_data
            .week_patterns
            .week_pattern_map
            .insert(id, week_pattern.into());
    }

    Ok(())
}
