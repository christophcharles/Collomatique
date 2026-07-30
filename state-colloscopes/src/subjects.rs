//! Subjects submodule
//!
//! This module defines the relevant types to describes the subjects

use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, num::NonZeroU32};
use thiserror::Error;

use collomatique_state::{ContentOrd, Join, References};

use crate::OrderedTable;
use crate::ids::{NewId, PeriodId, SubjectId};
use crate::non_empty_range::NonEmptyRangeInclusive;
use crate::ops::AnnotatedSubjectOp;

/// Description of the subjects
#[derive(Clone, Debug, Default, PartialEq, Eq, ContentOrd)]
pub struct Subjects {
    /// Ordered list of subjects
    ///
    /// Each item represent a subject. It is described
    /// by a unique id and a description of type [Subject]
    pub ordered_subject_list: OrderedTable<SubjectId, Subject>,
}

/// Description of one subject
#[derive(
    Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, References, Join, ContentOrd,
)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ContentOrd)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ContentOrd)]
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
    // A scalar leaf whose type is foreign: same duration or incomparable.
    // Shortening an interrogation is a change of value, not a removal.
    #[ord(atom)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ContentOrd)]
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
        // The block list is *relational*, not a collection of independent
        // items: each block's `delay_in_weeks` is measured from the previous
        // block, so dropping or truncating blocks re-dates every block after
        // it. The chain is therefore one composite value — an atom (plan
        // step 6.5, decision 10). Even a strict truncation is incomparable,
        // not below.
        #[ord(atom)]
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

/// Precondition errors of the forced subject ops — the carve-out subset
/// (step-3 survey Table 2). Kept: no-clobber, op-target existence + `AddAfter`
/// anchor ([Self::InvalidSubjectId]), and position bounds. `validate_subject`,
/// the Remove reference scans, the interrogations-off guards and the
/// newly-excluded-period guards are stripped.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SubjectPrecheckError {
    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// The subject id already exists
    #[error("subject id ({0:?}) already exists")]
    SubjectIdAlreadyExists(SubjectId),

    /// A position is outside the subject list
    #[error("position {position} is outside the list (size = {size})")]
    PositionOutOfBounds { position: usize, size: usize },
}

impl crate::Data {
    /// Used internally by [crate::Data::force_apply]
    ///
    /// Force-applies a subject op: carve-out guards kept (returned as
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
                // Target existence before bounds (the slots/weeks order): a
                // doubly-bad op reports its dangling target, not the position.
                let Some(old_pos) = self.inner_data.params.subjects.find_subject_position(*id)
                else {
                    return Err(SubjectPrecheckError::InvalidSubjectId(*id));
                };
                let size = self.inner_data.params.subjects.ordered_subject_list.len();
                if *new_pos >= size {
                    return Err(SubjectPrecheckError::PositionOutOfBounds {
                        position: *new_pos,
                        size,
                    });
                }

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
