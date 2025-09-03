//! General settings submodule
//!
//! This module defines the relevant types to describes general settings

use std::num::NonZeroU32;

/// Description of the general settings
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct GeneralSettings {
    /// Strict limits to impose during resolution
    pub strict_limits: StrictLimits,
}

/// Strict limits in resolution
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StrictLimits {
    /// Number of interrogations for each student per week
    pub interrogations_per_week: Option<std::ops::RangeInclusive<u32>>,
    /// maximum number of interrogation in a single day for each student
    pub max_interrogations_per_day: Option<NonZeroU32>,
}
