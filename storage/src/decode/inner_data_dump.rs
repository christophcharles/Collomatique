//! Assignment map submodule
//!
//! This module contains the code for decoding InnerDataDump entries

use super::*;

pub fn decode_entry(
    inner_data: collomatique_state_colloscopes::InnerData,
    pre_data: &mut PreData,
) -> Result<(), DecodeError> {
    let default_inner_data = collomatique_state_colloscopes::InnerData::default();
    if pre_data.inner_data != default_inner_data {
        return Err(DecodeError::InnerDataDumpUsedOnModifiedInnerData);
    }

    pre_data.inner_data = inner_data;

    Ok(())
}
