//! Solution submodule of [crate::base].
//!
//! This submodule defines the various types to describe a colloscope.
//!
//! The main such structure is [Colloscope] which describes
//! a (partially completed or not) colloscope.
use std::collections::BTreeMap;

use super::Identifier;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupList<StudentId: Identifier> {
    pub student_map: BTreeMap<StudentId, u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupListsForPeriod<SubjectId: Identifier, StudentId: Identifier> {
    pub group_lists: BTreeMap<SubjectId, GroupList<StudentId>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupListSolution<PeriodId: Identifier, SubjectId: Identifier, StudentId: Identifier> {
    pub period_map: BTreeMap<PeriodId, GroupListsForPeriod<SubjectId, StudentId>>,
}
