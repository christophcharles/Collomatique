//! Solution submodule of [crate::base].
//!
//! This submodule defines the various types to describe a colloscope.
//!
//! The main such structure is [GroupListSolution] which describes
//! a (partially completed or not) group list.
use std::collections::BTreeMap;

use super::Identifier;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupList<StudentId: Identifier> {
    pub student_map: BTreeMap<StudentId, u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupListsForSubject<PeriodId: Identifier, StudentId: Identifier> {
    pub period_map: BTreeMap<PeriodId, GroupList<StudentId>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupListSolution<PeriodId: Identifier, SubjectId: Identifier, StudentId: Identifier> {
    pub subject_map: BTreeMap<SubjectId, GroupListsForSubject<PeriodId, StudentId>>,
}
