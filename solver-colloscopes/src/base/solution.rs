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
    group_count: usize,
    assigned_students: BTreeMap<StudentId, usize>,
    unassigned_students: BTreeSet<StudentId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interrogation {
    assigned_groups: BTreeSet<usize>,
    unassigned_groups: BTreeSet<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterrogationSlot {
    slot_start: collomatique_time::SlotStart,
    interrogations: Vec<Option<Interrogation>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeacherInterrogations<InterrogationId: Identifier> {
    interrogation_slots: BTreeMap<InterrogationId, InterrogationSlot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectInterrogations<
    TeacherId: Identifier,
    InterrogationId: Identifier,
    GroupListId: Identifier,
    StudentId: Identifier,
> {
    duration: collomatique_time::NonZeroDurationInMinutes,
    group_assignments: super::GroupAssignments<GroupListId, StudentId>,
    teacher_map: BTreeMap<TeacherId, TeacherInterrogations<InterrogationId>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Colloscope<
    SubjectId: Identifier,
    TeacherId: Identifier,
    InterrogationId: Identifier,
    GroupListId: Identifier,
    StudentId: Identifier,
> {
    week_count: usize,
    subject_map: BTreeMap<
        SubjectId,
        SubjectInterrogations<TeacherId, InterrogationId, GroupListId, StudentId>,
    >,
    group_lists: BTreeMap<GroupListId, GroupList<StudentId>>,
}
