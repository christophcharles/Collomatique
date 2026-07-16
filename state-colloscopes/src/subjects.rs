//! Subjects submodule
//!
//! This module defines the relevant types to describes the subjects

use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, num::NonZeroU32};
use thiserror::Error;

use collomatique_state::{Join, References};

use crate::OrderedTable;
use crate::colloscopes;
use crate::ids::{
    GroupListId, IncompatId, NewId, PairingRuleId, PeriodId, SlotId, SubjectId, TeacherId,
};
use crate::ops::AnnotatedSubjectOp;

/// Description of the subjects
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Subjects {
    /// Ordered list of subjects
    ///
    /// Each item represent a subject. It is described
    /// by a unique id and a description of type [Subject]
    pub ordered_subject_list: OrderedTable<SubjectId, Subject>,
}

/// Description of one subject
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, References, Join)]
#[join(error = NewId)]
pub struct Subject {
    /// Parameters for the subject
    ///
    /// This is separated because those parameters do
    /// not need to be checked
    pub parameters: SubjectParameters,
    /// Periods that should not be covered by the subject
    ///
    /// By default a subject is present for every period.
    #[fk]
    pub excluded_periods: BTreeSet<PeriodId>,
}

/// Description of one subject
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectParameters {
    /// Name of the subject
    ///
    /// This is just a descriptive string
    pub name: String,
    /// Parameters for the interrogations
    ///
    /// If `None`, this means there are no interrogations
    /// for this subject.
    pub interrogation_parameters: Option<SubjectInterrogationParameters>,
}

/// Description of the interrogations parameters for a subject
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectInterrogationParameters {
    /// Students per group
    ///
    /// This is the number of students that should be
    /// in a single group.
    ///
    /// This is not entirely fixed by the group list as
    /// the same group list can be used for different
    /// subjects and not all students must attend all subjects.
    pub students_per_group: std::ops::RangeInclusive<NonZeroU32>,
    /// number of groups to have during a single interrogation
    ///
    /// an interrogation can always have no groups. But we can
    /// force having several groups in a single interrogation
    /// and obviously, we can limit the number.
    ///
    /// This has two main applications:
    /// - for practical tutorials (in physics or computer science for instance),
    ///   it is sometimes practical to use the same group list as for other
    ///   subjects with 2 or 3 students per group, but the tutorial should host
    ///   basically half the class.
    ///
    ///   This allows the use of the same group list in such cases.
    /// - for some subjects, the use of groups might not be ideal and students should
    ///   be registered individually. But it might be possible to have several
    ///   students at the same time. Having group size of 1 student and several
    ///   groups at the same time can represent this situation.
    pub groups_per_interrogation: std::ops::RangeInclusive<NonZeroU32>,
    /// Duration of an interrogation in minutes
    pub duration: collomatique_time::NonZeroMinutes,
    /// This is useful when we try to limit or regulate
    /// the number of interrogations a student has in a week.
    ///
    /// This settles the question of: should we take this time into
    /// account?
    ///
    /// If set to `true`, the time will be taken into account and possibility limited.
    /// If set to `false`, this will be ignored when accounting for the total amount of time
    pub take_duration_into_account: bool,
    /// Periodicity of the interrogations.
    ///
    /// See [SubjectPeriodicity] for more details.
    pub periodicity: SubjectPeriodicity,
}

/// Periodicity information for a subject
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubjectPeriodicity {
    /// The interrogation must happen once for every block of time
    ///
    /// For instance, with a block of 2 weeks, a student must have
    /// an interrogation in the first two weeks (either on the first
    /// or second week) then a second interrogation in the next two
    /// weeks (so either on the third or forth week) but it can perfectly
    /// be on week 2 and week 3. We do not enforce a *perfect* regularity.
    OnceForEveryBlockOfWeeks {
        /// Number of weeks per block
        weeks_per_block: NonZeroU32,
        /// Minimum of weeks between two interrogations for the same student
        ///
        /// Note that `0` is not a valid possibility:
        /// because there is at most one interrogation per block, there can't
        /// be two interrogations the same week
        minimum_week_separation: NonZeroU32,
    },
    /// The interrogation must happen every week or every other week
    /// and the periodicity must be *strict*.
    ///
    /// For instance, with a periodicity of 2 weeks, a student must have
    /// an interrogation in the first two weeks (either on the first
    /// or second week) then a second interrogation in the next two
    /// weeks (so either on the third or forth week). However, if they have
    /// an interrogation on week 1, then the other one *must* be on week 3.
    /// Similarly, if they have an interrogation on week 2, the next one will
    /// be on week 4. We **do** enforce a *perfect* regularity.
    ExactlyPeriodic {
        /// Periodicity expressed in week count
        periodicity_in_weeks: NonZeroU32,
    },
    /// Fixes the total number of interrogations during the year
    ///
    /// This leaves the maximum flexibility on the placement of each
    /// interrogation. But this can lead to *very* unequal colloscopes.
    ///
    /// Apart from the total number of interrogations, we can also
    /// impose a minimum separation between two consecutive interrogations
    /// for a student.
    AmountInYear {
        /// Total number of interrogations during the year
        ///
        /// The total amount can be in a range and it is technically possible
        /// to have a minimum of zero interrogations
        interrogation_count_in_year: std::ops::RangeInclusive<u32>,
        /// Minimum of weeks between two interrogations for the same student
        ///
        /// Note that `0` is a valid possibility: it might be possible to have
        /// two interrogations during the same week!
        minimum_week_separation: u32,
    },
    /// This is a generalization of [SubjectPeriodicity::OnceForEveryBlockOfWeeks].
    ///
    /// Interrogations should happen every block but the blocks are arbitrary.
    ///
    /// This is useful for instance when we have a limited number of interrogations
    /// in the year (say 2) but the dates are not quite regular.
    ///
    /// Technically, [SubjectPeriodicity::OnceForEveryBlockOfWeeks] is a special
    /// case where the blocks start on the first week and then all have the same
    /// size. We distinguish between them for practical purposes:
    /// [SubjectPeriodicity::OnceForEveryBlockOfWeeks] is used *way* more often
    /// and can be represented in a simpler way on screen in a GUI.
    AmountForEveryArbitraryBlock {
        /// Description of the blocks that should each have a number of interrogations
        ///
        /// Blocks are in order and described by a [WeekBlock] structure.
        ///
        /// It is technically possible to have 0 blocks. This will imply that there are
        /// no interrogations for the subject which is a bit weird.
        ///
        /// It is also possible to have blocks after the end of the schedule or without
        /// any actual interrogations planned in them. But of course, no consistent
        /// colloscope will be found for this.
        blocks: Vec<WeekBlock>,
        /// Minimum of weeks between two interrogations for the same student
        ///
        /// Note that `0` is a valid possibility: it might be possible to have
        /// two interrogations during the same week!
        minimum_week_separation: u32,
    },
}

/// Description of a block of weeks for [SubjectPeriodicity::AmountForEveryArbitraryBlock]
///
/// This describes a single block of weeks that should have one interrogation.
/// There are two parameters: [WeekBlock::delay_in_weeks] is the number of weeks
/// between the previous block and the current block. It can be zero if two blocks are consecutive.
/// For the first block, this correspond to the number of weeks from the start of
/// the schedule without interrogations.
///
/// The second parameter is [WeekBlock::size_in_weeks]. This is the length of the block
/// in weeks. It cannot be zero sized: a block always has at least one week.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeekBlock {
    /// Delay (in weeks) between the previous blocks and the current one.
    ///
    /// If this is the first block, this is the delay between the start of the schedule
    /// and the first block.
    pub delay_in_weeks: u32,
    /// Number of weeks in the block.
    ///
    /// This can't be zero.
    pub size_in_weeks: NonZeroU32,
    /// Total number of interrogations that should happen during this block
    ///
    /// This is described by a range and it is technically possible
    /// The total amount can be in a range
    /// to have a minimum of zero interrogations
    pub interrogation_count_in_block: std::ops::RangeInclusive<u32>,
}

impl Default for SubjectParameters {
    fn default() -> Self {
        SubjectParameters {
            name: String::new(),
            interrogation_parameters: Some(SubjectInterrogationParameters::default()),
        }
    }
}

impl Default for SubjectInterrogationParameters {
    fn default() -> Self {
        SubjectInterrogationParameters {
            students_per_group: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            groups_per_interrogation: NonZeroU32::new(1).unwrap()..=NonZeroU32::new(1).unwrap(),
            duration: collomatique_time::NonZeroMinutes::new(60).unwrap(),
            take_duration_into_account: true,
            periodicity: SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks: NonZeroU32::new(2).unwrap(),
            },
        }
    }
}

impl Subjects {
    /// Finds the position of a subject by id
    pub fn find_subject_position(&self, id: SubjectId) -> Option<usize> {
        self.ordered_subject_list.position_of(&id)
    }

    /// Finds a subject by id
    pub fn find_subject(&self, id: SubjectId) -> Option<&Subject> {
        self.ordered_subject_list.get(&id)
    }
}

/// Errors for subject operations
///
/// These errors can be returned when trying to modify [crate::Data] with a subject op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SubjectError {
    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// The subject id already exists
    #[error("subject id ({0:?}) already exists")]
    SubjectIdAlreadyExists(SubjectId),

    /// A position is outside of bounds
    #[error("Position {0} is outside the list (size = {1})")]
    PositionOutOfBounds(usize, usize),

    /// A reference period is invalid
    #[error("Referenced period id {0:?} is invalid")]
    InvalidPeriodId(PeriodId),

    /// Invalid parameters : students per group
    #[error("Students per group range should allow at least one value")]
    StudentsPerGroupRangeIsEmpty,

    /// Invalid parameters : groups per interrogation
    #[error("Groups per interrogations range should allow at least one value")]
    GroupsPerInterrogationRangeIsEmpty,

    /// Invalid parameters : week block has empty range for interrogation count
    #[error("Interrogation count range should allow at least one value")]
    InterrogationCountRangeIsEmpty,

    /// Some non-default assignments are still present for the subject
    #[error(
        "period id ({0:?}) has non-default assignments for subject id {1:?} and cannot be removed or updated"
    )]
    SubjectStillHasNonTrivialAssignments(PeriodId, SubjectId),

    /// Some teachers still are associated to the subject
    #[error("teacher id ({0:?}) is associated to the subject id {1:?}")]
    SubjectStillHasAssociatedTeachers(TeacherId, SubjectId),

    /// The subject is referenced by a slot
    #[error("subject id ({0:?}) is referenced by slots")]
    SubjectStillHasAssociatedSlots(SubjectId),

    /// The subject is referenced by a schedule incompatibility
    #[error("subject id ({0:?}) is referenced by the incompat id {1:?}")]
    SubjectStillHasAssociatedIncompats(SubjectId, IncompatId),

    /// The subject is associated to a group list
    #[error("subject id ({0:?}) is associated to group list id {1:?} for period {2:?}")]
    SubjectStillHasAssociatedGroupList(SubjectId, GroupListId, PeriodId),

    /// The subject has filled slots in colloscope
    #[error("subject id {0:?} has a least one non-empty slot {1:?} in colloscope")]
    SubjectStillHasNonEmptySlotInColloscope(SubjectId, SlotId),

    /// The subject still has balancing options
    #[error("subject id {0:?} still has balancing options")]
    SubjectStillHasBalancingOptions(SubjectId),

    /// The subject is referenced by a pairing rule
    #[error("subject id ({0:?}) is referenced by pairing rule {1:?}")]
    SubjectIsReferencedByPairingRule(SubjectId, PairingRuleId),
}

impl crate::Data {
    /// Used internally
    ///
    /// Apply period operations
    pub(crate) fn apply_subject(
        &mut self,
        subject_op: &AnnotatedSubjectOp,
    ) -> std::result::Result<AnnotatedSubjectOp, SubjectError> {
        match subject_op {
            AnnotatedSubjectOp::AddAfter(new_id, after_id, params) => {
                if self
                    .inner_data
                    .params
                    .subjects
                    .find_subject_position(*new_id)
                    .is_some()
                {
                    return Err(SubjectError::SubjectIdAlreadyExists(*new_id));
                }
                self.inner_data.params.validate_subject(params)?;

                let position = match after_id {
                    Some(id) => {
                        self.inner_data
                            .params
                            .subjects
                            .find_subject_position(*id)
                            .ok_or(SubjectError::InvalidSubjectId(*id))?
                            + 1
                    }
                    None => 0,
                };

                self.inner_data
                    .params
                    .subjects
                    .ordered_subject_list
                    .insert_at(position, *new_id, params.clone())
                    .expect("subject id absence checked above");
                if params.parameters.interrogation_parameters.is_some() {
                    self.inner_data.params.slots.add_subject_entry(*new_id);
                }
                // A fresh subject carries no assignments, so the (sparse)
                // assignments table gets no row until a student is assigned.

                Ok(AnnotatedSubjectOp::Remove(*new_id))
            }
            AnnotatedSubjectOp::ChangePosition(id, new_pos) => {
                if *new_pos >= self.inner_data.params.subjects.ordered_subject_list.len() {
                    return Err(SubjectError::PositionOutOfBounds(
                        *new_pos,
                        self.inner_data.params.subjects.ordered_subject_list.len(),
                    ));
                }
                let Some(old_pos) = self.inner_data.params.subjects.find_subject_position(*id)
                else {
                    return Err(SubjectError::InvalidSubjectId(*id));
                };

                self.inner_data
                    .params
                    .subjects
                    .ordered_subject_list
                    .move_entry(old_pos, *new_pos);
                Ok(AnnotatedSubjectOp::ChangePosition(*id, old_pos))
            }
            AnnotatedSubjectOp::Remove(id) => {
                let Some(position) = self.inner_data.params.subjects.find_subject_position(*id)
                else {
                    return Err(SubjectError::InvalidSubjectId(*id));
                };

                if self.inner_data.params.balancing.subjects.contains(id) {
                    return Err(SubjectError::SubjectStillHasBalancingOptions(*id));
                }

                for (rule_id, rule) in self.inner_data.params.pairings.pairing_rule_map.iter() {
                    if rule.antecedent.subject_id == *id || rule.consequent.subject_id == *id {
                        return Err(SubjectError::SubjectIsReferencedByPairingRule(*id, rule_id));
                    }
                }

                for ((period_id, subject_id), group_list_id) in self
                    .inner_data
                    .params
                    .group_lists
                    .subjects_associations
                    .iter()
                {
                    if subject_id == *id {
                        return Err(SubjectError::SubjectStillHasAssociatedGroupList(
                            *id,
                            *group_list_id,
                            period_id,
                        ));
                    }
                }

                if let Some(slot_count) = self.inner_data.params.slots.slot_count_for_subject(*id)
                    && slot_count != 0
                {
                    return Err(SubjectError::SubjectStillHasAssociatedSlots(*id));
                }

                for (teacher_id, teacher) in self.inner_data.params.teachers.teacher_map.iter() {
                    if teacher.subjects.contains(id) {
                        return Err(SubjectError::SubjectStillHasAssociatedTeachers(
                            teacher_id, *id,
                        ));
                    }
                }

                for (incompat_id, incompat) in self.inner_data.params.incompats.incompat_map.iter()
                {
                    if incompat.subject_id == *id {
                        return Err(SubjectError::SubjectStillHasAssociatedIncompats(
                            *id,
                            incompat_id,
                        ));
                    }
                }

                // Under canonical-absent, a row exists iff it is non-trivial,
                // so any surviving row for this subject blocks the removal.
                if let Some((period_id, _, _)) = self
                    .inner_data
                    .params
                    .assignments
                    .iter()
                    .find(|&(_, subject_id, _)| subject_id == *id)
                {
                    return Err(SubjectError::SubjectStillHasNonTrivialAssignments(
                        period_id, *id,
                    ));
                }

                let previous_id = (position > 0).then(|| {
                    self.inner_data
                        .params
                        .subjects
                        .ordered_subject_list
                        .get_at(position - 1)
                        .expect("position > 0 checked")
                        .0
                });

                let (_, params) = self
                    .inner_data
                    .params
                    .subjects
                    .ordered_subject_list
                    .remove_at(position);
                self.inner_data.params.slots.remove_subject_entry(*id);
                // No assignment rows to drop: the guard above rejects the
                // removal while any survive.

                Ok(AnnotatedSubjectOp::AddAfter(*id, previous_id, params))
            }
            AnnotatedSubjectOp::Update(id, new_params) => {
                self.inner_data.params.validate_subject(new_params)?;
                let Some(position) = self.inner_data.params.subjects.find_subject_position(*id)
                else {
                    return Err(SubjectError::InvalidSubjectId(*id));
                };

                let old_params = self
                    .inner_data
                    .params
                    .subjects
                    .ordered_subject_list
                    .get_at(position)
                    .expect("position comes from find_subject_position")
                    .1
                    .clone();

                if old_params.parameters.interrogation_parameters.is_some()
                    && new_params.parameters.interrogation_parameters.is_none()
                {
                    if self.inner_data.params.balancing.subjects.contains(id) {
                        return Err(SubjectError::SubjectStillHasBalancingOptions(*id));
                    }

                    // The new subject does not have interrogations, let's check that no teacher has been assigned to it
                    for (teacher_id, teacher) in self.inner_data.params.teachers.teacher_map.iter()
                    {
                        if teacher.subjects.contains(id) {
                            return Err(SubjectError::SubjectStillHasAssociatedTeachers(
                                teacher_id, *id,
                            ));
                        }
                    }

                    // Also, we should not have a corresponding group list
                    for ((period_id, subject_id), group_list_id) in self
                        .inner_data
                        .params
                        .group_lists
                        .subjects_associations
                        .iter()
                    {
                        if subject_id == *id {
                            return Err(SubjectError::SubjectStillHasAssociatedGroupList(
                                *id,
                                *group_list_id,
                                period_id,
                            ));
                        }
                    }

                    // Let's also check that we don't have corresponding interrogations
                    let slot_count = self
                        .inner_data
                        .params
                        .slots
                        .slot_count_for_subject(*id)
                        .expect("Subject should have a slot list at this point");

                    if slot_count != 0 {
                        return Err(SubjectError::SubjectStillHasAssociatedSlots(*id));
                    }
                }

                for (period_id, _period) in
                    self.inner_data.params.periods.ordered_period_list.iter()
                {
                    // If the period was excluded before, there is no structure to check
                    // and if the period is not excluded now, the structure will be fine anyway
                    if old_params.excluded_periods.contains(&period_id)
                        || !new_params.excluded_periods.contains(&period_id)
                    {
                        continue;
                    }

                    // Sparse assignments: an absent row means nobody is
                    // assigned, so only a present (non-empty) row blocks the
                    // exclusion.
                    let has_assignments = self
                        .inner_data
                        .params
                        .assignments
                        .students(period_id, *id)
                        .is_some_and(|students| !students.is_empty());

                    if has_assignments {
                        return Err(SubjectError::SubjectStillHasNonTrivialAssignments(
                            period_id, *id,
                        ));
                    }

                    if let Some(group_list_id) = self
                        .inner_data
                        .params
                        .group_lists
                        .subjects_associations
                        .get(&(period_id, *id))
                    {
                        return Err(SubjectError::SubjectStillHasAssociatedGroupList(
                            *id,
                            *group_list_id,
                            period_id,
                        ));
                    }

                    // Check if there are non-empty slots in colloscope for the subject
                    if let Some(subject_slots) = self.inner_data.params.slots.slots_for_subject(*id)
                    {
                        let colloscope_period = self
                            .inner_data
                            .colloscope
                            .period_map
                            .get(&period_id)
                            .expect("Period ID should be valid at this point");

                        for (slot_id, _slot) in subject_slots {
                            let Some(collo_slot) = colloscope_period.slot_map.get(slot_id) else {
                                continue;
                            };
                            if !collo_slot.is_empty() {
                                return Err(SubjectError::SubjectStillHasNonEmptySlotInColloscope(
                                    *id, *slot_id,
                                ));
                            }
                        }
                    }
                }

                self.inner_data
                    .params
                    .subjects
                    .ordered_subject_list
                    .replace_value_at(position, new_params.clone());
                if new_params.parameters.interrogation_parameters.is_some()
                    != old_params.parameters.interrogation_parameters.is_some()
                {
                    if new_params.parameters.interrogation_parameters.is_some() {
                        // We don't need to update the colloscope in this case: no slots have been added so far
                        self.inner_data.params.slots.add_subject_entry(*id);
                    } else {
                        // We don't need to update the colloscope in this case: all slots have already been removed
                        self.inner_data.params.slots.remove_subject_entry(*id);
                    }
                }

                // Let's update the colloscope.
                // However, if there are no interrogations, then we don't have slots to update
                if new_params.parameters.interrogation_parameters.is_some() {
                    // Snapshot the slot ids so the params borrow does not overlap the
                    // mutable colloscope borrow below.
                    let slot_ids: Vec<SlotId> = self
                        .inner_data
                        .params
                        .slots
                        .slots_for_subject(*id)
                        .expect("Subject should have a slot list at this point")
                        .map(|(slot_id, _slot)| *slot_id)
                        .collect();

                    for (period_id, collo_period) in &mut self.inner_data.colloscope.period_map {
                        // Only change in period status should be considered
                        if old_params.excluded_periods.contains(period_id)
                            == new_params.excluded_periods.contains(period_id)
                        {
                            continue;
                        }

                        if old_params.excluded_periods.contains(period_id) {
                            // The period was excluded but is not anymore
                            for slot_id in &slot_ids {
                                collo_period.slot_map.insert(
                                    *slot_id,
                                    colloscopes::ColloscopeSlot::new_empty_from_params(
                                        &self.inner_data.params,
                                        *period_id,
                                        *slot_id,
                                    ),
                                );
                            }
                        } else {
                            // The period was included but will now be excluded
                            for slot_id in &slot_ids {
                                collo_period.slot_map.remove(slot_id);
                            }
                        }
                    }
                }

                // Sparse assignments need no fan-out on an exclusion change: a
                // newly-included period starts with no row, and a newly-excluded
                // period has none either (the guard above rejects the update
                // while any survive).

                Ok(AnnotatedSubjectOp::Update(*id, old_params))
            }
        }
    }
}
