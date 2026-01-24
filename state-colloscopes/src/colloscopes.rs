//! Colloscopes submodule
//!
//! This module defines the relevant types to describes the colloscopes

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ids::{GroupListId, PeriodId, SlotId, StudentId};
use crate::ops::ColloscopeOp;

/// Description of a colloscope
///
/// the ids should be valid with respect to the corresponding params
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Colloscope {
    pub period_map: BTreeMap<PeriodId, ColloscopePeriod>,
    pub group_lists: BTreeMap<GroupListId, ColloscopeGroupList>,
}

impl Colloscope {
    /// Builds an empty colloscope compatible with the given parameters
    ///
    /// The function might panic if the parameters do not satisfy parameters invariants
    /// You should check this before hand with [super::colloscope_params::Parameters::check_invariants].
    pub fn new_empty_from_params(params: &super::colloscope_params::Parameters) -> Self {
        let group_lists = params
            .group_lists
            .group_list_map
            .iter()
            .filter(|(_id, group_list)| !group_list.is_prefilled())
            .map(|(group_list_id, _group_list)| (*group_list_id, ColloscopeGroupList::new_empty()))
            .collect();

        let period_map = params
            .periods
            .ordered_period_list
            .iter()
            .map(|(period_id, _period)| {
                (
                    *period_id,
                    ColloscopePeriod::new_empty_from_params(params, *period_id),
                )
            })
            .collect();

        Colloscope {
            period_map,
            group_lists,
        }
    }

    pub fn update_ops(&self, other: Self) -> Option<Vec<ColloscopeOp>> {
        let mut output = vec![];

        if self.group_lists.len() != other.group_lists.len() {
            return None;
        }
        for (group_list_id, other_group_list) in other.group_lists {
            let group_list = self.group_lists.get(&group_list_id)?;

            if *group_list != other_group_list {
                output.push(ColloscopeOp::UpdateGroupList(
                    group_list_id,
                    other_group_list,
                ));
            }
        }

        if self.period_map.len() != other.period_map.len() {
            return None;
        }
        for (period_id, other_period) in other.period_map {
            let period = self.period_map.get(&period_id)?;
            if period.slot_map.len() != other_period.slot_map.len() {
                return None;
            }
            for (slot_id, other_slot) in other_period.slot_map {
                let slot = period.slot_map.get(&slot_id)?;
                if slot.interrogations.len() != other_slot.interrogations.len() {
                    return None;
                }
                for (week_in_period, other_interrogation_opt) in
                    other_slot.interrogations.into_iter().enumerate()
                {
                    let interrogation_opt = slot.interrogations.get(week_in_period)?;
                    if interrogation_opt.is_some() != other_interrogation_opt.is_some() {
                        return None;
                    }
                    let (Some(interrogation), Some(other_interrogation)) =
                        (interrogation_opt, other_interrogation_opt)
                    else {
                        continue;
                    };
                    if *interrogation != other_interrogation {
                        output.push(ColloscopeOp::UpdateInterrogation(
                            period_id,
                            slot_id,
                            week_in_period,
                            other_interrogation,
                        ));
                    }
                }
            }
        }

        Some(output)
    }
}

impl Colloscope {
    pub(crate) fn validate_against_params(
        &self,
        params: &super::colloscope_params::Parameters,
    ) -> Result<(), super::ColloscopeError> {
        use super::ColloscopeError;

        if self.period_map.len() != params.periods.ordered_period_list.len() {
            return Err(ColloscopeError::WrongPeriodCountInColloscopeData);
        }

        for (period_id, period) in &self.period_map {
            if params.periods.find_period(*period_id).is_none() {
                return Err(ColloscopeError::InvalidPeriodId(*period_id));
            }
            period.validate_against_params(*period_id, params)?;
        }

        // Validate group lists: should contain exactly non-prefilled group lists
        for (group_list_id, group_list) in &self.group_lists {
            let Some(params_group_list) = params.group_lists.group_list_map.get(group_list_id)
            else {
                return Err(ColloscopeError::InvalidGroupListId(*group_list_id));
            };
            if params_group_list.is_prefilled() {
                return Err(ColloscopeError::PrefilledGroupListInColloscope(
                    *group_list_id,
                ));
            }
            group_list.validate_against_params(
                *group_list_id,
                &params_group_list.params,
                &params.students,
            )?;
        }

        // Check all non-prefilled group lists are present
        for (group_list_id, group_list) in &params.group_lists.group_list_map {
            if !group_list.is_prefilled() && !self.group_lists.contains_key(group_list_id) {
                return Err(ColloscopeError::MissingNonPrefilledGroupList(
                    *group_list_id,
                ));
            }
        }

        Ok(())
    }

    pub(crate) fn check_empty_on_removed_weeks(
        &self,
        slot_id: SlotId,
        periods: &super::periods::Periods,
        pattern: &[bool],
    ) -> bool {
        let mut current_first_week = 0usize;
        for (period_id, period_desc) in &periods.ordered_period_list {
            let last_week = current_first_week + period_desc.len();
            if pattern.len() < last_week {
                return false;
            }

            let collo_period = self
                .period_map
                .get(period_id)
                .expect("Period Id should be valid");
            if let Some(collo_slot) = collo_period.slot_map.get(&slot_id) {
                if !collo_slot.check_empty_on_removed_weeks(&pattern[current_first_week..last_week])
                {
                    return false;
                }
            }

            current_first_week += period_desc.len();
        }
        if current_first_week != pattern.len() {
            return false;
        }
        true
    }

    pub(crate) fn update_slot_to_match_week_pattern(
        &mut self,
        slot_id: SlotId,
        params: &super::colloscope_params::Parameters,
    ) {
        let slot = params
            .slots
            .find_slot(slot_id)
            .expect("Slot ID should be valid");
        let pattern = params.get_merged_pattern(slot.week_pattern);

        self.update_slot_for_week_pattern(slot_id, &params.periods, &pattern);
    }

    pub(crate) fn update_slot_for_week_pattern(
        &mut self,
        slot_id: SlotId,
        periods: &super::periods::Periods,
        pattern: &[bool],
    ) {
        let mut current_first_week = 0usize;
        for (period_id, period_desc) in &periods.ordered_period_list {
            let last_week = current_first_week + period_desc.len();
            assert!(pattern.len() >= last_week);

            let collo_period = self
                .period_map
                .get_mut(period_id)
                .expect("Period Id should be valid");
            if let Some(collo_slot) = collo_period.slot_map.get_mut(&slot_id) {
                collo_slot.update_slot_for_week_pattern(&pattern[current_first_week..last_week]);
            }

            current_first_week += period_desc.len();
        }
        assert!(current_first_week == pattern.len());
    }
}

/// Description of a single period in a colloscope
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColloscopePeriod {
    /// Map between slots and interrogations for these slots
    pub slot_map: BTreeMap<SlotId, ColloscopeSlot>,
}

impl ColloscopePeriod {
    pub fn is_empty(&self) -> bool {
        self.slot_map.iter().all(|(_slot_id, slot)| slot.is_empty())
    }

    pub(crate) fn new_empty_from_params(
        params: &super::colloscope_params::Parameters,
        period_id: PeriodId,
    ) -> Self {
        if params.periods.find_period_position(period_id).is_none() {
            panic!("Period ID should be valid");
        }

        let mut slot_map = BTreeMap::new();

        for (subject_id, subject) in &params.subjects.ordered_subject_list {
            if subject.excluded_periods.contains(&period_id) {
                continue;
            }
            if subject.parameters.interrogation_parameters.is_none() {
                continue;
            }

            let subject_slots = params
                .slots
                .subject_map
                .get(subject_id)
                .expect("Subjects should have slots");

            for (slot_id, _slot) in &subject_slots.ordered_slots {
                slot_map.insert(
                    *slot_id,
                    ColloscopeSlot::new_empty_from_params(params, period_id, *slot_id),
                );
            }
        }

        ColloscopePeriod { slot_map }
    }

    pub(crate) fn validate_against_params(
        &self,
        period_id: PeriodId,
        params: &super::colloscope_params::Parameters,
    ) -> Result<(), super::ColloscopeError> {
        use super::ColloscopeError;

        let mut slot_count = 0usize;

        for (subject_id, subject) in &params.subjects.ordered_subject_list {
            if subject.excluded_periods.contains(&period_id) {
                continue;
            }
            if subject.parameters.interrogation_parameters.is_none() {
                continue;
            }

            let subject_slots = params
                .slots
                .subject_map
                .get(subject_id)
                .expect("Subject should have slots at this point");
            slot_count += subject_slots.ordered_slots.len();
        }

        if slot_count != self.slot_map.len() {
            return Err(ColloscopeError::WrongSlotCountInPeriodInColloscopeData(
                period_id,
            ));
        }

        for (slot_id, slot) in &self.slot_map {
            let Some((subject_id, _pos)) = params.slots.find_slot_subject_and_position(*slot_id)
            else {
                return Err(ColloscopeError::InvalidSlotId(*slot_id));
            };

            let param_subject = params
                .subjects
                .find_subject(subject_id)
                .expect("Subject ID should be valid");

            if param_subject.excluded_periods.contains(&period_id) {
                return Err(ColloscopeError::InvalidSlotId(*slot_id));
            }

            if param_subject.parameters.interrogation_parameters.is_none() {
                panic!("Inconsistent data in params")
            }

            slot.validate_against_params(period_id, *slot_id, params)?;
        }

        Ok(())
    }
}

/// Description of a single slot for a subject in a period in a colloscope
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColloscopeSlot {
    /// List of interrogations in a slot during the period
    ///
    /// The list contains a cell for each week in the period.
    /// If there cannot be an interrogation
    /// there is is still a cell, but the option is set to None.
    ///
    /// If however there is a possible interrogation, it will be a Some
    /// value, even if no group is actually assigned. This will rather
    /// be described within the [ColloscopeInterrogation] struct.
    pub interrogations: Vec<Option<ColloscopeInterrogation>>,
}

impl ColloscopeSlot {
    pub fn is_empty(&self) -> bool {
        self.interrogations
            .iter()
            .all(|interrogation_opt| match interrogation_opt {
                Some(interrogation) => interrogation.is_empty(),
                None => true,
            })
    }

    pub(crate) fn new_empty_from_params(
        params: &super::colloscope_params::Parameters,
        period_id: PeriodId,
        slot_id: SlotId,
    ) -> Self {
        let Some(period_pos) = params.periods.find_period_position(period_id) else {
            panic!("Period ID should be valid");
        };
        let period = &params.periods.ordered_period_list[period_pos].1;

        let first_week: usize = (0..period_pos)
            .into_iter()
            .map(|i| params.periods.ordered_period_list[i].1.len())
            .sum();

        let (subject_id, pos) = params
            .slots
            .find_slot_subject_and_position(slot_id)
            .expect("Slot ID should be valid");

        let subject = params
            .subjects
            .find_subject(subject_id)
            .expect("Subject ID should be valid");

        if subject.excluded_periods.contains(&period_id) {
            panic!("Subject should run on given period")
        }
        if subject.parameters.interrogation_parameters.is_none() {
            panic!("Subject should have interrogations")
        }

        let orig_slots = params
            .slots
            .subject_map
            .get(&subject_id)
            .expect("Subject ID should be valid");

        let slot = &orig_slots.ordered_slots[pos].1;

        let mut interrogations = vec![];

        let week_pattern = params.get_merged_pattern(slot.week_pattern);
        for i in 0..period.len() {
            let current_week = first_week + i;
            let is_week_active = week_pattern[current_week];
            interrogations.push(if is_week_active {
                Some(ColloscopeInterrogation::default())
            } else {
                None
            });
        }

        ColloscopeSlot { interrogations }
    }

    pub(crate) fn validate_against_params(
        &self,
        period_id: PeriodId,
        slot_id: SlotId,
        params: &super::colloscope_params::Parameters,
    ) -> Result<(), super::ColloscopeError> {
        use super::ColloscopeError;

        let Some(period_pos) = params.periods.find_period_position(period_id) else {
            return Err(ColloscopeError::InvalidPeriodId(period_id));
        };
        let period = &params.periods.ordered_period_list[period_pos].1;

        let first_week_num: usize = (0..period_pos)
            .into_iter()
            .map(|i| params.periods.ordered_period_list[i].1.len())
            .sum();

        let Some(orig_slot) = params.slots.find_slot(slot_id) else {
            return Err(ColloscopeError::InvalidSlotId(slot_id));
        };

        if period.len() != self.interrogations.len() {
            return Err(
                ColloscopeError::WrongInterrogationCountForSlotInPeriodInColloscopeData(
                    period_id, slot_id,
                ),
            );
        }

        let week_pattern = params.get_merged_pattern(orig_slot.week_pattern);

        for (i, interrogation_opt) in self.interrogations.iter().enumerate() {
            let current_week = first_week_num + i;
            let is_week_active = week_pattern[current_week];

            if !is_week_active {
                if interrogation_opt.is_some() {
                    return Err(ColloscopeError::InterrogationOnNonInterrogationWeek(
                        period_id,
                        slot_id,
                        current_week,
                    ));
                }
                continue;
            }

            let Some(interrogation) = interrogation_opt else {
                return Err(ColloscopeError::MissingInterrogationOnInterrogationWeek(
                    period_id,
                    slot_id,
                    current_week,
                ));
            };

            interrogation.validate_against_params(period_id, slot_id, current_week, params)?;
        }

        Ok(())
    }

    pub(crate) fn check_empty_on_removed_weeks(&self, pattern: &[bool]) -> bool {
        for i in 0..self.interrogations.len() {
            let interrogation_opt = &self.interrogations[i];
            let week_active = match pattern.get(i) {
                Some(val) => *val,
                None => false,
            };

            if week_active {
                continue;
            }

            if let Some(interrogation) = interrogation_opt {
                if !interrogation.is_empty() {
                    return false;
                }
            }
        }

        true
    }

    pub(crate) fn update_slot_for_week_pattern(&mut self, pattern: &[bool]) {
        self.interrogations.resize(pattern.len(), None);

        for (interrogation_opt, week_active) in self.interrogations.iter_mut().zip(pattern.iter()) {
            if *week_active {
                if interrogation_opt.is_none() {
                    *interrogation_opt = Some(ColloscopeInterrogation::default());
                }
            } else {
                *interrogation_opt = None;
            }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColloscopeInterrogation {
    /// List of groups assigned to the interrogation
    pub assigned_groups: BTreeSet<u32>,
}

impl ColloscopeInterrogation {
    pub fn is_empty(&self) -> bool {
        self.assigned_groups.is_empty()
    }

    pub(crate) fn validate_against_params(
        &self,
        period_id: PeriodId,
        slot_id: SlotId,
        week: usize,
        params: &super::colloscope_params::Parameters,
    ) -> Result<(), super::ColloscopeError> {
        use super::ColloscopeError;

        let Some(subject_association) = params.group_lists.subjects_associations.get(&period_id)
        else {
            return Err(ColloscopeError::InvalidPeriodId(period_id));
        };

        let Some((subject_id, _pos)) = params.slots.find_slot_subject_and_position(slot_id) else {
            return Err(ColloscopeError::InvalidSlotId(slot_id));
        };

        let group_list_id_opt = subject_association.get(&subject_id);

        let first_forbidden_value = match group_list_id_opt {
            None => 0u32,
            Some(group_list_id) => {
                let group_list = params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                    .expect("Group list id should be valid");

                group_list.params.group_names.len() as u32
            }
        };

        for group_num in &self.assigned_groups {
            if *group_num >= first_forbidden_value {
                return Err(ColloscopeError::InvalidGroupNumInInterrogation(
                    period_id, slot_id, week,
                ));
            }
        }

        Ok(())
    }
}

/// Description of a group list in a colloscope
///
/// This is basically map between students that are in the group lists
/// and actual group numbers
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ColloscopeGroupList {
    pub groups_for_students: BTreeMap<StudentId, u32>,
}

impl ColloscopeGroupList {
    pub fn is_empty(&self) -> bool {
        self.groups_for_students.is_empty()
    }

    /// Builds an empty group list compatible with the given parameters
    ///
    /// `group_list_id` refers to the group list in the parameters to start from.
    /// The function panics if the id is not valid.
    ///
    /// The function might panic if the parameters do not satisfy parameters invariants
    /// You should check this before hand with [super::colloscope_params::Parameters::check_invariants].
    pub(crate) fn new_empty() -> Self {
        ColloscopeGroupList {
            groups_for_students: BTreeMap::new(),
        }
    }
}

impl ColloscopeGroupList {
    pub(crate) fn validate_against_params(
        &self,
        group_list_id: GroupListId,
        group_list_params: &super::group_lists::GroupListParameters,
        students: &super::students::Students,
    ) -> Result<(), super::ColloscopeError> {
        use super::ColloscopeError;

        let first_forbidden_value = group_list_params.group_names.len() as u32;

        for (student_id, group_num) in &self.groups_for_students {
            if group_list_params.excluded_students.contains(student_id) {
                return Err(ColloscopeError::ExcludedStudentInGroupList(
                    group_list_id,
                    *student_id,
                ));
            }

            if !students.student_map.contains_key(student_id) {
                return Err(ColloscopeError::InvalidStudentId(*student_id));
            }

            if *group_num >= first_forbidden_value {
                return Err(ColloscopeError::InvalidGroupNumForStudentInGroupList(
                    group_list_id,
                    *student_id,
                ));
            }
        }

        Ok(())
    }
}
