//! Solution submodule of [crate::base].
//!
//! This submodule defines the various types to describe a colloscope.
//!
//! The main such structure is [Colloscope] which describes
//! a (partially completed or not) colloscope.
use std::collections::{BTreeMap, BTreeSet};

use super::Identifier;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupList<StudentId: Identifier> {
    group_count: u32,
    assigned_students: BTreeMap<StudentId, u32>,
    unassigned_students: BTreeSet<StudentId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interrogation {
    assigned_groups: BTreeSet<u32>,
    unassigned_groups: BTreeSet<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectInterrogations<SlotId: Identifier> {
    slots: BTreeMap<SlotId, Vec<Option<Interrogation>>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Colloscope<
    SubjectId: Identifier,
    SlotId: Identifier,
    GroupListId: Identifier,
    StudentId: Identifier,
> {
    subject_map: BTreeMap<SubjectId, SubjectInterrogations<SlotId>>,
    group_lists: BTreeMap<GroupListId, GroupList<StudentId>>,
}
