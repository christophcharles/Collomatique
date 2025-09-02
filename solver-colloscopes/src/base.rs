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

/// Data for prefilling of a group
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefilledGroup<StudentId: Identifier> {
    pub students: BTreeSet<StudentId>,
    pub sealed: bool,
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
    /// List of groups with preassigned students
    ///
    /// There should be a prefilled group for each possible existing group
    /// So the length of prefilled_groups is the maximum number of groups
    pub prefilled_groups: Vec<PrefilledGroup<StudentId>>,
    /// Minimum number of groups that should actually have students
    pub minimum_group_count: u32,
    /// Remaining students (not already in prefilled groups) to dispatch
    pub remaining_students: BTreeSet<StudentId>,
}

impl<StudentId: Identifier> GroupListDescription<StudentId> {
    /// Builds the complete list of students for the group list
    /// but fails with `None` if there is a duplicate
    pub fn build_student_set_or_none(&self) -> Option<BTreeSet<StudentId>> {
        let mut output = self.remaining_students.clone();

        for group in &self.prefilled_groups {
            for student_id in &group.students {
                if !output.insert(*student_id) {
                    return None;
                }
            }
        }

        Some(output)
    }
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
pub enum ValidationError<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier> {
    #[error("The number of weeks varies from slot to slot")]
    InconsistentWeekCount,
    #[error("A week (week {0}) is marked valid for slot {1:?} but the subject is not active on this week")]
    InconsistentWeekStatusInSlot(usize, SlotId),
    #[error("Invalid group list id ({0:?}) in group assignment for week {1} in subject {2:?}")]
    InvalidGroupListId(GroupListId, usize, SubjectId),
    #[error("Group count range should allow some value (group list {0:?})")]
    EmptyGroupCountRange(GroupListId),
    #[error("Student per group range should allow some value (group list {0:?})")]
    EmptyStudentPerGroupRange(GroupListId),
    #[error("Student per group range should allow some value (subject {0:?})")]
    EmptyStudentPerGroupRangeForSubject(SubjectId),
    #[error("Some students enrolled in subjects {0:?} do not appear in group list {1:?}")]
    GroupListDoesNotContainAllStudents(SubjectId, GroupListId),
    #[error("Group count should fit in u32 (group list {0:?}")]
    GroupCountTooBigForU32(GroupListId),
    #[error("Group list {0:?} contains a student (at least) twice: once in non-assigned students, once in a prefilled group")]
    DuplicateStudentInGroupList(GroupListId),
    #[error(
        "Prefilled group {1} exceeds the maximum number of students per group (group list {0:?})"
    )]
    TooManyStudentsInPrefilledGroup(GroupListId, u32),
    #[error("Sealed group {1} does not have enough students (group list {0:?})")]
    TooFewStudentsInSealedGroup(GroupListId, u32),
    #[error("Prefilled group {2} exceeds the maximum number of students per group when specialized for subject {0:?} on week {1}")]
    TooManyStudentsInPrefilledGroupForSubject(SubjectId, usize, u32),
    #[error("Sealed group {2} does not have enough students when specialized for subject {0:?} on week {1}")]
    TooFewStudentsInSealedGroupForSubject(SubjectId, usize, u32),
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    ColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>
{
    /// Check the consistency of data
    pub fn validate(
        self,
    ) -> Result<
        ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>,
        ValidationError<SubjectId, SlotId, GroupListId>,
    > {
        let mut students_in_group_lists = BTreeMap::new();
        for (group_list_id, group_list_desc) in &self.group_list_descriptions {
            let Some(students) = group_list_desc.build_student_set_or_none() else {
                return Err(ValidationError::DuplicateStudentInGroupList(*group_list_id));
            };

            students_in_group_lists.insert(*group_list_id, students);
        }

        let mut week_count = None;
        for (subject_id, subject_desc) in &self.subject_descriptions {
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

            for (slot_id, slot_desc) in &subject_desc.slots_descriptions {
                if slot_desc.weeks.len() != subject_desc.group_assignments.len() {
                    return Err(ValidationError::InconsistentWeekCount);
                }

                for ((week, slot), group_assignment_opt) in slot_desc
                    .weeks
                    .iter()
                    .enumerate()
                    .zip(&subject_desc.group_assignments)
                {
                    if *slot && group_assignment_opt.is_none() {
                        return Err(ValidationError::InconsistentWeekStatusInSlot(
                            week, *slot_id,
                        ));
                    }
                }
            }

            for (week, group_assignment_opt) in subject_desc.group_assignments.iter().enumerate() {
                if let Some(group_assignment) = group_assignment_opt {
                    let Some(group_list_students) =
                        students_in_group_lists.get(&group_assignment.group_list_id)
                    else {
                        return Err(ValidationError::InvalidGroupListId(
                            group_assignment.group_list_id,
                            week,
                            *subject_id,
                        ));
                    };

                    if !group_assignment
                        .enrolled_students
                        .is_subset(group_list_students)
                    {
                        return Err(ValidationError::GroupListDoesNotContainAllStudents(
                            *subject_id,
                            group_assignment.group_list_id,
                        ));
                    }

                    let group_list_desc = self
                        .group_list_descriptions
                        .get(&group_assignment.group_list_id)
                        .expect("Group list id should be valid at this point");

                    for (i, group) in group_list_desc.prefilled_groups.iter().enumerate() {
                        let group_students: BTreeSet<_> = group
                            .students
                            .difference(&group_assignment.enrolled_students)
                            .copied()
                            .collect();

                        if group_students.len() > u32::MAX as usize {
                            return Err(ValidationError::TooManyStudentsInPrefilledGroup(
                                group_assignment.group_list_id,
                                i as u32,
                            ));
                        }
                        if (group_students.len() as u32)
                            > subject_desc.students_per_group.end().get()
                        {
                            return Err(
                                ValidationError::TooManyStudentsInPrefilledGroupForSubject(
                                    *subject_id,
                                    week,
                                    i as u32,
                                ),
                            );
                        }
                        if group.sealed && (group_students.len() as u32) != 0 {
                            if (group_students.len() as u32)
                                < subject_desc.students_per_group.start().get()
                            {
                                return Err(
                                    ValidationError::TooFewStudentsInSealedGroupForSubject(
                                        *subject_id,
                                        week,
                                        i as u32,
                                    ),
                                );
                            }
                        }
                    }
                }
            }

            if subject_desc.students_per_group.is_empty() {
                return Err(ValidationError::EmptyStudentPerGroupRangeForSubject(
                    *subject_id,
                ));
            }
        }

        for (group_list_id, group_list_desc) in &self.group_list_descriptions {
            if group_list_desc.prefilled_groups.len() > u32::MAX as usize {
                return Err(ValidationError::GroupCountTooBigForU32(*group_list_id));
            }

            if group_list_desc.minimum_group_count > group_list_desc.prefilled_groups.len() as u32 {
                return Err(ValidationError::EmptyGroupCountRange(*group_list_id));
            }

            if group_list_desc.students_per_group.is_empty() {
                return Err(ValidationError::EmptyStudentPerGroupRange(*group_list_id));
            }

            for (i, group) in group_list_desc.prefilled_groups.iter().enumerate() {
                if group.students.len() > u32::MAX as usize {
                    return Err(ValidationError::TooManyStudentsInPrefilledGroup(
                        *group_list_id,
                        i as u32,
                    ));
                }
                if group.students.len() as u32 > group_list_desc.students_per_group.end().get() {
                    return Err(ValidationError::TooManyStudentsInPrefilledGroup(
                        *group_list_id,
                        i as u32,
                    ));
                }
                if group.sealed && (group.students.len() as u32) != 0 {
                    if (group.students.len() as u32)
                        < group_list_desc.students_per_group.start().get()
                    {
                        return Err(ValidationError::TooFewStudentsInSealedGroup(
                            *group_list_id,
                            i as u32,
                        ));
                    }
                }
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
    ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>
{
    pub fn inner(&self) -> &ColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId> {
        &self.internal
    }

    pub fn into_inner(self) -> ColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId> {
        self.internal
    }
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    std::ops::Deref for ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>
{
    type Target = ColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    collomatique_solver::SimpleBaseProblem
    for ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>
{
    type MainVariable = variables::MainVariable<GroupListId, StudentId, SubjectId, SlotId>;
    type PartialSolution = solution::ValidatedColloscope<SubjectId, SlotId, GroupListId, StudentId>;
    type StructureVariable =
        variables::StructureVariable<GroupListId, StudentId, SubjectId, SlotId>;

    fn main_variables(&self) -> BTreeMap<Self::MainVariable, collomatique_ilp::Variable> {
        let mut variables = BTreeMap::new();

        for (group_list_id, group_list_desc) in &self.internal.group_list_descriptions {
            let max_group_count = group_list_desc.prefilled_groups.len() as u32;
            for student_id in &group_list_desc.remaining_students {
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

                let max_group_count = group_list.prefilled_groups.len() as u32;

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
        use collomatique_solver::generics::BaseVariable;
        use collomatique_solver::tools::{
            AndVariable, FixedVariable, OrVariable, UIntToBinVariables,
        };

        let mut variables = vec![];

        for (group_list_id, group_list_desc) in &self.internal.group_list_descriptions {
            let max_group_count = group_list_desc.prefilled_groups.len() as u32;
            let group_list = group_list_id.clone();
            for student_id in &group_list_desc.remaining_students {
                let student = student_id.clone();
                variables.push(Box::new(UIntToBinVariables {
                    variable_name_builder: move |i| {
                        BaseVariable::Structure(variables::StructureVariable::StudentInGroup {
                            group_list,
                            student,
                            group: i,
                        })
                    },
                    original_variable: BaseVariable::Main(
                        variables::MainVariable::GroupForStudent {
                            group_list,
                            student,
                        },
                    ),
                    original_range: 0..=(max_group_count - 1),
                })
                    as Box<dyn collomatique_solver::tools::AggregatedVariables<_>>);
            }

            for group in 0..max_group_count {
                let prefilled_group = &group_list_desc.prefilled_groups[group as usize];
                if !prefilled_group.students.is_empty() {
                    variables.push(Box::new(FixedVariable {
                        variable_name: BaseVariable::Structure(
                            variables::StructureVariable::NonEmptyGroup { group_list, group },
                        ),
                        value: true,
                    })
                        as Box<dyn collomatique_solver::tools::AggregatedVariables<_>>);
                } else if prefilled_group.sealed {
                    variables.push(Box::new(FixedVariable {
                        variable_name: BaseVariable::Structure(
                            variables::StructureVariable::NonEmptyGroup { group_list, group },
                        ),
                        value: false,
                    })
                        as Box<dyn collomatique_solver::tools::AggregatedVariables<_>>);
                } else {
                    variables.push(Box::new(OrVariable {
                        variable_name: BaseVariable::Structure(
                            variables::StructureVariable::NonEmptyGroup { group_list, group },
                        ),
                        original_variables: group_list_desc
                            .remaining_students
                            .iter()
                            .map(|student_id| {
                                let student = *student_id;
                                BaseVariable::Structure(
                                    variables::StructureVariable::StudentInGroup {
                                        group_list,
                                        student,
                                        group,
                                    },
                                )
                            })
                            .collect(),
                    })
                        as Box<dyn collomatique_solver::tools::AggregatedVariables<_>>);
                }
            }
        }

        let mut subclasses_for_group_lists: BTreeMap<GroupListId, BTreeSet<BTreeSet<StudentId>>> =
            self.group_list_descriptions
                .keys()
                .map(|group_list_id| (*group_list_id, BTreeSet::new()))
                .collect();

        for (subject_id, subject_desc) in &self.internal.subject_descriptions {
            for (week, group_assignment_opt) in subject_desc.group_assignments.iter().enumerate() {
                let Some(group_assignment) = group_assignment_opt else {
                    continue;
                };

                let subclass = group_assignment.enrolled_students.clone();

                let subclasses = subclasses_for_group_lists
                    .get_mut(&group_assignment.group_list_id)
                    .expect("Group list id should be listed");
                subclasses.insert(subclass);

                let group_list_desc = self
                    .internal
                    .group_list_descriptions
                    .get(&group_assignment.group_list_id)
                    .expect("group list id should be valid");
                let max_group_count = group_list_desc.prefilled_groups.len() as u32;
                for group in 0..max_group_count {
                    for student_id in &group_assignment.enrolled_students {
                        if !group_list_desc.remaining_students.contains(student_id) {
                            continue;
                        }
                        for (slot_id, slot_desc) in &subject_desc.slots_descriptions {
                            if !slot_desc.weeks[week] {
                                continue;
                            }

                            variables.push(Box::new(AndVariable {
                                variable_name: BaseVariable::Structure(
                                    variables::StructureVariable::StudentInGroupAndSlot {
                                        subject: *subject_id,
                                        student: *student_id,
                                        group,
                                        slot: *slot_id,
                                        week,
                                    },
                                ),
                                original_variables: BTreeSet::from([
                                    BaseVariable::Main(variables::MainVariable::GroupInSlot {
                                        subject: *subject_id,
                                        slot: *slot_id,
                                        week,
                                        group,
                                    }),
                                    BaseVariable::Structure(
                                        variables::StructureVariable::StudentInGroup {
                                            group_list: group_assignment.group_list_id,
                                            student: *student_id,
                                            group,
                                        },
                                    ),
                                ]),
                            })
                                as Box<dyn collomatique_solver::tools::AggregatedVariables<_>>);
                        }
                    }
                }

                for (slot_id, slot_desc) in &subject_desc.slots_descriptions {
                    if !slot_desc.weeks[week] {
                        continue;
                    }

                    variables.push(Box::new(OrVariable {
                        variable_name: BaseVariable::Structure(
                            variables::StructureVariable::NonEmptySlot {
                                subject: *subject_id,
                                slot: *slot_id,
                                week,
                            },
                        ),
                        original_variables: (0..max_group_count)
                            .into_iter()
                            .map(|group| {
                                BaseVariable::Main(variables::MainVariable::GroupInSlot {
                                    subject: *subject_id,
                                    slot: *slot_id,
                                    group,
                                    week,
                                })
                            })
                            .collect(),
                    })
                        as Box<dyn collomatique_solver::tools::AggregatedVariables<_>>);
                }
            }
        }

        for (group_list_id, subclasses) in subclasses_for_group_lists {
            let group_list_desc = self
                .internal
                .group_list_descriptions
                .get(&group_list_id)
                .expect("group list id should be valid");
            let max_group_count = group_list_desc.prefilled_groups.len() as u32;

            for subclass in subclasses {
                for group in 0..max_group_count {
                    let prefilled_group = &group_list_desc.prefilled_groups[group as usize];
                    if !prefilled_group.students.is_disjoint(&subclass) {
                        variables.push(Box::new(FixedVariable {
                            variable_name: BaseVariable::Structure(
                                variables::StructureVariable::NonEmptyGroupForSubClass {
                                    subclass: subclass.clone(),
                                    group_list: group_list_id,
                                    group,
                                },
                            ),
                            value: true,
                        })
                            as Box<dyn collomatique_solver::tools::AggregatedVariables<_>>);
                    } else if prefilled_group.sealed {
                        variables.push(Box::new(FixedVariable {
                            variable_name: BaseVariable::Structure(
                                variables::StructureVariable::NonEmptyGroupForSubClass {
                                    subclass: subclass.clone(),
                                    group_list: group_list_id,
                                    group,
                                },
                            ),
                            value: false,
                        })
                            as Box<dyn collomatique_solver::tools::AggregatedVariables<_>>);
                    } else {
                        variables.push(Box::new(OrVariable {
                            variable_name: BaseVariable::Structure(
                                variables::StructureVariable::NonEmptyGroupForSubClass {
                                    subclass: subclass.clone(),
                                    group_list: group_list_id,
                                    group,
                                },
                            ),
                            original_variables: subclass
                                .iter()
                                .map(|student_id| {
                                    let student = *student_id;
                                    BaseVariable::Structure(
                                        variables::StructureVariable::StudentInGroup {
                                            group_list: group_list_id,
                                            student,
                                            group,
                                        },
                                    )
                                })
                                .collect(),
                        })
                            as Box<dyn collomatique_solver::tools::AggregatedVariables<_>>);
                    }
                }
            }
        }

        variables
    }

    fn configuration_to_partial_solution(
        &self,
        config: &collomatique_ilp::ConfigData<Self::MainVariable>,
    ) -> Self::PartialSolution {
        let colloscope = solution::Colloscope {
            group_lists: self
                .internal
                .group_list_descriptions
                .iter()
                .map(|(group_list_id, group_list_desc)| {
                    let mut groups_for_remaining_students = BTreeMap::new();

                    for student_id in &group_list_desc.remaining_students {
                        let group_opt = config.get(variables::MainVariable::GroupForStudent {
                            group_list: *group_list_id,
                            student: *student_id,
                        });

                        let group_u32_opt = group_opt.map(|x| x as u32);

                        groups_for_remaining_students.insert(*student_id, group_u32_opt);
                    }

                    (
                        *group_list_id,
                        solution::GroupList {
                            groups_for_remaining_students,
                        },
                    )
                })
                .collect(),
            subject_map: self
                .internal
                .subject_descriptions
                .iter()
                .map(|(subject_id, subject_desc)| {
                    let mut slots = BTreeMap::new();

                    for (slot_id, slot_desc) in &subject_desc.slots_descriptions {
                        let mut slot_interrogations = vec![];

                        for (week, group_assignment_opt) in
                            subject_desc.group_assignments.iter().enumerate()
                        {
                            let Some(group_assignment) = group_assignment_opt else {
                                slot_interrogations.push(None);
                                continue;
                            };

                            if !slot_desc.weeks[week] {
                                slot_interrogations.push(None);
                                continue;
                            }

                            let mut interrogation = solution::Interrogation {
                                group_list_id: group_assignment.group_list_id,
                                assigned_groups: BTreeSet::new(),
                                unassigned_groups: BTreeSet::new(),
                            };
                            let group_list = self
                                .internal
                                .group_list_descriptions
                                .get(&group_assignment.group_list_id)
                                .expect("group list id should be valid");
                            let max_group_count = group_list.prefilled_groups.len() as u32;
                            for group in 0..max_group_count {
                                let status_opt = config.get(variables::MainVariable::GroupInSlot {
                                    subject: *subject_id,
                                    slot: *slot_id,
                                    week,
                                    group,
                                });

                                match status_opt {
                                    Some(status) => {
                                        if status > 0.5 {
                                            interrogation.assigned_groups.insert(group);
                                        }
                                    }
                                    None => {
                                        interrogation.unassigned_groups.insert(group);
                                    }
                                }
                            }
                        }

                        slots.insert(*slot_id, slot_interrogations);
                    }

                    (*subject_id, solution::SubjectInterrogations { slots })
                })
                .collect(),
        };

        colloscope
            .validate()
            .expect("Colloscope solution should always be valid at this point")
    }

    fn partial_solution_to_configuration(
        &self,
        sol: &Self::PartialSolution,
    ) -> Option<collomatique_ilp::ConfigData<Self::MainVariable>> {
        let mut config = collomatique_ilp::ConfigData::new();

        if sol.group_lists.len() != self.internal.group_list_descriptions.len() {
            return None;
        }

        for (group_list_id, group_list_desc) in &self.internal.group_list_descriptions {
            let Some(group_list) = sol.group_lists.get(group_list_id) else {
                return None;
            };
            if group_list.groups_for_remaining_students.len()
                != group_list_desc.remaining_students.len()
            {
                return None;
            }
            let max_group_count = group_list_desc.prefilled_groups.len() as u32;

            for (student_id, group_opt) in &group_list.groups_for_remaining_students {
                if !group_list_desc.remaining_students.contains(student_id) {
                    return None;
                }
                if let Some(group) = group_opt {
                    if *group >= max_group_count {
                        return None;
                    }

                    config = config.set(
                        variables::MainVariable::GroupForStudent {
                            group_list: *group_list_id,
                            student: *student_id,
                        },
                        f64::from(*group),
                    )
                }
            }
        }

        if sol.subject_map.len() != self.subject_descriptions.len() {
            return None;
        }

        for (subject_id, subject_desc) in &self.subject_descriptions {
            let Some(subject) = sol.subject_map.get(subject_id) else {
                return None;
            };

            if subject_desc.slots_descriptions.len() != subject.slots.len() {
                return None;
            }

            for (slot_id, slot_desc) in &subject_desc.slots_descriptions {
                let Some(slot) = subject.slots.get(slot_id) else {
                    return None;
                };

                if slot.len() != slot_desc.weeks.len() {
                    return None;
                }

                for (week, interrogation_opt) in slot.iter().enumerate() {
                    if !slot_desc.weeks[week] {
                        if interrogation_opt.is_some() {
                            return None;
                        }
                    }

                    let Some(interrogation) = interrogation_opt else {
                        return None;
                    };

                    let group_assignment = subject_desc.group_assignments[week]
                        .as_ref()
                        .expect("There should be a group assignment as the slot is marked valid");

                    if interrogation.group_list_id != group_assignment.group_list_id {
                        return None;
                    }

                    let group_list_desc = self
                        .group_list_descriptions
                        .get(&group_assignment.group_list_id)
                        .expect("Group list id should be valid");
                    let max_group_count = group_list_desc.prefilled_groups.len() as u32;
                    for group in 0..max_group_count {
                        if interrogation.assigned_groups.contains(&group) {
                            config = config.set(
                                variables::MainVariable::GroupInSlot {
                                    subject: *subject_id,
                                    slot: *slot_id,
                                    week,
                                    group,
                                },
                                1.,
                            );
                        } else if !interrogation.unassigned_groups.contains(&group) {
                            config = config.set(
                                variables::MainVariable::GroupInSlot {
                                    subject: *subject_id,
                                    slot: *slot_id,
                                    week,
                                    group,
                                },
                                0.,
                            );
                        }
                    }
                }
            }
        }

        Some(config)
    }
}
