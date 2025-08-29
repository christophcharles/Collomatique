//! rule submodule
//!
//! This module contains the code for decoding RuleList entries

use super::*;

pub fn decode_entry(
    rule_list: json::rule_list::List,
    pre_data: &mut PreData,
) -> Result<(), DecodeError> {
    if !pre_data.rules.rule_map.is_empty() {
        return Err(DecodeError::RulesAlreadyDecoded);
    }

    for (rule_id, rule) in rule_list.rule_map {
        let pre_rule = rule.into();
        pre_data.rules.rule_map.insert(rule_id, pre_rule);
    }

    Ok(())
}
