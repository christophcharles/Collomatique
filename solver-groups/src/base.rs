//! Definition of relevant structures to describe group lists

pub mod solution;
pub mod variables;

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use std::ops::RangeInclusive;

pub trait Identifier:
    Clone + Copy + std::fmt::Debug + Ord + PartialOrd + Eq + PartialEq + Send + Sync
{
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectDescription<StudentId: Identifier> {
    students: BTreeSet<StudentId>,
    group_count: RangeInclusive<u32>,
    students_per_group: RangeInclusive<NonZeroU32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupListProblem<SubjectId: Identifier, StudentId: Identifier> {
    subject_descriptions: BTreeMap<SubjectId, SubjectDescription<StudentId>>,
}
