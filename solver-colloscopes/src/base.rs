//! Definition of relevant structures to describe colloscopes
//!
//! This module contains the basic colloscope structures.
//! The submodule [solution] defines how solved colloscopes are represented.
//!
//! The main struct is [ColloscopeProblem] which describes the various (base) constraints
//! that a colloscope is subject to.

pub mod solution;
pub mod variables;

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;
use std::ops::RangeInclusive;

pub trait Identifier:
    Clone + Copy + std::fmt::Debug + Ord + PartialOrd + Eq + PartialEq + Send + Sync
{
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupAssignment<GroupListId: Identifier, StudentId: Identifier> {
    group_list_id: GroupListId,
    enrolled_students: BTreeSet<StudentId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatedGroupAssignment<GroupListId: Identifier, StudentId: Identifier> {
    start_week: NonZeroUsize,
    group_assignment: GroupAssignment<GroupListId, StudentId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupAssignments<GroupListId: Identifier, StudentId: Identifier> {
    starting_group_assignment: GroupAssignment<GroupListId, StudentId>,
    other_group_assignments: Vec<DatedGroupAssignment<GroupListId, StudentId>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupListDescription<StudentId: Identifier> {
    students_per_group: RangeInclusive<NonZeroUsize>,
    group_count: RangeInclusive<usize>,
    students: BTreeSet<StudentId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterrogationDescription {
    slot_start: collomatique_time::SlotStart,
    weeks: BTreeSet<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeacherInterrogationDescriptions<InterrogationId: Identifier> {
    interrogation_descriptions: BTreeMap<InterrogationId, InterrogationDescription>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectDescription<
    TeacherId: Identifier,
    InterrogationId: Identifier,
    GroupListId: Identifier,
    StudentId: Identifier,
> {
    duration: collomatique_time::NonZeroDurationInMinutes,
    students_per_group: RangeInclusive<NonZeroUsize>,
    groups_per_interrogation: RangeInclusive<NonZeroUsize>,
    period: NonZeroUsize,
    teacher_map: BTreeMap<TeacherId, TeacherInterrogationDescriptions<InterrogationId>>,
    group_assignments: GroupAssignments<GroupListId, StudentId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColloscopeProblem<
    SubjectId: Identifier,
    TeacherId: Identifier,
    InterrogationId: Identifier,
    GroupListId: Identifier,
    StudentId: Identifier,
> {
    week_count: usize,
    subject_descriptions:
        BTreeMap<SubjectId, SubjectDescription<TeacherId, InterrogationId, GroupListId, StudentId>>,
    group_list_descriptions: BTreeMap<GroupListId, GroupListDescription<StudentId>>,
}
