//! Week patterns submodule
//!
//! This module defines the relevant types to describes the week patterns

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Table;
use crate::ids::{IncompatId, SlotId, WeekPatternId};
use crate::ops::AnnotatedWeekPatternOp;

/// Description of the week patterns
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeekPatterns {
    /// Week patterns
    ///
    /// Each item associate to a single ID a sequence of weeks
    pub week_pattern_map: Table<WeekPatternId, WeekPattern>,
}

impl WeekPatterns {
    pub(crate) fn get_pattern(&self, week_pattern_id: WeekPatternId) -> Vec<bool> {
        self.week_pattern_map
            .get(&week_pattern_id)
            .expect("Week pattern id must be valid for get_pattern")
            .weeks
            .clone()
    }
}

/// Description of a week pattern
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeekPattern {
    /// Name of the week pattern for identification
    pub name: String,
    /// Weeks the interrogation happens on
    ///
    /// If the Vec is shorter than the total amount of weeks
    /// it is assumed the interrogation happens on all the
    /// remaining weeks.
    ///
    /// If the Vec is longer, the extra weeks are ignored
    /// They are kept in case some one expands again the number of weeks.
    pub weeks: Vec<bool>,
}

impl WeekPattern {
    pub fn add_weeks(&mut self, first_week: usize, week_count: usize) {
        assert!(self.weeks.len() >= first_week);

        self.weeks
            .splice(first_week..first_week, vec![true; week_count]);
    }

    pub fn clean_weeks(&mut self, first_week: usize, week_count: usize) {
        assert!(self.weeks.len() > first_week);

        let last_week = first_week + week_count;
        assert!(self.weeks.len() >= last_week);

        for week in &mut self.weeks[first_week..last_week] {
            *week = true;
        }
    }

    pub fn remove_weeks(&mut self, first_week: usize, week_count: usize) {
        assert!(self.weeks.len() > first_week);

        let last_week = first_week + week_count;
        assert!(self.weeks.len() >= last_week);

        for week in &self.weeks[first_week..last_week] {
            assert!(*week);
        }

        self.weeks.splice(first_week..last_week, vec![]);
    }

    pub fn can_remove_weeks(&self, first_week: usize, week_count: usize) -> bool {
        assert!(self.weeks.len() > first_week);

        let last_week = first_week + week_count;
        assert!(self.weeks.len() >= last_week);

        for week in &self.weeks[first_week..last_week] {
            if !*week {
                return false;
            }
        }

        true
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

    /// The week pattern does not have the right length
    #[error("week pattern does not have the right length")]
    BadWeekPatternLength,

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
                    .contains_key(new_id)
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
                    .contains_key(id)
                {
                    return Err(WeekPatternError::InvalidWeekPatternId(*id));
                }

                for subject_slots in self.inner_data.params.slots.subject_map.values() {
                    for (slot_id, slot) in &subject_slots.ordered_slots {
                        if let Some(week_pattern_id) = &slot.week_pattern
                            && *id == *week_pattern_id
                        {
                            return Err(WeekPatternError::WeekPatternStillHasAssociatedSlots(
                                *id, *slot_id,
                            ));
                        }
                    }
                }

                for (incompat_id, incompat) in
                    self.inner_data.params.incompats.incompat_map.entries()
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
                    .merge_pattern(&new_week_pattern.weeks);

                let Some(current_week_pattern) = self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .get_mut(id)
                else {
                    return Err(WeekPatternError::InvalidWeekPatternId(*id));
                };

                for subject_slots in self.inner_data.params.slots.subject_map.values() {
                    for (slot_id, slot) in &subject_slots.ordered_slots {
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
                }

                let old_week_pattern =
                    std::mem::replace(current_week_pattern, new_week_pattern.clone());
                for subject_slots in self.inner_data.params.slots.subject_map.values() {
                    for (slot_id, slot) in &subject_slots.ordered_slots {
                        if slot.week_pattern != Some(*id) {
                            continue;
                        }

                        self.inner_data.colloscope.update_slot_for_week_pattern(
                            *slot_id,
                            &self.inner_data.params.periods,
                            &new_merged_pattern,
                        );
                    }
                }

                Ok(AnnotatedWeekPatternOp::Update(*id, old_week_pattern))
            }
        }
    }
}
