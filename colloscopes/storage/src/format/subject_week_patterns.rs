//! The `SubjectWeekPatterns` block (spec §4.17)

use serde::{Deserialize, Serialize};

use super::keyed::{KeyedRow, KeyedVec};

/// Which week pattern restricts a subject's active weeks, keyed by
/// `subject_id`
///
/// Sparse: only the subjects carrying a pattern have a row. The key set is
/// free: every present row carries real state (`week_pattern_id`), so there
/// is no neutral-content rule here.
///
/// Default: no associations.
pub type SubjectWeekPatterns = KeyedVec<SubjectWeekPattern>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectWeekPattern {
    pub subject_id: u64,
    pub week_pattern_id: u64,
}

impl KeyedRow for SubjectWeekPattern {
    type Key = u64;

    fn key(&self) -> u64 {
        self.subject_id
    }
}

#[cfg(test)]
mod tests;
