//! Week patterns submodule
//!
//! This module defines the relevant types to describes the week patterns

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use collomatique_state::References;

use crate::Table;
use crate::ids::{IncompatId, SlotId, WeekId, WeekPatternId};
use crate::ops::AnnotatedWeekPatternOp;

/// Description of the week patterns
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WeekPatterns {
    /// Week patterns
    ///
    /// Each item associates a single ID with the set of weeks it disables.
    pub week_pattern_map: Table<WeekPatternId, WeekPattern>,
}

/// Description of a week pattern
///
/// A pattern is stored as the *exception set* of the weeks it disables; every
/// week not listed is active. This is the sparse dual of the historical
/// positional bitmask.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References)]
pub struct WeekPattern {
    /// Name of the week pattern for identification
    pub name: String,
    /// Weeks the pattern *disables*. Absent = active (the trivial value).
    ///
    /// May reference non-interrogation weeks: the bit is preserved regardless
    /// of the week's `interrogations` flag (byte-stability, decision 12). The
    /// merged activity of a week is `week.interrogations ∧ ¬excluded`.
    #[fk]
    pub excluded_weeks: BTreeSet<WeekId>,
}

impl WeekPatterns {
    /// The single definition of "a slot can carry an interrogation on `week`":
    /// the week runs interrogations and is not excluded by the given pattern (or
    /// there is no pattern). Homed here so consumers holding only a `Weeks` +
    /// `WeekPatterns` pair — e.g. the gtk4 colloscope grid — can call it;
    /// [`super::colloscope_params::Parameters::is_week_active`] delegates to it.
    ///
    /// Returns `false` for a dangling week id; a dangling pattern id is treated
    /// as "no exclusion". Both are bugs on validated data.
    pub fn is_week_active(
        &self,
        weeks: &super::weeks::Weeks,
        week: WeekId,
        pattern: Option<WeekPatternId>,
    ) -> bool {
        let Some(week_desc) = weeks.find_week(week) else {
            return false;
        };
        week_desc.interrogations
            && pattern.is_none_or(|p| {
                self.week_pattern_map
                    .get(&p)
                    .is_none_or(|wp| !wp.excluded_weeks.contains(&week))
            })
    }
}

/// Errors for week pattern operations
///
/// These errors can be returned when trying to modify [crate::Data] with a week pattern op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum WeekPatternError {
    /// A week pattern id is invalid
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(WeekPatternId),

    /// The week pattern id already exists
    #[error("week pattern id ({0:?}) already exists")]
    WeekPatternIdAlreadyExists(WeekPatternId),

    /// The week pattern is referenced by a slot
    #[error("week pattern id ({0:?}) is referenced by a slot ({1:?})")]
    WeekPatternStillHasAssociatedSlots(WeekPatternId, SlotId),

    /// The week pattern is referenced by a schedule incompatibility
    #[error("week pattern id ({0:?}) is referenced by an incompat ({1:?})")]
    WeekPatternStillHasAssociatedIncompat(WeekPatternId, IncompatId),

    /// The week pattern excludes a week that does not exist
    #[error("week pattern excludes an invalid week ({0:?})")]
    WeekPatternExcludesInvalidWeek(WeekId),

    /// The slot in colloscope is incompatible with the new week pattern
    #[error("slot {0:?} in colloscope is not compatible with the new week pattern")]
    NotCompatibleSlotInColloscope(SlotId),
}

/// Precondition errors of the forced week-pattern ops — the carve-out subset
/// (step-3 survey Table 2). Only no-clobber and op-target existence survive;
/// `validate_week_pattern` and the reference scans are stripped. Variants
/// copied verbatim from [WeekPatternError].
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum WeekPatternPrecheckError {
    /// A week pattern id is invalid
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(WeekPatternId),

    /// The week pattern id already exists
    #[error("week pattern id ({0:?}) already exists")]
    WeekPatternIdAlreadyExists(WeekPatternId),
}

impl crate::Data {
    /// Used internally
    ///
    /// Apply week pattern operations
    pub(crate) fn apply_week_pattern(
        &mut self,
        week_pattern_op: &AnnotatedWeekPatternOp,
    ) -> std::result::Result<AnnotatedWeekPatternOp, WeekPatternError> {
        match week_pattern_op {
            AnnotatedWeekPatternOp::Add(new_id, week_pattern) => {
                if self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .contains(new_id)
                {
                    return Err(WeekPatternError::WeekPatternIdAlreadyExists(*new_id));
                }

                self.inner_data.params.validate_week_pattern(week_pattern)?;

                self.inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .insert(*new_id, week_pattern.clone());

                Ok(AnnotatedWeekPatternOp::Remove(*new_id))
            }
            AnnotatedWeekPatternOp::Remove(id) => {
                if !self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .contains(id)
                {
                    return Err(WeekPatternError::InvalidWeekPatternId(*id));
                }

                for (slot_id, slot) in self.inner_data.params.slots.all_slots() {
                    if let Some(week_pattern_id) = &slot.week_pattern
                        && *id == *week_pattern_id
                    {
                        return Err(WeekPatternError::WeekPatternStillHasAssociatedSlots(
                            *id, *slot_id,
                        ));
                    }
                }

                for (incompat_id, incompat) in self.inner_data.params.incompats.incompat_map.iter()
                {
                    if let Some(week_pattern_id) = &incompat.week_pattern_id
                        && *id == *week_pattern_id
                    {
                        return Err(WeekPatternError::WeekPatternStillHasAssociatedIncompat(
                            *id,
                            incompat_id,
                        ));
                    }
                }

                let old_week_pattern = self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .remove(id)
                    .expect("Week pattern ID was checked above");

                Ok(AnnotatedWeekPatternOp::Add(*id, old_week_pattern))
            }
            AnnotatedWeekPatternOp::Update(id, new_week_pattern) => {
                self.inner_data
                    .params
                    .validate_week_pattern(new_week_pattern)?;

                if !self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .contains(id)
                {
                    return Err(WeekPatternError::InvalidWeekPatternId(*id));
                }

                // Guard: for every slot bound to this pattern, no colloscope row
                // may sit on a week the new pattern would silence — that would
                // strand an interrogation on an inactive week. A week is active
                // under the new pattern iff it runs interrogations and is not in
                // the new exclusion set. Rows key on the week id, so nothing else
                // needs to move; reject before mutating.
                for (slot_id, slot) in self.inner_data.params.slots.all_slots() {
                    if slot.week_pattern != Some(*id) {
                        continue;
                    }
                    for (week, _groups) in
                        self.inner_data.colloscope.interrogations_for_slot(*slot_id)
                    {
                        let week_runs = self
                            .inner_data
                            .params
                            .weeks()
                            .find_week(week)
                            .is_some_and(|w| w.interrogations);
                        if !week_runs || new_week_pattern.excluded_weeks.contains(&week) {
                            return Err(WeekPatternError::NotCompatibleSlotInColloscope(*slot_id));
                        }
                    }
                }

                let current_week_pattern = self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .get_mut(id)
                    .expect("week pattern id checked above");
                let old_week_pattern =
                    std::mem::replace(current_week_pattern, new_week_pattern.clone());

                Ok(AnnotatedWeekPatternOp::Update(*id, old_week_pattern))
            }
        }
    }

    /// Used internally by [crate::Data::force_apply]
    ///
    /// Thin copy of [Self::apply_week_pattern]: carve-out guards kept (returned
    /// as [WeekPatternPrecheckError]), invariant guards stripped (step-3 survey
    /// Table 1). May leave the state invalid; the caller owns checking and
    /// rollback.
    pub(crate) fn force_apply_week_pattern(
        &mut self,
        week_pattern_op: &AnnotatedWeekPatternOp,
    ) -> std::result::Result<AnnotatedWeekPatternOp, WeekPatternPrecheckError> {
        match week_pattern_op {
            AnnotatedWeekPatternOp::Add(new_id, week_pattern) => {
                if self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .contains(new_id)
                {
                    return Err(WeekPatternPrecheckError::WeekPatternIdAlreadyExists(
                        *new_id,
                    ));
                }

                // stripped: validate_week_pattern

                self.inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .insert(*new_id, week_pattern.clone());

                Ok(AnnotatedWeekPatternOp::Remove(*new_id))
            }
            AnnotatedWeekPatternOp::Remove(id) => {
                // stripped: slot-reference / incompat-reference scans
                let Some(old_week_pattern) = self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .remove(id)
                else {
                    return Err(WeekPatternPrecheckError::InvalidWeekPatternId(*id));
                };

                Ok(AnnotatedWeekPatternOp::Add(*id, old_week_pattern))
            }
            AnnotatedWeekPatternOp::Update(id, new_week_pattern) => {
                // stripped: validate_week_pattern + the colloscope silencing guard
                let Some(current_week_pattern) = self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .get_mut(id)
                else {
                    return Err(WeekPatternPrecheckError::InvalidWeekPatternId(*id));
                };

                let old_week_pattern =
                    std::mem::replace(current_week_pattern, new_week_pattern.clone());

                Ok(AnnotatedWeekPatternOp::Update(*id, old_week_pattern))
            }
        }
    }
}
