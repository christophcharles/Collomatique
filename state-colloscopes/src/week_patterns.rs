//! Week patterns submodule
//!
//! This module defines the relevant types to describes the week patterns

use std::collections::BTreeMap;

use crate::ids::WeekPatternId;

/// Description of the week patterns
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WeekPatterns {
    /// Week patterns
    ///
    /// Each item associate to a single ID a sequence of weeks
    pub week_pattern_map: BTreeMap<WeekPatternId, WeekPattern>,
}

/// Description of a week pattern
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeekPattern {
    /// Name of the week pattern for identification
    pub name: String,
    /// Weeks the interrogation happens on
    ///
    /// If the Vec is shorter than the total amount of weeks
    /// it is assumed the interrogation happens on all the
    /// remaining weeks.
    ///
    /// If the Vec is longer, the extra weeks are ignored
    /// They are kept in case some one expands again the number of weeks.
    pub weeks: Vec<bool>,
}

/// Description of week patterns but unchecked
///
/// This structure is an unchecked equivalent of [WeekPatterns].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WeekPatternsExternalData {
    /// Week patterns
    ///
    /// Each item associate to a single ID a sequence of weeks
    pub week_pattern_map: BTreeMap<u64, WeekPattern>,
}

impl WeekPatterns {
    /// Builds week patterns from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    pub(crate) unsafe fn from_external_data(
        external_data: WeekPatternsExternalData,
    ) -> WeekPatterns {
        WeekPatterns {
            week_pattern_map: external_data
                .week_pattern_map
                .into_iter()
                .map(|(id, week_pattern)| (unsafe { WeekPatternId::new(id) }, week_pattern))
                .collect(),
        }
    }
}
