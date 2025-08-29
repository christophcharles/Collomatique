//! subjects submodule
//!
//! This module defines the subjects entry for the JSON description
//!
use super::*;

use std::collections::BTreeSet;
use std::num::{NonZeroU32, NonZeroUsize};

/// JSON desc of subjects
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct List {
    /// ordered subject list
    ///
    /// each subject is described by an id (which should not
    /// be duplicate) and a structure [Subject]
    pub ordered_subject_list: Vec<(u64, Subject)>,
}

/// JSON desc of a single subject
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subject {
    pub parameters: SubjectParameters,
    pub excluded_periods: BTreeSet<u64>,
}

impl From<&collomatique_state_colloscopes::Subject> for Subject {
    fn from(value: &collomatique_state_colloscopes::Subject) -> Self {
        Subject {
            parameters: value.parameters.clone().into(),
            excluded_periods: value.excluded_periods.iter().map(|x| x.inner()).collect(),
        }
    }
}

impl From<Subject> for collomatique_state_colloscopes::subjects::SubjectExternalData {
    fn from(value: Subject) -> Self {
        collomatique_state_colloscopes::subjects::SubjectExternalData {
            parameters: value.parameters.into(),
            excluded_periods: value.excluded_periods,
        }
    }
}

/// JSON desc of a single subject parameters
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectParameters {
    pub name: String,
    pub students_per_group: std::ops::RangeInclusive<NonZeroU32>,
    pub groups_per_interrogation: std::ops::RangeInclusive<NonZeroU32>,
    pub duration: NonZeroU32,
    pub take_duration_into_account: bool,
    pub periodicity: SubjectPeriodicity,
}

impl From<collomatique_state_colloscopes::SubjectParameters> for SubjectParameters {
    fn from(value: collomatique_state_colloscopes::SubjectParameters) -> Self {
        SubjectParameters {
            name: value.name,
            students_per_group: value.students_per_group,
            groups_per_interrogation: value.groups_per_interrogation,
            duration: value.duration.get(),
            take_duration_into_account: value.take_duration_into_account,
            periodicity: value.periodicity.into(),
        }
    }
}

impl From<SubjectParameters> for collomatique_state_colloscopes::SubjectParameters {
    fn from(value: SubjectParameters) -> Self {
        collomatique_state_colloscopes::SubjectParameters {
            name: value.name,
            students_per_group: value.students_per_group,
            groups_per_interrogation: value.groups_per_interrogation,
            duration: value.duration.into(),
            take_duration_into_account: value.take_duration_into_account,
            periodicity: value.periodicity.into(),
        }
    }
}

/// JSON desc of a subject periodicity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubjectPeriodicity {
    OnceForEveryBlockOfWeeks {
        weeks_per_block: u32,
    },
    ExactlyPeriodic {
        periodicity_in_weeks: NonZeroU32,
    },
    AmountInYear {
        interrogation_count_in_year: std::ops::RangeInclusive<u32>,
        minimum_week_separation: u32,
    },
    OnceForEveryArbitraryBlock {
        blocks: Vec<WeekBlock>,
    },
}

impl From<collomatique_state_colloscopes::SubjectPeriodicity> for SubjectPeriodicity {
    fn from(value: collomatique_state_colloscopes::SubjectPeriodicity) -> Self {
        match value {
            collomatique_state_colloscopes::SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                weeks_per_block,
            } => SubjectPeriodicity::OnceForEveryBlockOfWeeks { weeks_per_block },
            collomatique_state_colloscopes::SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks,
            } => SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks,
            },
            collomatique_state_colloscopes::SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year,
                minimum_week_separation,
            } => SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year,
                minimum_week_separation,
            },
            collomatique_state_colloscopes::SubjectPeriodicity::OnceForEveryArbitraryBlock {
                blocks,
            } => SubjectPeriodicity::OnceForEveryArbitraryBlock {
                blocks: blocks.into_iter().map(|b| b.into()).collect(),
            },
        }
    }
}

impl From<SubjectPeriodicity> for collomatique_state_colloscopes::SubjectPeriodicity {
    fn from(value: SubjectPeriodicity) -> Self {
        match value {
            SubjectPeriodicity::OnceForEveryBlockOfWeeks { weeks_per_block } => {
                collomatique_state_colloscopes::SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                    weeks_per_block,
                }
            }
            SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks,
            } => collomatique_state_colloscopes::SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks,
            },
            SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year,
                minimum_week_separation,
            } => collomatique_state_colloscopes::SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year,
                minimum_week_separation,
            },
            SubjectPeriodicity::OnceForEveryArbitraryBlock { blocks } => {
                collomatique_state_colloscopes::SubjectPeriodicity::OnceForEveryArbitraryBlock {
                    blocks: blocks.into_iter().map(|b| b.into()).collect(),
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeekBlock {
    pub delay: usize,
    pub size: NonZeroUsize,
}

impl From<collomatique_state_colloscopes::subjects::WeekBlock> for WeekBlock {
    fn from(value: collomatique_state_colloscopes::subjects::WeekBlock) -> Self {
        WeekBlock {
            delay: value.delay_in_weeks,
            size: value.size_in_weeks,
        }
    }
}

impl From<WeekBlock> for collomatique_state_colloscopes::subjects::WeekBlock {
    fn from(value: WeekBlock) -> Self {
        collomatique_state_colloscopes::subjects::WeekBlock {
            delay_in_weeks: value.delay,
            size_in_weeks: value.size,
        }
    }
}
