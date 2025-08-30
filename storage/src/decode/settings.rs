//! Settings submodule
//!
//! This module contains the code for decoding Settings entries

use super::*;

pub fn decode_entry(
    settings: json::settings::Settings,
    pre_data: &mut PreData,
) -> Result<(), DecodeError> {
    pre_data.settings = settings.into();

    Ok(())
}
