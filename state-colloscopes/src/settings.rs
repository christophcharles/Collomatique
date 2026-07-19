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

impl Settings {
    /// Return the effective [`Limits`] for a student.
    ///
    /// A per-student override entry wins **verbatim** (whole-entry): if the
    /// student has an entry in [`Settings::students`], that entry is returned as
    /// is — a `None` field disables the corresponding global limit. Otherwise the
    /// [`Settings::global`] limits apply.
    pub fn limits_for(&self, student: StudentId) -> &Limits {
        self.students.get(&student).unwrap_or(&self.global)
    }
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

/// Precondition errors of the forced settings op — the carve-out subset
/// (step-3 survey Table 2). The settings op has no transition/input guards
/// (only `validate_settings`, which strips), so this enum is empty; kept for
/// uniformity across the [crate::PrecheckError] family.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SettingsPrecheckError {}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::Id;

    fn soft(value: u32) -> Option<SoftParam<u32>> {
        Some(SoftParam { soft: true, value })
    }

    #[test]
    fn limits_for_falls_back_to_global_without_override() {
        let mut settings = Settings::default();
        settings.global.interrogations_per_week_max = soft(3);

        let student = unsafe { StudentId::new(1) };
        assert_eq!(settings.limits_for(student), &settings.global);
        assert_eq!(
            settings.limits_for(student).interrogations_per_week_max,
            soft(3)
        );
    }

    #[test]
    fn limits_for_returns_override_entry_verbatim() {
        let mut settings = Settings::default();
        settings.global.interrogations_per_week_max = soft(3);

        // A whole-entry override with the weekly-max field `None` must win
        // verbatim — it disables the global limit rather than inheriting it.
        let student = unsafe { StudentId::new(1) };
        let override_limits = Limits {
            interrogations_per_week_max: None,
            ..Default::default()
        };
        settings.students.insert(student, override_limits.clone());

        assert_eq!(settings.limits_for(student), &override_limits);
        assert_eq!(
            settings.limits_for(student).interrogations_per_week_max,
            None
        );
    }
}
