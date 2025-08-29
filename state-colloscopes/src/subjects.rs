//! Subjects submodule
//!
//! This module defines the relevant types to describes the subjects

use std::{collections::BTreeSet, num::NonZeroU32};

use crate::ids::{PeriodId, SubjectId};

/// Description of the subjects
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Subjects {
    /// Ordered list of subjects
    ///
    /// Each item represent a subject. It is described
    /// by a unique id and a description of type [Subject]
    pub ordered_period_list: Vec<(SubjectId, Subject)>,
}

/// Description of one subject
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Subject {
    /// Parameters for the subject
    ///
    /// This is separated because those parameters do
    /// not need to be checked
    pub parameters: SubjectParameters,
    /// Periods that should not be covered by the subject
    ///
    /// By default a subject is present for every period.
    pub excluded_periods: BTreeSet<PeriodId>,
}

/// Description of one subject
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectParameters {
    /// Name of the subject
    ///
    /// This is just a descriptive string
    pub name: String,
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
    pub duration: collomatique_time::NonZeroDurationInMinutes,
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
        weeks_per_block: u32,
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
        interrogation_count_in_year: bool,
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
    OnceForEveryArbitraryBlock {
        /// Dates that separate blocks
        ///
        /// If this list is empty, there will be a single block that
        /// starts at the first week of the first period and will end at the
        /// last week of the last period.
        ///
        /// Here, we can split this single blocks by giving other dates that
        /// separate them. So there always is `dates_between_blocks.len()+1` blocks.
        dates_between_blocks: BTreeSet<collomatique_time::NaiveMondayDate>,
    },
}

impl Default for SubjectParameters {
    fn default() -> Self {
        SubjectParameters {
            name: String::new(),
            students_per_group: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            groups_per_interrogation: NonZeroU32::new(1).unwrap()..=NonZeroU32::new(1).unwrap(),
            duration: collomatique_time::NonZeroDurationInMinutes::new(60).unwrap(),
            take_duration_into_account: true,
            periodicity: SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks: NonZeroU32::new(2).unwrap(),
            },
        }
    }
}

impl Subject {
    /// Builds a subject from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [SubjectExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: SubjectExternalData) -> Subject {
        Subject {
            parameters: external_data.parameters,
            excluded_periods: external_data
                .excluded_periods
                .into_iter()
                .map(|x| unsafe { PeriodId::new(x) })
                .collect(),
        }
    }
}

impl Subjects {
    /// Builds subjects from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [SubjectsExternalData::validate_all] and [SubjectExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: SubjectsExternalData) -> Subjects {
        Subjects {
            ordered_period_list: external_data
                .ordered_period_list
                .into_iter()
                .map(|(id, data)| {
                    (unsafe { SubjectId::new(id) }, unsafe {
                        Subject::from_external_data(data)
                    })
                })
                .collect(),
        }
    }

    /// Finds the position of a subject by id
    pub fn find_subject_position(&self, id: SubjectId) -> Option<usize> {
        self.ordered_period_list
            .iter()
            .position(|(current_id, _desc)| *current_id == id)
    }
}

/// Description of the subjects but unchecked
///
/// This structure is an unchecked equivalent of [Subjects].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SubjectsExternalData {
    /// Ordered list of subjects
    ///
    /// Each item represent a subject. It is described
    /// by what should be a unique id and a description of type [SubjectExternalData]
    pub ordered_period_list: Vec<(u64, SubjectExternalData)>,
}

impl SubjectsExternalData {
    /// Checks the validity of all [SubjectExternalData] in the ordered list.
    ///
    /// In practice, this means checking that the ids for periods are valid
    ///
    /// **Beware**, this does not check the validity of the ids for the subjects!
    pub fn validate_all(&self, period_ids: &BTreeSet<u64>) -> bool {
        self.ordered_period_list
            .iter()
            .all(|(_id, data)| data.validate(period_ids))
    }
}

/// Description of one subject but unchecked
///
/// This structure is an unchecked equivalent of [Subject].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SubjectExternalData {
    /// Parameters for the subject
    ///
    /// This is separated because those parameters do
    /// not need to be checked
    pub parameters: SubjectParameters,
    /// Periods that should not be covered by the subject
    ///
    /// By default a subject is present for every period.
    /// Ids that appear here should be period ids.
    pub excluded_periods: BTreeSet<u64>,
}

impl SubjectExternalData {
    /// Checks the validity of a [SubjectExternalData].
    ///
    /// In practice, this means checking that the ids for periods are valid
    pub fn validate(&self, period_ids: &BTreeSet<u64>) -> bool {
        self.excluded_periods.iter().all(|x| period_ids.contains(x))
    }
}
