//! Subjects submodule
//!
//! This module defines the relevant types to describes the subjects

use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, num::NonZeroU32};
use thiserror::Error;

use collomatique_state::{Join, References};

use crate::OrderedTable;
use crate::ids::{
    GroupListId, IncompatId, NewId, PairingRuleId, PeriodId, SlotId, SubjectId, TeacherId,
};
use crate::non_empty_range::NonEmptyRangeInclusive;
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
    pub students_per_group: NonEmptyRangeInclusive<NonZeroU32>,
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
    pub groups_per_interrogation: NonEmptyRangeInclusive<NonZeroU32>,
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
        interrogation_count_in_year: NonEmptyRangeInclusive<u32>,
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
    pub interrogation_count_in_block: NonEmptyRangeInclusive<u32>,
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
            students_per_group: NonEmptyRangeInclusive::new(
                NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            )
            .expect("statically non-empty"),
            groups_per_interrogation: NonEmptyRangeInclusive::new(
                NonZeroU32::new(1).unwrap()..=NonZeroU32::new(1).unwrap(),
            )
            .expect("statically non-empty"),
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

/// Precondition errors of the forced subject ops — the carve-out subset
/// (step-3 survey Table 2). Kept: no-clobber, op-target existence + `AddAfter`
/// anchor ([Self::InvalidSubjectId]), and position bounds. `validate_subject`,
/// the Remove reference scans, the interrogations-off guards and the
/// newly-excluded-period guards are stripped. Variants copied verbatim from
/// [SubjectError].
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SubjectPrecheckError {
    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// The subject id already exists
    #[error("subject id ({0:?}) already exists")]
    SubjectIdAlreadyExists(SubjectId),

    /// A position is outside of bounds
    #[error("Position {0} is outside the list (size = {1})")]
    PositionOutOfBounds(usize, usize),
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
                // Sparse ordering: a fresh subject gets no slots row until its
                // first slot is added, so nothing to register here. Likewise
                // the sparse assignments table gets no row until a student is
                // assigned.

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
                    if rule.antecedent().subject_id == *id || rule.consequent().subject_id == *id {
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
                // Sparse ordering: a removable subject has no slots (the
                // associated-slots guard above blocks otherwise), so it has no
                // ordering row to drop. No assignment rows to drop either: the
                // guard above rejects the removal while any survive.

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

                    // Let's also check that we don't have corresponding interrogations.
                    // Sparse ordering: an absent row means no slots.
                    let slot_count = self
                        .inner_data
                        .params
                        .slots
                        .slot_count_for_subject(*id)
                        .unwrap_or(0);

                    if slot_count != 0 {
                        return Err(SubjectError::SubjectStillHasAssociatedSlots(*id));
                    }
                }

                for period_id in self.inner_data.params.periods.period_ids() {
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

                    // Check if there are non-empty slots in colloscope for the
                    // subject on the newly-excluded period. Canonical-absent
                    // surface: a slot blocks the exclusion iff it holds an
                    // interrogation row on a week of this period.
                    if let Some(subject_slots) = self.inner_data.params.slots.slots_for_subject(*id)
                    {
                        let weeks = &self.inner_data.params.weeks;
                        for (slot_id, _slot) in subject_slots {
                            let has_row = self
                                .inner_data
                                .colloscope
                                .interrogations_for_slot(*slot_id)
                                .any(|(week, _groups)| {
                                    weeks.week_position(week).map(|(p, _pos)| p) == Some(period_id)
                                });
                            if has_row {
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
                // Sparse ordering: gaining interrogations creates no row (the
                // first slot does that lazily); losing interrogations requires
                // no slots (guarded above), so no row exists to drop.

                // The colloscope needs no fan-out on an exclusion change either.
                // Its rows key on `(slot, week)`: a newly-included period starts
                // with no rows (an absent row is an empty cell, and future writes
                // are gated on `is_interrogation_possible`), and a newly-excluded
                // period has none (the guard above rejects the update while any
                // survive). Sparse assignments are handled the same way.

                Ok(AnnotatedSubjectOp::Update(*id, old_params))
            }
        }
    }

    /// Used internally by [crate::Data::force_apply]
    ///
    /// Thin copy of [Self::apply_subject]: carve-out guards kept (returned as
    /// [SubjectPrecheckError] — no-clobber, target existence, `AddAfter` anchor,
    /// position bounds), invariant guards stripped (step-3 survey Table 1). May
    /// leave the state invalid; the caller owns checking and rollback.
    pub(crate) fn force_apply_subject(
        &mut self,
        subject_op: &AnnotatedSubjectOp,
    ) -> std::result::Result<AnnotatedSubjectOp, SubjectPrecheckError> {
        match subject_op {
            AnnotatedSubjectOp::AddAfter(new_id, after_id, params) => {
                if self
                    .inner_data
                    .params
                    .subjects
                    .find_subject_position(*new_id)
                    .is_some()
                {
                    return Err(SubjectPrecheckError::SubjectIdAlreadyExists(*new_id));
                }
                // stripped: validate_subject

                let position = match after_id {
                    Some(id) => {
                        self.inner_data
                            .params
                            .subjects
                            .find_subject_position(*id)
                            .ok_or(SubjectPrecheckError::InvalidSubjectId(*id))?
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

                Ok(AnnotatedSubjectOp::Remove(*new_id))
            }
            AnnotatedSubjectOp::ChangePosition(id, new_pos) => {
                if *new_pos >= self.inner_data.params.subjects.ordered_subject_list.len() {
                    return Err(SubjectPrecheckError::PositionOutOfBounds(
                        *new_pos,
                        self.inner_data.params.subjects.ordered_subject_list.len(),
                    ));
                }
                let Some(old_pos) = self.inner_data.params.subjects.find_subject_position(*id)
                else {
                    return Err(SubjectPrecheckError::InvalidSubjectId(*id));
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
                    return Err(SubjectPrecheckError::InvalidSubjectId(*id));
                };

                // stripped: balancing / pairing / association / slot / teacher /
                // incompat / assignment reference scans

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

                Ok(AnnotatedSubjectOp::AddAfter(*id, previous_id, params))
            }
            AnnotatedSubjectOp::Update(id, new_params) => {
                // stripped: validate_subject
                let Some(position) = self.inner_data.params.subjects.find_subject_position(*id)
                else {
                    return Err(SubjectPrecheckError::InvalidSubjectId(*id));
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

                // stripped: interrogations-off guards + newly-excluded-period guards

                self.inner_data
                    .params
                    .subjects
                    .ordered_subject_list
                    .replace_value_at(position, new_params.clone());

                Ok(AnnotatedSubjectOp::Update(*id, old_params))
            }
        }
    }
}
