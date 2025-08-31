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
use std::num::NonZeroU32;
use std::ops::RangeInclusive;

pub trait Identifier:
    Clone + Copy + std::fmt::Debug + Ord + PartialOrd + Eq + PartialEq + Send + Sync
{
}

impl<T: Clone + Copy + std::fmt::Debug + Ord + PartialOrd + Eq + PartialEq + Send + Sync> Identifier
    for T
{
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupAssignment<GroupListId: Identifier, StudentId: Identifier> {
    group_list_id: GroupListId,
    enrolled_students: BTreeSet<StudentId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatedGroupAssignment<GroupListId: Identifier, StudentId: Identifier> {
    start_week: NonZeroU32,
    group_assignment: GroupAssignment<GroupListId, StudentId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupAssignments<GroupListId: Identifier, StudentId: Identifier> {
    starting_group_assignment: GroupAssignment<GroupListId, StudentId>,
    other_group_assignments: Vec<DatedGroupAssignment<GroupListId, StudentId>>,
}

/// Description of an interrogation slot
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotDescription {
    /// Start day and time
    ///
    /// The duration is provided by the subject
    slot_start: collomatique_time::SlotStart,

    /// Weeks the slot is valid
    weeks: Vec<bool>,
}

/// Description of the constraints for a subject
///
/// Actually a type of constraint is missing: the
/// periodicity of the subject interrogations.
///
/// This is described in its own independant structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectDescription<SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier> {
    /// Duration of each slot for the subject
    duration: collomatique_time::NonZeroDurationInMinutes,
    /// How many students per group (range)
    ///
    /// This is not redundant with the constraints from [GroupListDescription]
    /// because in a given group from a group list some students might not follow
    /// the subject. So only a subgroup might be present.
    students_per_group: RangeInclusive<NonZeroU32>,
    /// How many groups should be in a given slot
    ///
    /// This is useful in particular for tutorial session that happen
    /// with half of the classroom and the other half can have interrogations.
    ///
    /// In that case, the tutorial can be included in the colloscope and
    /// the students attending to the tutorial are selected by groups.
    /// If we know what group list is used by the interrogations that happen
    /// during the tutorial, this is usually *way faster* to solve.
    ///
    /// If it is not known if a specific group list is used, this can still be
    /// useful by using a group list with groups of one student each. This then allows
    /// multiple students to be dispatched to the tutorial.
    ///
    /// This also allows, for the same reason, changing groups for standard
    /// interrogation if this is desired for some reason.
    groups_per_interrogation: RangeInclusive<NonZeroU32>,
    /// Description of each interrogation slot for the subject
    slots_descriptions: BTreeMap<SlotId, SlotDescription>,
    /// Group lists to use for each period and what students are attending the subject
    /// for each periods
    group_assignments: GroupAssignments<GroupListId, StudentId>,
}

/// Description of a group list
///
/// This is only the general constraints on the group list.
/// There is no partial solution, it just describes what a
/// valid group list is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupListDescription<StudentId: Identifier> {
    /// Students count per group (range)
    students_per_group: RangeInclusive<NonZeroU32>,
    /// Number of groups in the list (range)
    group_count: RangeInclusive<u32>,
    /// Students to dispatch in the groups
    students: BTreeSet<StudentId>,
}

/// Description of a colloscope problem - reduced
///
/// This structure will describe a general colloscope problem.
/// The goal is just to describe the broad lines. Some finer constraints
/// might need additional information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColloscopeProblem<
    SubjectId: Identifier,
    SlotId: Identifier,
    GroupListId: Identifier,
    StudentId: Identifier,
> {
    /// List of every subject that appear in the colloscope along with a description
    subject_descriptions: BTreeMap<SubjectId, SubjectDescription<SlotId, GroupListId, StudentId>>,
    /// List of group lists constraints
    group_list_descriptions: BTreeMap<GroupListId, GroupListDescription<StudentId>>,
}
