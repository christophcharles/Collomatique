//! General settings submodule
//!
//! This module defines the relevant types to describes general settings

use crate::Table;
use crate::ids::StudentId;
use crate::ops::AnnotatedSettingsOp;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// Re-export for backward compatibility
pub use crate::soft_param::SoftParam;

/// Description of the general settings
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    /// Global limits to impose during resolution
    pub global: Limits,
    /// Optional limits per students
    pub students: Table<StudentId, Limits>,
}

/// Strict limits in resolution
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    /// Number of interrogations for each student per week
    pub interrogations_per_week_min: Option<SoftParam<u32>>,
    /// Number of interrogations for each student per week
    pub interrogations_per_week_max: Option<SoftParam<u32>>,
    /// maximum number of interrogation in a single day for each student
    pub max_interrogations_per_day: Option<SoftParam<NonZeroU32>>,
}

/// Errors for settings operations
///
/// These errors can be returned when trying to modify [crate::Data] with a settings op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SettingsError {
    /// student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),
}

impl crate::Data {
    /// Used internally
    ///
    /// Apply settings operations
    pub(crate) fn apply_settings(
        &mut self,
        settings_op: &AnnotatedSettingsOp,
    ) -> std::result::Result<AnnotatedSettingsOp, SettingsError> {
        match settings_op {
            AnnotatedSettingsOp::Update(new_settings) => {
                self.inner_data.params.validate_settings(new_settings)?;
                let old_settings =
                    std::mem::replace(&mut self.inner_data.params.settings, new_settings.clone());
                Ok(AnnotatedSettingsOp::Update(old_settings))
            }
        }
    }
}
