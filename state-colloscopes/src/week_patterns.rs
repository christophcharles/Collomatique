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
    /// there is no pattern). Homed here so consumers holding only a `Periods` +
    /// `WeekPatterns` pair — e.g. the gtk4 colloscope grid — can call it;
    /// [`super::colloscope_params::Parameters::is_week_active`] delegates to it.
    ///
    /// Returns `false` for a dangling week id; a dangling pattern id is treated
    /// as "no exclusion". Both are bugs on validated data.
    pub fn is_week_active(
        &self,
        periods: &super::periods::Periods,
        week: WeekId,
        pattern: Option<WeekPatternId>,
    ) -> bool {
        let Some(week_desc) = periods.find_week(week) else {
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
                let new_merged_pattern = self
                    .inner_data
                    .params
                    .merge_excluded(&new_week_pattern.excluded_weeks);

                let Some(current_week_pattern) = self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .get_mut(id)
                else {
                    return Err(WeekPatternError::InvalidWeekPatternId(*id));
                };

                for (slot_id, slot) in self.inner_data.params.slots.all_slots() {
                    if slot.week_pattern != Some(*id) {
                        continue;
                    }

                    if !self.inner_data.colloscope.check_empty_on_removed_weeks(
                        *slot_id,
                        &self.inner_data.params.periods,
                        &new_merged_pattern,
                    ) {
                        return Err(WeekPatternError::NotCompatibleSlotInColloscope(*slot_id));
                    }
                }

                let old_week_pattern =
                    std::mem::replace(current_week_pattern, new_week_pattern.clone());
                for (slot_id, slot) in self.inner_data.params.slots.all_slots() {
                    if slot.week_pattern != Some(*id) {
                        continue;
                    }

                    self.inner_data.colloscope.update_slot_for_week_pattern(
                        *slot_id,
                        &self.inner_data.params.periods,
                        &new_merged_pattern,
                    );
                }

                Ok(AnnotatedWeekPatternOp::Update(*id, old_week_pattern))
            }
        }
    }
}
