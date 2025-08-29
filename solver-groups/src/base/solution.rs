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
    group_count: u32,
    student_map: BTreeMap<StudentId, u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupLists<
    SubjectId: Identifier,
    StudentId: Identifier,
> {
    group_lists: BTreeMap<SubjectId, GroupList<StudentId>>,
}
