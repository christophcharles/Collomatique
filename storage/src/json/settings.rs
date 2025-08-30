//! settings submodule
//!
//! This module defines the settings entry for the JSON description
//!

use super::*;

use std::num::NonZeroU32;

/// JSON desc of settings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Strict limits to impose during resolution
    pub strict_limits: StrictLimits,
}

/// JSON desc of strict limits
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrictLimits {
    /// Number of interrogations for each student per week
    pub interrogations_per_week: Option<std::ops::Range<u32>>,
    /// maximum number of interrogation in a single day for each student
    pub max_interrogations_per_day: Option<NonZeroU32>,
}

impl From<&collomatique_state_colloscopes::settings::StrictLimits> for StrictLimits {
    fn from(value: &collomatique_state_colloscopes::settings::StrictLimits) -> Self {
        StrictLimits {
            interrogations_per_week: value.interrogations_per_week.clone(),
            max_interrogations_per_day: value.max_interrogations_per_day,
        }
    }
}

impl From<StrictLimits> for collomatique_state_colloscopes::settings::StrictLimits {
    fn from(value: StrictLimits) -> Self {
        collomatique_state_colloscopes::settings::StrictLimits {
            interrogations_per_week: value.interrogations_per_week,
            max_interrogations_per_day: value.max_interrogations_per_day,
        }
    }
}

impl From<&collomatique_state_colloscopes::settings::GeneralSettings> for Settings {
    fn from(value: &collomatique_state_colloscopes::settings::GeneralSettings) -> Self {
        Settings {
            strict_limits: (&value.strict_limits).into(),
        }
    }
}

impl From<Settings> for collomatique_state_colloscopes::settings::GeneralSettings {
    fn from(value: Settings) -> Self {
        collomatique_state_colloscopes::settings::GeneralSettings {
            strict_limits: value.strict_limits.into(),
        }
    }
}
