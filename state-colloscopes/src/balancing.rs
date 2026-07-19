//! Balancing submodule
//!
//! This module defines the relevant types to describe balancing requirements
//! for interrogation scheduling (teacher rotation, avoiding same teacher twice in a row).

use crate::Table;
use crate::ids::SubjectId;
use crate::ops::AnnotatedBalancingOp;
use crate::soft_param::SoftParam;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Description of the balancing configuration
///
/// Contains global balancing options and optional per-subject overrides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Balancing {
    /// Global balancing options
    pub global: BalancingOptions,
    /// Optional per-subject overrides
    pub subjects: Table<SubjectId, BalancingOptions>,
}

impl Default for Balancing {
    fn default() -> Self {
        Self {
            global: BalancingOptions::default(),
            subjects: Table::new(),
        }
    }
}

impl Balancing {
    /// Return the effective [`BalancingOptions`] for a subject.
    ///
    /// A per-subject override entry wins **verbatim** (whole-entry): if the
    /// subject has an entry in [`Balancing::subjects`], that entry is returned as
    /// is — a `None` field disables the corresponding global option. Otherwise the
    /// [`Balancing::global`] options apply.
    pub fn options_for(&self, subject: SubjectId) -> &BalancingOptions {
        self.subjects.get(&subject).unwrap_or(&self.global)
    }
}

/// Options for balancing interrogations
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalancingOptions {
    /// Whether to rotate teachers across groups
    pub teacher_rotation: Option<SoftParam<()>>,
    /// Whether to rotate time slots across groups
    pub slot_rotation: Option<SoftParam<()>>,
    /// Whether to avoid having the same teacher twice in a row for a group
    pub avoid_twice_in_a_row: bool,
    /// Whether to enforce fair teacher distribution over the entire year
    pub year_teacher_rotation: bool,
    /// Whether to enforce fair teacher distribution within each period
    pub period_teacher_rotation: bool,
}

impl Default for BalancingOptions {
    fn default() -> Self {
        Self {
            teacher_rotation: Some(SoftParam {
                soft: true,
                value: (),
            }),
            slot_rotation: None,
            avoid_twice_in_a_row: true,
            year_teacher_rotation: false,
            period_teacher_rotation: false,
        }
    }
}

/// Errors for balancing operations
///
/// These errors can be returned when trying to modify [crate::Data] with a balancing op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum BalancingError {
    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),
    /// Subject does not have interrogations
    #[error("subject id ({0:?}) does not have interrogations")]
    SubjectHasNoInterrogation(SubjectId),
}

/// Precondition errors of the forced balancing op — the carve-out subset
/// (step-3 survey Table 2). The balancing op has no transition/input guards
/// (only `validate_balancing`, which strips), so this enum is empty; kept for
/// uniformity across the [crate::PrecheckError] family.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum BalancingPrecheckError {}

impl crate::Data {
    /// Used internally
    ///
    /// Apply balancing operations
    pub(crate) fn apply_balancing(
        &mut self,
        balancing_op: &AnnotatedBalancingOp,
    ) -> std::result::Result<AnnotatedBalancingOp, BalancingError> {
        match balancing_op {
            AnnotatedBalancingOp::Update(new_balancing) => {
                self.inner_data.params.validate_balancing(new_balancing)?;
                let old_balancing =
                    std::mem::replace(&mut self.inner_data.params.balancing, new_balancing.clone());
                Ok(AnnotatedBalancingOp::Update(old_balancing))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::Id;

    #[test]
    fn options_for_falls_back_to_global_without_override() {
        let balancing = Balancing::default();

        let subject = unsafe { SubjectId::new(1) };
        assert_eq!(balancing.options_for(subject), &balancing.global);
        assert!(balancing.options_for(subject).teacher_rotation.is_some());
    }

    #[test]
    fn options_for_returns_override_entry_verbatim() {
        let mut balancing = Balancing::default();
        assert!(balancing.global.teacher_rotation.is_some());

        // A whole-entry override with `teacher_rotation: None` must win verbatim —
        // it disables the global option rather than inheriting it.
        let subject = unsafe { SubjectId::new(1) };
        let override_options = BalancingOptions {
            teacher_rotation: None,
            ..Default::default()
        };
        balancing.subjects.insert(subject, override_options.clone());

        assert_eq!(balancing.options_for(subject), &override_options);
        assert!(balancing.options_for(subject).teacher_rotation.is_none());
    }
}
