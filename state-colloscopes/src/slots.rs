//! Slots submodule
//!
//! This module defines the relevant types to describes the interrogation slots

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::colloscopes;
use crate::ids::{PeriodId, SlotId, SlotPairingRuleId, SubjectId, TeacherId, WeekPatternId};
use crate::ops::AnnotatedSlotOp;

/// Description of the interrogation slots
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slots {
    /// Slots for each subject
    ///
    /// Each item associates a subject id to a collection of slots
    /// There should be an entry for each valid subject with interrogations
    pub subject_map: BTreeMap<SubjectId, SubjectSlots>,
}

/// Description of the interrogation slots for a subject
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectSlots {
    /// Slots for the subject in order
    pub ordered_slots: Vec<(SlotId, Slot)>,
}

/// Description of a single slot
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    /// Teacher for the interrogation
    pub teacher_id: TeacherId,
    /// Day and start time for the interrogation
    /// The duration is fixed by the subject
    pub start_time: collomatique_time::SlotStart,
    /// Extra info that can be exported (like the room number)
    pub extra_info: String,
    /// Week pattern for the interrogation
    ///
    /// If None, the interrogation happens everyweek
    pub week_pattern: Option<WeekPatternId>,
    /// Cost for the interrogation
    ///
    /// An optional cost can be defined. By default, this will be 0.
    /// But a positive cost can be chosen to avoid a slot.
    /// A negative cost would rather favor a given slot
    pub cost: i32,
}

impl Slot {
    pub(crate) fn build_pattern_for_new_period(
        &self,
        new_period_desc: &[super::periods::WeekDesc],
        first_week: usize,
        week_patterns: &super::week_patterns::WeekPatterns,
    ) -> Vec<bool> {
        let mut base_pattern: Vec<_> = new_period_desc.iter().map(|x| x.interrogations).collect();

        if let Some(week_pattern_id) = self.week_pattern {
            let pattern = week_patterns.get_pattern(week_pattern_id);
            for (i, base_status) in base_pattern.iter_mut().enumerate() {
                let week_pattern_status = match pattern.get(first_week + i) {
                    Some(val) => *val,
                    None => true,
                };
                if !week_pattern_status {
                    *base_status = false;
                }
            }
        }

        base_pattern
    }
}

impl SubjectSlots {
    pub fn find_slot_position(&self, slot_id: SlotId) -> Option<usize> {
        for (pos, (id, _slot)) in self.ordered_slots.iter().enumerate() {
            if slot_id == *id {
                return Some(pos);
            }
        }
        None
    }

    pub fn find_slot(&self, slot_id: SlotId) -> Option<&Slot> {
        let pos = self.find_slot_position(slot_id)?;

        Some(
            &self
                .ordered_slots
                .get(pos)
                .expect("Position should be valid at this point")
                .1,
        )
    }
}

impl Slots {
    pub fn find_slot_subject_and_position(&self, slot_id: SlotId) -> Option<(SubjectId, usize)> {
        for (subject_id, subject_slots) in &self.subject_map {
            if let Some(pos) = subject_slots.find_slot_position(slot_id) {
                return Some((*subject_id, pos));
            }
        }
        None
    }

    pub fn find_slot(&self, slot_id: SlotId) -> Option<&Slot> {
        let (subject_id, pos) = self.find_slot_subject_and_position(slot_id)?;

        Some(
            &self
                .subject_map
                .get(&subject_id)
                .expect("Subject id should be valid at this point")
                .ordered_slots
                .get(pos)
                .expect("Position should be valid at this point")
                .1,
        )
    }
}

/// Errors for interrogation slot operations
///
/// These errors can be returned when trying to modify [crate::Data] with a slot op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SlotError {
    /// A slot id is invalid
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(SlotId),

    /// The slot id already exists
    #[error("slot id ({0:?}) already exists")]
    SlotIdAlreadyExists(SlotId),

    /// A position is outside of bounds
    #[error("Position {0} is outside the list (size = {1})")]
    PositionOutOfBounds(usize, usize),

    /// The previous slot given is not for the same subject
    #[error("Slot {0:?} to be previous slot is not for subject {1:?}")]
    PreviousSlotIsNotInRightSubject(SlotId, SubjectId),

    /// subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// subject has no interrogations
    #[error("subject ({0:?}) does not have interrogations")]
    SubjectHasNoInterrogation(SubjectId),

    /// teacher id is invalid
    #[error("invalid teacher id ({0:?})")]
    InvalidTeacherId(TeacherId),

    /// week pattern id is invalid
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(WeekPatternId),

    /// Provided teacher does not teach in the corresponding subject
    #[error("Provided teacher ({0:?}) does not teach in subject ({1:?})")]
    TeacherDoesNotTeachInSubject(TeacherId, SubjectId),

    /// Slot overlaps with next day
    #[error("The slot start time is too late and the slot overlaps with the next day")]
    SlotOverlapsWithNextDay,

    /// The slot is not empty in colloscope
    #[error("slot {0:?} in colloscope is not empty for period {1:?}")]
    NotEmptySlotInColloscope(SlotId, PeriodId),

    /// The slot in colloscope is incomaptible with the new week pattern
    #[error("slot {0:?} in colloscope is not compatible with the new week pattern {1:?}")]
    NotCompatibleSlotInColloscope(SlotId, Option<WeekPatternId>),

    /// The slot is referenced by a slot pairing rule
    #[error("slot id ({0:?}) is referenced by a slot pairing rule ({1:?})")]
    SlotIsReferencedBySlotPairingRule(SlotId, SlotPairingRuleId),
}

impl crate::Data {
    /// Used internally
    ///
    /// Apply slot operations
    pub(crate) fn apply_slot(
        &mut self,
        slot_op: &AnnotatedSlotOp,
    ) -> std::result::Result<AnnotatedSlotOp, SlotError> {
        match slot_op {
            AnnotatedSlotOp::AddAfter(new_id, subject_id, after_id, slot) => {
                if self
                    .inner_data
                    .params
                    .slots
                    .find_slot_subject_and_position(*new_id)
                    .is_some()
                {
                    return Err(SlotError::SlotIdAlreadyExists(*new_id));
                }
                self.inner_data.params.validate_slot(slot, *subject_id)?;

                let position = match after_id {
                    Some(id) => {
                        let (sub_id, after_pos) = self
                            .inner_data
                            .params
                            .slots
                            .find_slot_subject_and_position(*id)
                            .ok_or(SlotError::InvalidSlotId(*id))?;
                        if sub_id != *subject_id {
                            return Err(SlotError::PreviousSlotIsNotInRightSubject(
                                *id,
                                *subject_id,
                            ));
                        }

                        after_pos + 1
                    }
                    None => 0,
                };

                let subject_slots = self
                    .inner_data
                    .params
                    .slots
                    .subject_map
                    .get_mut(subject_id)
                    .ok_or(SlotError::SubjectHasNoInterrogation(*subject_id))?;

                subject_slots
                    .ordered_slots
                    .insert(position, (*new_id, slot.clone()));

                let subject = self
                    .inner_data
                    .params
                    .subjects
                    .find_subject(*subject_id)
                    .expect("Subject ID should be valid at this point");
                for (period_id, period) in &mut self.inner_data.colloscope.period_map {
                    if subject.excluded_periods.contains(period_id) {
                        continue;
                    }

                    period.slot_map.insert(
                        *new_id,
                        colloscopes::ColloscopeSlot::new_empty_from_params(
                            &self.inner_data.params,
                            *period_id,
                            *new_id,
                        ),
                    );
                }

                Ok(AnnotatedSlotOp::Remove(*new_id))
            }
            AnnotatedSlotOp::ChangePosition(id, new_pos) => {
                let Some((subject_id, old_pos)) = self
                    .inner_data
                    .params
                    .slots
                    .find_slot_subject_and_position(*id)
                else {
                    return Err(SlotError::InvalidSlotId(*id));
                };

                let subject_slots = self
                    .inner_data
                    .params
                    .slots
                    .subject_map
                    .get_mut(&subject_id)
                    .expect("Subject id should be valid at this point");

                if *new_pos >= subject_slots.ordered_slots.len() {
                    return Err(SlotError::PositionOutOfBounds(
                        *new_pos,
                        subject_slots.ordered_slots.len(),
                    ));
                }

                let data = subject_slots.ordered_slots.remove(old_pos);
                subject_slots.ordered_slots.insert(*new_pos, data);

                Ok(AnnotatedSlotOp::ChangePosition(*id, old_pos))
            }
            AnnotatedSlotOp::Remove(id) => {
                let Some((subject_id, old_pos)) = self
                    .inner_data
                    .params
                    .slots
                    .find_slot_subject_and_position(*id)
                else {
                    return Err(SlotError::InvalidSlotId(*id));
                };

                for (period_id, collo_period) in &self.inner_data.colloscope.period_map {
                    let Some(collo_slot) = collo_period.slot_map.get(id) else {
                        continue;
                    };

                    if !collo_slot.is_empty() {
                        return Err(SlotError::NotEmptySlotInColloscope(*id, *period_id));
                    }
                }

                for (rule_id, rule) in self
                    .inner_data
                    .params
                    .slot_pairings
                    .slot_pairing_rule_map
                    .entries()
                {
                    if rule.antecedent.slot_id == *id || rule.consequent.slot_id == *id {
                        return Err(SlotError::SlotIsReferencedBySlotPairingRule(*id, rule_id));
                    }
                }

                let subject_slots = self
                    .inner_data
                    .params
                    .slots
                    .subject_map
                    .get_mut(&subject_id)
                    .expect("Subject id should be valid at this point");
                let previous_id = (old_pos > 0).then(|| subject_slots.ordered_slots[old_pos - 1].0);
                let (_, old_slot) = subject_slots.ordered_slots.remove(old_pos);
                for collo_period in self.inner_data.colloscope.period_map.values_mut() {
                    // The slot might not be in period but this won't raise an error
                    collo_period.slot_map.remove(id);
                }

                Ok(AnnotatedSlotOp::AddAfter(
                    *id,
                    subject_id,
                    previous_id,
                    old_slot,
                ))
            }
            AnnotatedSlotOp::Update(slot_id, new_slot) => {
                let Some((subject_id, position)) = self
                    .inner_data
                    .params
                    .slots
                    .find_slot_subject_and_position(*slot_id)
                else {
                    return Err(SlotError::InvalidSlotId(*slot_id));
                };

                self.inner_data.params.validate_slot(new_slot, subject_id)?;
                let pattern = self
                    .inner_data
                    .params
                    .get_merged_pattern(new_slot.week_pattern);
                if !self.inner_data.colloscope.check_empty_on_removed_weeks(
                    *slot_id,
                    &self.inner_data.params.periods,
                    &pattern[..],
                ) {
                    return Err(SlotError::NotCompatibleSlotInColloscope(
                        *slot_id,
                        new_slot.week_pattern,
                    ));
                }

                let subject_slots = self
                    .inner_data
                    .params
                    .slots
                    .subject_map
                    .get_mut(&subject_id)
                    .expect("Subject id should be valid at this point");

                let old_slot = std::mem::replace(
                    &mut subject_slots.ordered_slots[position].1,
                    new_slot.clone(),
                );
                self.inner_data.colloscope.update_slot_for_week_pattern(
                    *slot_id,
                    &self.inner_data.params.periods,
                    &pattern[..],
                );

                Ok(AnnotatedSlotOp::Update(*slot_id, old_slot))
            }
        }
    }
}
