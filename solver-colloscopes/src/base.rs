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
    Clone + Copy + std::fmt::Debug + Ord + PartialOrd + Eq + PartialEq + Send + Sync + 'static
{
}

impl<
        T: Clone + Copy + std::fmt::Debug + Ord + PartialOrd + Eq + PartialEq + Send + Sync + 'static,
    > Identifier for T
{
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupAssignment<GroupListId: Identifier, StudentId: Identifier> {
    pub group_list_id: GroupListId,
    pub enrolled_students: BTreeSet<StudentId>,
}

/// Description of an interrogation slot
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotDescription {
    /// Start day and time
    ///
    /// The duration is provided by the subject
    pub slot_start: collomatique_time::SlotStart,

    /// Weeks the slot is valid
    pub weeks: Vec<bool>,
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
    pub duration: collomatique_time::NonZeroDurationInMinutes,
    /// How many students per group (range)
    ///
    /// This is not redundant with the constraints from [GroupListDescription]
    /// because in a given group from a group list some students might not follow
    /// the subject. So only a subgroup might be present.
    pub students_per_group: RangeInclusive<NonZeroU32>,
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
    pub groups_per_interrogation: RangeInclusive<NonZeroU32>,
    /// Description of each interrogation slot for the subject
    pub slots_descriptions: BTreeMap<SlotId, SlotDescription>,
    /// Group lists to use for each week and what students are attending the subject
    pub group_assignments: Vec<Option<GroupAssignment<GroupListId, StudentId>>>,
}

/// Description of a group list
///
/// This is only the general constraints on the group list.
/// There is no partial solution, it just describes what a
/// valid group list is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupListDescription<StudentId: Identifier> {
    /// Students count per group (range)
    pub students_per_group: RangeInclusive<NonZeroU32>,
    /// Number of groups in the list (range)
    pub group_count: RangeInclusive<u32>,
    /// Students to dispatch in the groups
    pub students: BTreeSet<StudentId>,
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
    pub subject_descriptions:
        BTreeMap<SubjectId, SubjectDescription<SlotId, GroupListId, StudentId>>,
    /// List of group lists constraints
    pub group_list_descriptions: BTreeMap<GroupListId, GroupListDescription<StudentId>>,
}

use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum ValidationError {
    #[error("The number of weeks varies from slot to slot")]
    InconsistentWeekCount,
    #[error("Invalid group list id in group assignment")]
    InvalidGroupListId,
    #[error("Group count range should allow some value")]
    EmptyGroupCountRange,
    #[error("Student per group range should allow some value")]
    EmptyStudentPerGroupRange,
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    ColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>
{
    /// Check the consistency of data
    pub fn validate(
        self,
    ) -> Result<
        ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>,
        ValidationError,
    > {
        let mut week_count = None;
        for (_subject_id, subject_desc) in &self.subject_descriptions {
            match week_count {
                Some(count) => {
                    if subject_desc.group_assignments.len() != count {
                        return Err(ValidationError::InconsistentWeekCount);
                    }
                }
                None => {
                    week_count = Some(subject_desc.group_assignments.len());
                }
            }

            for (_slot_id, slot_desc) in &subject_desc.slots_descriptions {
                if slot_desc.weeks.len() != subject_desc.group_assignments.len() {
                    return Err(ValidationError::InconsistentWeekCount);
                }
            }

            for group_assignment_opt in &subject_desc.group_assignments {
                if let Some(group_assignment) = group_assignment_opt {
                    if !self
                        .group_list_descriptions
                        .contains_key(&group_assignment.group_list_id)
                    {
                        return Err(ValidationError::InvalidGroupListId);
                    }
                }
            }

            if subject_desc.students_per_group.is_empty() {
                return Err(ValidationError::EmptyStudentPerGroupRange);
            }
        }

        for (_group_list_id, group_list_desc) in &self.group_list_descriptions {
            if group_list_desc.group_count.is_empty() {
                return Err(ValidationError::EmptyGroupCountRange);
            }

            if group_list_desc.students_per_group.is_empty() {
                return Err(ValidationError::EmptyStudentPerGroupRange);
            }
        }

        Ok(ValidatedColloscopeProblem { internal: self })
    }
}

/// Validated problme that can be used in a solver
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedColloscopeProblem<
    SubjectId: Identifier,
    SlotId: Identifier,
    GroupListId: Identifier,
    StudentId: Identifier,
> {
    internal: ColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>,
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    collomatique_solver::SimpleBaseProblem
    for ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>
{
    type MainVariable = variables::MainVariable<GroupListId, StudentId, SubjectId, SlotId>;
    type PartialSolution = solution::Colloscope<SubjectId, SlotId, GroupListId, StudentId>;
    type StructureVariable =
        variables::StructureVariable<GroupListId, StudentId, SubjectId, SlotId>;

    fn main_variables(&self) -> BTreeMap<Self::MainVariable, collomatique_ilp::Variable> {
        let mut variables = BTreeMap::new();

        for (group_list_id, group_list_desc) in &self.internal.group_list_descriptions {
            let max_group_count = *group_list_desc.group_count.end();
            for student_id in &group_list_desc.students {
                variables.insert(
                    variables::MainVariable::GroupForStudent {
                        group_list: *group_list_id,
                        student: *student_id,
                    },
                    collomatique_ilp::Variable::integer()
                        .min(0.)
                        .max(f64::from(max_group_count - 1)),
                );
            }
        }

        for (subject_id, subject_desc) in &self.internal.subject_descriptions {
            for week in 0..subject_desc.group_assignments.len() {
                let Some(group_assignment) = &subject_desc.group_assignments[week] else {
                    continue;
                };
                let group_list = self
                    .internal
                    .group_list_descriptions
                    .get(&group_assignment.group_list_id)
                    .expect("Group list ID should be valid");

                let max_group_count = *group_list.group_count.end();

                for (slot_id, slot_desc) in &subject_desc.slots_descriptions {
                    if !slot_desc.weeks[week] {
                        continue;
                    }

                    for group in 0..max_group_count {
                        variables.insert(
                            variables::MainVariable::GroupInSlot {
                                subject: *subject_id,
                                slot: *slot_id,
                                week,
                                group,
                            },
                            collomatique_ilp::Variable::binary(),
                        );
                    }
                }
            }
        }

        variables
    }

    fn aggregated_variables(
        &self,
    ) -> Vec<
        Box<
            dyn collomatique_solver::tools::AggregatedVariables<
                collomatique_solver::generics::BaseVariable<
                    Self::MainVariable,
                    Self::StructureVariable,
                >,
            >,
        >,
    > {
        todo!()
    }

    fn configuration_to_partial_solution(
        &self,
        _config: &collomatique_ilp::ConfigData<Self::MainVariable>,
    ) -> Self::PartialSolution {
        todo!()
    }

    fn partial_solution_to_configuration(
        &self,
        _sol: &Self::PartialSolution,
    ) -> Option<collomatique_ilp::ConfigData<Self::MainVariable>> {
        todo!()
    }
}
