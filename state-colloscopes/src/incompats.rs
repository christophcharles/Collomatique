//! Incompats submodule
//!
//! This module defines the relevant types to describes the schedule incompatibilities

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use crate::ids::{IncompatId, SubjectId, WeekPatternId};

/// Description of the schedule incompatibilities
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Incompats {
    /// Incompats for subjects
    ///
    /// Each item associates an incompat id to a schedule incompatibility
    pub incompat_map: BTreeMap<IncompatId, Incompatibility>,
}

/// Description of a single schedule incompat
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Incompatibility {
    /// Subject the incompatibility is linked to
    pub subject_id: SubjectId,
    /// Name of the incompatibility for clarity
    pub name: String,
    /// Slots of time when the students might not be available
    ///
    /// This is given as a weekday, a start time and a duration
    pub slots: Vec<collomatique_time::SlotWithDuration>,
    /// Number of slots to force to be free in the above list
    pub minimum_free_slots: NonZeroU32,
    /// Week pattern for the incompatibility
    ///
    /// If `None`, this means every week
    pub week_pattern_id: Option<WeekPatternId>,
}
