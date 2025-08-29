//! Definition of relevant structures to describe colloscopes
//! 
//! This module contains the basic colloscope structures.
//! The main such structure is [Colloscope] which describes
//! a (partially completed or not) colloscope.

use std::collections::{HashMap, HashSet, BTreeSet};
use std::num::NonZeroUsize;

pub trait Identifier : Clone + Copy + std::fmt::Debug + Ord + PartialOrd + Eq + PartialEq + std::hash::Hash + Send + Sync {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupList<StudentId: Identifier> {
    group_count: usize,
    assigned_students: HashMap<StudentId, usize>,
    unassigned_students: HashSet<StudentId>,
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
    interrogation_slots: HashMap<InterrogationId, InterrogationSlot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectInterrogations<TeacherId: Identifier, InterrogationId: Identifier, GroupListId: Identifier> {
    duration: collomatique_time::NonZeroDurationInMinutes,
    group_list_id: GroupListId,
    teacher_map: HashMap<TeacherId, TeacherInterrogations<InterrogationId>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Colloscope<SubjectId: Identifier, TeacherId: Identifier, InterrogationId: Identifier, GroupListId: Identifier, StudentId: Identifier> {
    week_count: usize,
    subject_map: HashMap<SubjectId, SubjectInterrogations<TeacherId, InterrogationId, GroupListId>>,
    group_lists: HashMap<GroupListId, GroupList<StudentId>>,
}

