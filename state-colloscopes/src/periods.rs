//! Period submodule
//!
//! This module defines the relevant types to describes the periods

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::OrderedTable;
use crate::colloscopes::ColloscopePeriod;
use crate::ids::{
    PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId, SubjectId, WeekPatternId,
};
use crate::ops::AnnotatedPeriodOp;

/// Description of the periods
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Periods {
    /// Start date for the colloscope
    ///
    /// The date might not be set but of course, this will hinder
    /// the eventual pretty output
    pub first_week: Option<collomatique_time::WeekStart>,

    /// Ordered list of periods
    ///
    /// This field gives the relative order of the different
    /// periods identified by their ids
    ///
    /// For each period, we get also a list of boolean
    /// Each boolean represents a week. If it is true
    /// there is an interrogation on the given week
    /// otherwise there isn't.
    pub ordered_period_list: OrderedTable<PeriodId, Vec<WeekDesc>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeekDesc {
    pub interrogations: bool,
    pub annotation: Option<non_empty_string::NonEmptyString>,
}

impl Default for WeekDesc {
    fn default() -> Self {
        WeekDesc {
            interrogations: true,
            annotation: None,
        }
    }
}

impl WeekDesc {
    pub fn new(interrogations: bool) -> WeekDesc {
        WeekDesc {
            interrogations,
            annotation: None,
        }
    }
}

impl Periods {
    pub fn count_weeks(&self) -> usize {
        self.ordered_period_list.iter().map(|x| x.1.len()).sum()
    }

    /// Finds the position of a period by id
    pub fn find_period_position(&self, id: PeriodId) -> Option<usize> {
        self.ordered_period_list.position_of(&id)
    }

    /// Finds the position of a period by id and gives the number of the first week
    pub fn find_period_position_and_first_week(&self, id: PeriodId) -> Option<(usize, usize)> {
        let mut first_week = 0usize;

        for (pos, (period_id, desc)) in self.ordered_period_list.iter().enumerate() {
            if period_id == id {
                return Some((pos, first_week));
            }
            first_week += desc.len();
        }

        None
    }

    /// Finds the position of a period by id and gives the total number of weeks up to and including the
    /// given period
    pub fn find_period_position_and_total_number_of_weeks(
        &self,
        id: PeriodId,
    ) -> Option<(usize, usize)> {
        let mut total_weeks = 0usize;

        for (pos, (period_id, desc)) in self.ordered_period_list.iter().enumerate() {
            total_weeks += desc.len();
            if period_id == id {
                return Some((pos, total_weeks));
            }
        }

        None
    }

    /// Finds a period by id
    pub fn find_period(&self, id: PeriodId) -> Option<&Vec<WeekDesc>> {
        self.ordered_period_list.get(&id)
    }

    /// Finds the first week number and the length of a period
    pub fn get_first_week_and_length_for_period(&self, id: PeriodId) -> Option<(usize, usize)> {
        let mut first_week = 0usize;

        for (period_id, desc) in self.ordered_period_list.iter() {
            if period_id == id {
                return Some((first_week, desc.len()));
            }
            first_week += desc.len();
        }

        None
    }
}

/// Errors for periods operations
///
/// These errors can be returned when trying to modify [crate::Data] with a period op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PeriodError {
    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// The period id already exists
    #[error("period id ({0:?}) already exists")]
    PeriodIdAlreadyExists(PeriodId),

    /// The period is referenced by a subject
    #[error("period id ({0:?}) is referenced by subject {1:?}")]
    PeriodIsReferencedBySubject(PeriodId, SubjectId),

    /// The period is referenced by a student
    #[error("period id ({0:?}) is referenced by student {1:?}")]
    PeriodIsReferencedByStudent(PeriodId, StudentId),

    /// Some non-default assignments are still present for the period
    #[error(
        "period id ({0:?}) has non-default assignments for subject id {1:?} and cannot be removed"
    )]
    PeriodStillHasNonTrivialAssignments(PeriodId, SubjectId),

    /// Some non-default group list association are still present for the period
    #[error("period id ({0:?}) has non-default group list associations and cannot be removed")]
    PeriodStillHasNonTrivialGroupListAssociation(PeriodId),

    /// Period is not empty in colloscope
    #[error("period id ({0:?}) is not empty in colloscope")]
    NotEmptyPeriodInColloscope(PeriodId),

    /// A week pattern is not trivial on the period to be cut
    #[error("week pattern {1:?} is not trivial for the period {0:?}")]
    NonTrivialWeekPattern(PeriodId, WeekPatternId),

    /// The slot in colloscope is incompatible with the new period
    #[error("slot {0:?} in colloscope is not compatible with the new period")]
    NotCompatibleSlotInColloscope(SlotId),

    /// The period is referenced by a pairing rule
    #[error("period id ({0:?}) is referenced by pairing rule {1:?}")]
    PeriodIsReferencedByPairingRule(PeriodId, PairingRuleId),

    /// The period is referenced by a slot pairing rule
    #[error("period id ({0:?}) is referenced by slot pairing rule {1:?}")]
    PeriodIsReferencedBySlotPairingRule(PeriodId, SlotPairingRuleId),
}

impl crate::Data {
    /// Used internally
    ///
    /// Apply period operations
    pub(crate) fn apply_period(
        &mut self,
        period_op: &AnnotatedPeriodOp,
    ) -> std::result::Result<AnnotatedPeriodOp, PeriodError> {
        match period_op {
            AnnotatedPeriodOp::ChangeStartDate(new_date) => {
                let old_date = std::mem::replace(
                    &mut self.inner_data.params.periods.first_week,
                    new_date.clone(),
                );
                Ok(AnnotatedPeriodOp::ChangeStartDate(old_date))
            }
            AnnotatedPeriodOp::AddFront(period_id, desc) => {
                if self
                    .inner_data
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_some()
                {
                    return Err(PeriodError::PeriodIdAlreadyExists(*period_id));
                }

                self.inner_data
                    .params
                    .periods
                    .ordered_period_list
                    .insert_at(0, *period_id, desc.clone())
                    .expect("period id absence checked above");
                // A fresh period carries no assignments and no associations,
                // so neither (sparse) junction table gets a row until content
                // is added.
                for week_pattern in self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .values_mut()
                {
                    week_pattern.add_weeks(0, desc.len());
                }
                self.inner_data.colloscope.period_map.insert(
                    *period_id,
                    ColloscopePeriod::new_empty_from_params(&self.inner_data.params, *period_id),
                );
                Ok(AnnotatedPeriodOp::Remove(*period_id))
            }
            AnnotatedPeriodOp::AddAfter(period_id, after_id, desc) => {
                if self
                    .inner_data
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_some()
                {
                    return Err(PeriodError::PeriodIdAlreadyExists(*period_id));
                }

                let Some((position, new_first_week)) = self
                    .inner_data
                    .params
                    .periods
                    .find_period_position_and_total_number_of_weeks(*after_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(*after_id));
                };

                self.inner_data
                    .params
                    .periods
                    .ordered_period_list
                    .insert_at(position + 1, *period_id, desc.clone())
                    .expect("period id absence checked above");
                // A fresh period carries no assignments and no associations,
                // so neither (sparse) junction table gets a row until content
                // is added.
                for week_pattern in self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .values_mut()
                {
                    week_pattern.add_weeks(new_first_week, desc.len());
                }
                self.inner_data.colloscope.period_map.insert(
                    *period_id,
                    ColloscopePeriod::new_empty_from_params(&self.inner_data.params, *period_id),
                );
                Ok(AnnotatedPeriodOp::Remove(*period_id))
            }
            AnnotatedPeriodOp::Remove(period_id) => {
                let Some((position, first_week)) = self
                    .inner_data
                    .params
                    .periods
                    .find_period_position_and_first_week(*period_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(*period_id));
                };

                let colloscope_period = self
                    .inner_data
                    .colloscope
                    .period_map
                    .get(period_id)
                    .expect("Period ID should be valid at this point");

                if !colloscope_period.is_empty() {
                    return Err(PeriodError::NotEmptyPeriodInColloscope(*period_id));
                }

                let week_count = self
                    .inner_data
                    .params
                    .periods
                    .ordered_period_list
                    .get_at(position)
                    .expect("position comes from find_period_position")
                    .1
                    .len();

                for (week_pattern_id, week_pattern) in
                    self.inner_data.params.week_patterns.week_pattern_map.iter()
                {
                    if !week_pattern.can_remove_weeks(first_week, week_count) {
                        return Err(PeriodError::NonTrivialWeekPattern(
                            *period_id,
                            week_pattern_id,
                        ));
                    }
                }

                for (subject_id, subject) in
                    self.inner_data.params.subjects.ordered_subject_list.iter()
                {
                    if subject.excluded_periods.contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedBySubject(
                            *period_id, subject_id,
                        ));
                    }
                }

                for (student_id, student) in self.inner_data.params.students.student_map.iter() {
                    if student.excluded_periods.contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedByStudent(
                            *period_id, student_id,
                        ));
                    }
                }

                for (rule_id, rule) in self.inner_data.params.pairings.pairing_rule_map.iter() {
                    if rule.excluded_periods.contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedByPairingRule(
                            *period_id, rule_id,
                        ));
                    }
                }

                for (rule_id, rule) in self
                    .inner_data
                    .params
                    .slot_pairings
                    .slot_pairing_rule_map
                    .iter()
                {
                    if rule.excluded_periods.contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedBySlotPairingRule(
                            *period_id, rule_id,
                        ));
                    }
                }

                // Under canonical-absent, a row exists iff it is non-trivial,
                // so any surviving row for this period blocks the removal.
                if let Some((subject_id, _)) = self
                    .inner_data
                    .params
                    .assignments
                    .subjects_for_period(*period_id)
                    .next()
                {
                    return Err(PeriodError::PeriodStillHasNonTrivialAssignments(
                        *period_id, subject_id,
                    ));
                }

                if self
                    .inner_data
                    .params
                    .group_lists
                    .subjects_associations
                    .keys()
                    .any(|(p, _)| p == *period_id)
                {
                    return Err(PeriodError::PeriodStillHasNonTrivialGroupListAssociation(
                        *period_id,
                    ));
                }

                let previous_id = (position > 0).then(|| {
                    self.inner_data
                        .params
                        .periods
                        .ordered_period_list
                        .get_at(position - 1)
                        .expect("position > 0 checked")
                        .0
                });

                let (_, old_desc) = self
                    .inner_data
                    .params
                    .periods
                    .ordered_period_list
                    .remove_at(position);
                // Drop this period's rows from the associations table (none
                // remain once the emptiness check passes, but stay consistent
                // regardless). Assignment rows are already gone: the guard
                // above rejects the removal while any survive.
                let association_keys: Vec<_> = self
                    .inner_data
                    .params
                    .group_lists
                    .subjects_associations
                    .keys()
                    .filter(|(p, _)| *p == *period_id)
                    .collect();
                for key in association_keys {
                    self.inner_data
                        .params
                        .group_lists
                        .subjects_associations
                        .remove(&key);
                }
                for week_pattern in self
                    .inner_data
                    .params
                    .week_patterns
                    .week_pattern_map
                    .values_mut()
                {
                    week_pattern.remove_weeks(first_week, week_count);
                }
                self.inner_data.colloscope.period_map.remove(period_id);

                Ok(match previous_id {
                    None => AnnotatedPeriodOp::AddFront(*period_id, old_desc),
                    Some(prev) => AnnotatedPeriodOp::AddAfter(*period_id, prev, old_desc),
                })
            }
            AnnotatedPeriodOp::Update(period_id, desc) => {
                let Some((position, first_week)) = self
                    .inner_data
                    .params
                    .periods
                    .find_period_position_and_first_week(*period_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(*period_id));
                };

                let period = self
                    .inner_data
                    .params
                    .periods
                    .ordered_period_list
                    .get_at(position)
                    .expect("position comes from find_period_position_and_first_week")
                    .1;
                let old_length = period.len();
                if desc.len() < old_length {
                    for (week_pattern_id, week_pattern) in
                        self.inner_data.params.week_patterns.week_pattern_map.iter()
                    {
                        if !week_pattern
                            .can_remove_weeks(first_week + desc.len(), old_length - desc.len())
                        {
                            return Err(PeriodError::NonTrivialWeekPattern(
                                *period_id,
                                week_pattern_id,
                            ));
                        }
                    }
                }
                let colloscope_period = self
                    .inner_data
                    .colloscope
                    .period_map
                    .get(period_id)
                    .expect("Period ID should be valid at this point");
                for (slot_id, collo_slot) in &colloscope_period.slot_map {
                    let slot = self
                        .inner_data
                        .params
                        .slots
                        .find_slot(*slot_id)
                        .expect("Slot ID should be valid");
                    let new_pattern = slot.build_pattern_for_new_period(
                        desc,
                        first_week,
                        &self.inner_data.params.week_patterns,
                    );

                    if !collo_slot.check_empty_on_removed_weeks(&new_pattern) {
                        return Err(PeriodError::NotCompatibleSlotInColloscope(*slot_id));
                    }
                }

                let old_desc = self
                    .inner_data
                    .params
                    .periods
                    .ordered_period_list
                    .replace_value_at(position, desc.clone());
                if desc.len() > old_length {
                    let first_week_to_add = first_week + old_length;
                    for week_pattern in self
                        .inner_data
                        .params
                        .week_patterns
                        .week_pattern_map
                        .values_mut()
                    {
                        week_pattern.add_weeks(first_week_to_add, desc.len() - old_length);
                    }
                } else if desc.len() < old_length {
                    let first_week_to_remove = first_week + desc.len();
                    for week_pattern in self
                        .inner_data
                        .params
                        .week_patterns
                        .week_pattern_map
                        .values_mut()
                    {
                        week_pattern.remove_weeks(first_week_to_remove, old_length - desc.len());
                    }
                }
                let slot_ids: Vec<_> = self
                    .inner_data
                    .params
                    .slots
                    .all_slots()
                    .map(|(slot_id, _slot)| *slot_id)
                    .collect();
                for slot_id in slot_ids {
                    self.inner_data
                        .colloscope
                        .update_slot_to_match_week_pattern(slot_id, &self.inner_data.params);
                }

                Ok(AnnotatedPeriodOp::Update(*period_id, old_desc))
            }
        }
    }
}
