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
    /// is — an override can for example harden a rotation that is globally soft,
    /// or vice versa. Otherwise the [`Balancing::global`] options apply.
    pub fn options_for(&self, subject: SubjectId) -> &BalancingOptions {
        self.subjects.get(&subject).unwrap_or(&self.global)
    }
}

// The container's half of the dense renumbering walk (see [crate::compact]).
// Only the per-subject override keys are ids; the option values are not.
impl Balancing {
    pub(crate) fn collect_ids(&self, ids: &mut std::collections::BTreeSet<u64>) {
        use crate::ids::Id as _;
        for subject_id in self.subjects.keys() {
            ids.insert(subject_id.inner());
        }
    }

    pub(crate) fn remap_ids(self, map: &crate::compact::IdMap) -> Self {
        use crate::compact::remap;
        Balancing {
            global: self.global,
            subjects: self
                .subjects
                .into_iter()
                .map(|(subject_id, options)| (remap(map, subject_id), options))
                .collect(),
        }
    }
}

/// Options for balancing interrogations
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalancingOptions {
    /// Teacher rotation across groups: `None` means the goal is not pursued at
    /// all (no constraint and no objective term), `Some { soft: true }` makes it
    /// a soft optimisation goal and `Some { soft: false }` a strict constraint.
    pub teacher_rotation: Option<SoftParam<()>>,
    /// Slot rotation across groups, with the same three states as
    /// [`Self::teacher_rotation`].
    pub slot_rotation: Option<SoftParam<()>>,
    /// Avoiding the same teacher twice in a row for a group, with the same three
    /// states as [`Self::teacher_rotation`].
    pub avoid_twice_in_a_row: Option<SoftParam<()>>,
    /// Whether to enforce fair teacher distribution over the entire year
    pub year_teacher_rotation: bool,
    /// Whether to enforce fair teacher distribution within each period
    pub period_teacher_rotation: bool,
}

// A whole-entry override record, exactly like [crate::settings::Limits]: a
// bundle of independent boolean choices with no natural partial order between
// two records — so the document order treats the whole record as one atom
// (plan step 6.5, decision 13).
collomatique_state::impl_content_ord_atom!(BalancingOptions);

impl Default for BalancingOptions {
    fn default() -> Self {
        Self {
            teacher_rotation: Some(SoftParam {
                soft: true,
                value: (),
            }),
            slot_rotation: None,
            avoid_twice_in_a_row: None,
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
        assert_eq!(
            balancing.options_for(subject).teacher_rotation,
            Some(SoftParam {
                soft: true,
                value: ()
            })
        );
    }

    #[test]
    fn options_for_returns_override_entry_verbatim() {
        let mut balancing = Balancing::default();
        assert_eq!(
            balancing.global.teacher_rotation,
            Some(SoftParam {
                soft: true,
                value: ()
            })
        );

        // A whole-entry override must win verbatim — here it hardens the teacher
        // rotation that is soft in the global options.
        let subject = unsafe { SubjectId::new(1) };
        let override_options = BalancingOptions {
            teacher_rotation: Some(SoftParam {
                soft: false,
                value: (),
            }),
            ..Default::default()
        };
        balancing.subjects.insert(subject, override_options.clone());

        assert_eq!(balancing.options_for(subject), &override_options);
        assert_eq!(
            balancing.options_for(subject).teacher_rotation,
            Some(SoftParam {
                soft: false,
                value: ()
            })
        );
    }
}
