//! Simple schedule example
//!
//! This module implements a *very* simple example of scheduling problem to illustrate
//! the usage of [crate::BaseConstraints] and [crate::ExtraConstraints].
//!
//! The problem being implemented here is a very simple scheduling problem where
//! we have a few courses (let's n of them) and a few groups of students (let's
//! note m their number) than should attend them.
//!
//! The courses are supposed to happen all at the same time and all groups should attend
//! each course exactly once. That's it. There are only three variables to our problem: n, m as
//! already described, as well as the number of weeks. Thus we only have to complete a very simple schedule.
//!
//! The problem itself is described by [SimpleScheduleBase].

use collomatique_ilp::{ConfigData, Constraint, Variable};
use std::collections::BTreeMap;

use super::*;

/// Basic description of the simple scheduling problem
///
/// As described in the module documentation (see [self]),
/// there are only three parameters: the number of courses,
/// the numbers of groups, and the number of weeks.
/// These are given by [Self::course_count], [Self::group_count] and
/// [Self::week_count] respectively.
///
/// This struct will implement [BaseConstraints]. Because the problem
/// is so simple, there is no structure constraints, no structure variables
/// and thus no reconstruction.
///
/// We just have to implement a few general constraints. See [SimpleScheduleConstraint]
/// to see the description of such constraints.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimpleScheduleBase {
    /// Number of courses in our simple scheduling problem
    pub course_count: u32,
    /// Number of groups (of students) in our simple scheduling problem
    pub group_count: u32,
    /// Number of weeks in our simple scheduling problem
    pub week_count: u32,
}

/// Variables for the simple scheduling problem
///
/// They are all binary variables.
/// The variable is 1 if indeed the group [Self::group_index]
/// attends to the course [Self::course_index] on week
/// [Self::week_index].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimpleScheduleVariable {
    /// Index of a group
    ///
    /// The variable is 1 if indeed the group [Self::group_index]
    /// attends to the course [Self::course_index] on week
    /// [Self::week_index].
    pub group_index: u32,
    /// Index of a course
    ///
    /// The variable is 1 if indeed the group [Self::group_index]
    /// attends to the course [Self::course_index] on week
    /// [Self::week_index].
    pub course_index: u32,
    /// Index of a week
    ///
    /// The variable is 1 if indeed the group [Self::group_index]
    /// attends to the course [Self::course_index] on week
    /// [Self::week_index].
    pub week_index: u32,
}

impl std::fmt::Display for SimpleScheduleVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GiCoW_{}_{}_{}",
            self.group_index, self.course_index, self.week_index
        )
    }
}

/// Constraints descriptions for the simple scheduling problem
///
/// We only have general constraints and no structure constraints
/// for such a simple scheduling problem. So this type is built to describe
/// the possible (general) constraints that describes the problem
/// as an ILP problem.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimpleScheduleConstraint {
    /// This describes a first kind of constraints: each group
    /// can have *at most* one course per week.
    AtMostOneCoursePerWeekForAGivenGroup {
        /// Number of the group concerned by the constraint
        group: u32,
        /// Index of the week that the constraint considers
        week: u32,
    },
    /// This describes a second kind of constraints: each course
    /// can only have one group at a time.
    AtMostOneGroupPerCourseOnAGivenWeek {
        /// Number of the course concerned by the constraint
        course: u32,
        /// Index of the week that the constraint considers
        week: u32,
    },
    /// This describes a third (and final) kind of constraints:
    /// each group should attend each course exactly once.
    EachGroupShouldAttendEachCourseExactlyOnce {
        /// Number of the course concerned by the constraint
        course: u32,
        /// Number of the group concerned by the constraint
        group: u32,
    },
}

impl std::fmt::Display for SimpleScheduleConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AtMostOneCoursePerWeekForAGivenGroup { group, week } => {
                write!(f, "At most one course on week {} for group {}", week, group)
            }
            Self::AtMostOneGroupPerCourseOnAGivenWeek { course, week } => write!(
                f,
                "At most one group for course {} on week {}",
                course, week
            ),
            Self::EachGroupShouldAttendEachCourseExactlyOnce { course, group } => {
                write!(f, "Group {} attends course {} exactly once", group, course)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimpleScheduleSolution {}

impl SimpleScheduleBase {
    fn generate_at_most_one_course_per_week_for_a_given_group_constraints(
        &self,
    ) -> Vec<(Constraint<SimpleScheduleVariable>, SimpleScheduleConstraint)> {
        todo![]
    }

    fn generate_at_most_one_group_per_course_on_a_given_week_constraints(
        &self,
    ) -> Vec<(Constraint<SimpleScheduleVariable>, SimpleScheduleConstraint)> {
        todo![]
    }

    fn generate_each_group_should_attend_each_course_exactly_once_constraints(
        &self,
    ) -> Vec<(Constraint<SimpleScheduleVariable>, SimpleScheduleConstraint)> {
        todo![]
    }
}

impl BaseConstraints for SimpleScheduleBase {
    type MainVariable = SimpleScheduleVariable;
    type StructureVariable = ();
    type GeneralConstraintDesc = SimpleScheduleConstraint;
    type StructureConstraintDesc = ();
    type PartialSolution = SimpleScheduleSolution;

    fn main_variables(&self) -> BTreeMap<Self::MainVariable, Variable> {
        let mut output = BTreeMap::new();

        for group_index in 0..self.group_count {
            for course_index in 0..self.course_count {
                for week_index in 0..self.week_count {
                    output.insert(
                        SimpleScheduleVariable {
                            group_index,
                            course_index,
                            week_index,
                        },
                        Variable::binary(),
                    );
                }
            }
        }

        output
    }

    fn structure_variables(&self) -> BTreeMap<Self::StructureVariable, Variable> {
        BTreeMap::new()
    }

    fn structure_constraints(
        &self,
    ) -> Vec<(
        Constraint<BaseVariable<Self::MainVariable, Self::StructureVariable>>,
        Self::StructureConstraintDesc,
    )> {
        vec![]
    }

    fn general_constraints(
        &self,
    ) -> Vec<(
        Constraint<BaseVariable<Self::MainVariable, Self::StructureVariable>>,
        Self::GeneralConstraintDesc,
    )> {
        let mut output = vec![];

        output.extend(
            self.generate_at_most_one_course_per_week_for_a_given_group_constraints()
                .into_iter()
                .map(|(c, d)| (c.into_transmuted(|x| BaseVariable::Main(x)), d)),
        );
        output.extend(
            self.generate_at_most_one_group_per_course_on_a_given_week_constraints()
                .into_iter()
                .map(|(c, d)| (c.into_transmuted(|x| BaseVariable::Main(x)), d)),
        );
        output.extend(
            self.generate_each_group_should_attend_each_course_exactly_once_constraints()
                .into_iter()
                .map(|(c, d)| (c.into_transmuted(|x| BaseVariable::Main(x)), d)),
        );

        output
    }

    fn partial_solution_to_configuration(
        &self,
        sol: &Self::PartialSolution,
    ) -> ConfigData<Self::MainVariable> {
        todo!()
    }

    fn configuration_to_partial_solution(
        &self,
        config: &ConfigData<Self::MainVariable>,
    ) -> Self::PartialSolution {
        todo!()
    }

    fn reconstruct_structure_variables(
        &self,
        _config: &ConfigData<Self::MainVariable>,
    ) -> ConfigData<Self::StructureVariable> {
        ConfigData::new()
    }
}
