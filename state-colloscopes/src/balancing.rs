//! Balancing submodule
//!
//! This module defines the relevant types to describe balancing requirements
//! for interrogation scheduling (teacher rotation, avoiding same teacher twice in a row).

use crate::Table;
use crate::ids::SubjectId;
use crate::ops::AnnotatedBalancingOp;
use crate::soft_param::SoftParam;
use collomatique_state::ContentOrd;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Description of the balancing configuration
///
/// Contains global balancing options and optional per-subject overrides.
#[derive(Clone, Debug, PartialEq, Eq, ContentOrd)]
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

// A whole-entry override record, exactly like [crate::settings::Limits]: a
// `None` field means "disabled" — an active choice, not absent content — so
// the document order treats the whole record as one atom (plan step 6.5,
// decision 13).
collomatique_state::impl_content_ord_atom!(BalancingOptions);

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

/// Precondition errors of the forced balancing op — the carve-out subset
/// (step-3 survey Table 2). The whole-value `Update` had no
/// transition/input guards at all (only `validate_balancing`, which strips);
/// the targeted [crate::BalancingOp::SetSubject] adds the coordinate
/// carve-out its key needs.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum BalancingPrecheckError {
    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),
}

impl crate::Data {
    /// Used internally by [crate::Data::force_apply]
    ///
    /// Force-applies a balancing op: `validate_balancing` is an invariant guard
    /// and is stripped (step-3 survey Table 1); what remains is the
    /// coordinate-existence check on the per-subject key, checked uniformly
    /// whether the override is being set or cleared — the same choice
    /// `force_apply_assignment`'s `SetRow` makes. May leave the state invalid;
    /// the caller owns checking and rollback.
    pub(crate) fn force_apply_balancing(
        &mut self,
        balancing_op: &AnnotatedBalancingOp,
    ) -> std::result::Result<AnnotatedBalancingOp, BalancingPrecheckError> {
        match balancing_op {
            AnnotatedBalancingOp::SetGlobal(new_options) => {
                // stripped: validate_balancing
                let old_options = std::mem::replace(
                    &mut self.inner_data.params.balancing.global,
                    new_options.clone(),
                );
                Ok(AnnotatedBalancingOp::SetGlobal(old_options))
            }
            AnnotatedBalancingOp::SetSubject(subject_id, new_options) => {
                if self
                    .inner_data
                    .params
                    .subjects
                    .find_subject(*subject_id)
                    .is_none()
                {
                    return Err(BalancingPrecheckError::InvalidSubjectId(*subject_id));
                }

                // Sparse canonical form: a subject has an entry iff an
                // override was set for it. `None` removes the entry.
                let subjects = &mut self.inner_data.params.balancing.subjects;
                let old_options = match new_options {
                    Some(options) => subjects.insert(*subject_id, options.clone()),
                    None => subjects.remove(subject_id),
                };

                Ok(AnnotatedBalancingOp::SetSubject(*subject_id, old_options))
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
