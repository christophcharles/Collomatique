//! Definition of relevant structures to describe colloscopes
//! 
//! This module contains the basic colloscope structures.
//! The main such structure is [Colloscope] which describes
//! a (partially completed or not) colloscope.

use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;

pub trait Identifier : Clone + Copy + std::fmt::Debug + Ord + PartialOrd + Eq + PartialEq + std::hash::Hash + Send + Sync {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupList<StudentId: Identifier> {
    students_per_group: std::ops::RangeInclusive<usize>,
    groups: Vec<HashSet<StudentId>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeacherInterrogations {

}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectInterrogations<TeacherId: Identifier, GroupListId: Identifier> {
    duration: collomatique_time::NonZeroDurationInMinutes,
    period: NonZeroUsize,
    group_list_id: GroupListId,
    teacher_map: HashMap<TeacherId, TeacherInterrogations>
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Colloscope<SubjectId: Identifier, TeacherId: Identifier, GroupListId: Identifier, StudentId: Identifier> {
    subject_map: HashMap<SubjectId, SubjectInterrogations<TeacherId, GroupListId>>,
    group_lists: HashMap<GroupListId, GroupList<StudentId>>,
}

