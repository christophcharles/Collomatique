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
//! The problem itself is described by [SimpleScheduleDesc] and the fundamental constraints are provided by
//! [SimpleScheduleConstraints].
//!
//! Here a simple example of using the library to solve such a simple scheduling problem:
//! ```
//! # use collomatique_core::{examples::simple_schedule::{SimpleScheduleDesc, SimpleScheduleConstraints}, ProblemBuilder};
//! # use std::collections::BTreeSet;
//! let problem_desc = SimpleScheduleDesc {
//!     group_count: 2,
//!     week_count: 2,
//!     course_count: 2,
//! };
//! let constraints = SimpleScheduleConstraints {};
//!
//! let mut problem_builder = ProblemBuilder::<_,_,_>::new(problem_desc, 1.0)
//!     .expect("Consistent ILP description");
//! let translator = problem_builder.add_constraints(constraints, 1.0)
//!     .expect("Consistent ILP description");
//! let problem = problem_builder.build();
//!
//! let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::new();
//! let solution = problem.solve(&solver).expect("There should be at least a solution").into_solution();
//!
//! // The solution should be complete as it was found by the solver
//! assert!(solution.is_complete());
//! // There should be less than one assignment per cell as the constraints are satisfied
//! for group in 0..1 {
//!     for week in 0..1 {
//!         assert!(solution.get_assigned(group, week).unwrap().len() <= 1);
//!     }
//! }
//!
//! // In fact, we know the two solutions for this small problem, let's check we got one of them
//! if solution.get_assigned(0, 0).unwrap().contains(&0) {
//!     assert_eq!(
//!         solution.get_assigned(0,0).cloned().unwrap(),
//!         BTreeSet::from([0])
//!     );
//!     assert_eq!(
//!         solution.get_assigned(0,1).cloned().unwrap(),
//!         BTreeSet::from([1])
//!     );
//!     assert_eq!(
//!         solution.get_assigned(1,0).cloned().unwrap(),
//!         BTreeSet::from([1])
//!     );
//!     assert_eq!(
//!         solution.get_assigned(1,1).cloned().unwrap(),
//!         BTreeSet::from([0])
//!     );
//! } else {
//!     assert_eq!(
//!         solution.get_assigned(0,0).cloned().unwrap(),
//!         BTreeSet::from([1])
//!     );
//!     assert_eq!(
//!         solution.get_assigned(0,1).cloned().unwrap(),
//!         BTreeSet::from([0])
//!     );
//!     assert_eq!(
//!         solution.get_assigned(1,0).cloned().unwrap(),
//!         BTreeSet::from([0])
//!     );
//!     assert_eq!(
//!         solution.get_assigned(1,1).cloned().unwrap(),
//!         BTreeSet::from([1])
//!     );
//! }
//! ```

use collomatique_ilp::{ConfigData, Constraint, LinExpr, Variable};
use std::collections::{BTreeMap, BTreeSet};

use super::*;

#[cfg(test)]
mod tests;

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
pub struct SimpleScheduleDesc {
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
/// The variable is 1 if indeed the group [Self::group]
/// attends to the course [Self::course] on week
/// [Self::week].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimpleScheduleVariable {
    /// Index of a group
    ///
    /// The variable is 1 if indeed the group [Self::group]
    /// attends to the course [Self::course] on week
    /// [Self::week].
    pub group: u32,
    /// Index of a course
    ///
    /// The variable is 1 if indeed the group [Self::group]
    /// attends to the course [Self::course] on week
    /// [Self::week].
    pub course: u32,
    /// Index of a week
    ///
    /// The variable is 1 if indeed the group [Self::group]
    /// attends to the course [Self::course] on week
    /// [Self::week].
    pub week: u32,
}

impl std::fmt::Display for SimpleScheduleVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GiCoW_{}_{}_{}", self.group, self.course, self.week)
    }
}

/// Description of a solution for the simple scheduling problem
///
/// Here, we must be attentive. A solution is in fact a *partial*
/// solution. This means that all variable values may not be set.
///
/// Also, a partial solution is not necessaraly a *feasable* solution.
/// This means that some constraints may not be satisfied. For instance,
/// we can't assume that each group has a single course a given week.
///
/// Mathematically, have a representation that is one to one with functions from
/// the variable set and taking their values in `Option<bool>`.
///
/// But we still want a representation that is closer to what is needed
/// in the rest of the program. Here, that might mean that we want for each
/// group and for each week the number of the course they should be attending.
///
/// If all the constraints were satisfied, we could represent this with a double-indexed
/// array where each row represents a group and each column a week. Each cell
/// would either be empty (no course this week for the group) or contain the
/// number of the course. This is a typical representation but assumes that all constraints
/// are satisfied.
///
/// We will do a slight variation: we will represent the data as an array. However each cell
/// will rather contain two  BTreeSet. The first one is assigned courses. These are the courses
/// the group should follow this week (eventhough more than one is impossible in practice,
/// this is a possible if the constraints are not all satisfied). The second one is
/// non-assigned courses. These are courses the group might or might not attend, the *partial*
/// solution does not specify it.
///
/// A solution is *complete* if and only if all the second BTreeSet are empty.
///
/// Internally, this is represented by two `Vec<BTreeSet<u32>>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimpleSchedulePartialSolution {
    week_count: usize,
    assigned_courses: Vec<BTreeSet<u32>>,
    unassigned_courses: Vec<BTreeSet<u32>>,
}

impl SimpleSchedulePartialSolution {
    fn compute_index(&self, group: u32, week: u32) -> Option<usize> {
        let group_usize = group as usize;
        let week_usize = week as usize;

        if week_usize >= self.week_count {
            return None;
        }

        let index = group_usize * self.week_count + week_usize;

        if index >= self.assigned_courses.len() {
            return None;
        }

        Some(index)
    }

    /// Returns the list of assigned courses for a given group on a given week
    pub fn get_assigned(&self, group: u32, week: u32) -> Option<&BTreeSet<u32>> {
        let index = self.compute_index(group, week)?;
        Some(&self.assigned_courses[index])
    }

    /// Returns the list of unassigned courses for a given group on a given week
    pub fn get_unassigned(&self, group: u32, week: u32) -> Option<&BTreeSet<u32>> {
        let index = self.compute_index(group, week)?;
        Some(&self.unassigned_courses[index])
    }

    /// Is the solution complete?
    pub fn is_complete(&self) -> bool {
        self.unassigned_courses.iter().all(|x| x.is_empty())
    }
}

impl BaseConstraints for SimpleScheduleDesc {
    type MainVariable = SimpleScheduleVariable;
    type StructureVariable = ();
    type GeneralConstraintDesc = ();
    type StructureConstraintDesc = ();
    type PartialSolution = SimpleSchedulePartialSolution;

    fn main_variables(&self) -> BTreeMap<Self::MainVariable, Variable> {
        let mut output = BTreeMap::new();

        for group in 0..self.group_count {
            for course in 0..self.course_count {
                for week in 0..self.week_count {
                    output.insert(
                        SimpleScheduleVariable {
                            group,
                            course,
                            week,
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
        vec![]
    }

    fn partial_solution_to_configuration(
        &self,
        sol: &Self::PartialSolution,
    ) -> Option<ConfigData<Self::MainVariable>> {
        if self.week_count as usize != sol.week_count {
            return None;
        }
        let cell_count = (self.week_count * self.group_count) as usize;
        if cell_count != sol.assigned_courses.len() || cell_count != sol.unassigned_courses.len() {
            return None;
        }

        let mut config_data = ConfigData::new();

        let mut index = 0usize;
        for group in 0..self.group_count {
            for week in 0..self.week_count {
                for course in 0..self.course_count {
                    if sol.assigned_courses[index].contains(&course) {
                        config_data = config_data.set(
                            SimpleScheduleVariable {
                                group,
                                course,
                                week,
                            },
                            1.0,
                        );
                    } else if !sol.unassigned_courses[index].contains(&course) {
                        config_data = config_data.set(
                            SimpleScheduleVariable {
                                group,
                                course,
                                week,
                            },
                            0.0,
                        );
                    }
                }
                index += 1;
            }
        }

        Some(config_data)
    }

    fn configuration_to_partial_solution(
        &self,
        config: &ConfigData<Self::MainVariable>,
    ) -> Self::PartialSolution {
        let week_count_usize = self.week_count as usize;
        let group_count_usize = self.group_count as usize;

        let vec_size = week_count_usize * group_count_usize;

        let mut assigned_courses = vec![BTreeSet::new(); vec_size];
        let mut unassigned_courses = vec![BTreeSet::new(); vec_size];

        let mut index = 0usize;
        for group in 0..self.group_count {
            for week in 0..self.week_count {
                for course in 0..self.course_count {
                    let var = config.get(SimpleScheduleVariable {
                        group,
                        course,
                        week,
                    });

                    match var {
                        Some(v) => {
                            if v > 0.5 {
                                assigned_courses[index].insert(course);
                            }
                        }
                        None => {
                            unassigned_courses[index].insert(course);
                        }
                    }
                }
                index += 1;
            }
        }

        SimpleSchedulePartialSolution {
            week_count: week_count_usize,
            assigned_courses,
            unassigned_courses,
        }
    }

    fn reconstruct_structure_variables(
        &self,
        _config: &ConfigData<Self::MainVariable>,
    ) -> ConfigData<Self::StructureVariable> {
        ConfigData::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimpleScheduleConstraints {}

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

impl SimpleScheduleConstraints {
    fn generate_at_most_one_course_per_week_for_a_given_group_constraint_for_specific_group_and_week(
        desc: &SimpleScheduleDesc,
        group: u32,
        week: u32,
    ) -> (Constraint<SimpleScheduleVariable>, SimpleScheduleConstraint) {
        let mut lhs = LinExpr::constant(0.);

        for course in 0..desc.course_count {
            lhs = lhs
                + LinExpr::var(SimpleScheduleVariable {
                    group,
                    week,
                    course,
                });
        }

        let rhs = LinExpr::constant(1.0);

        (
            lhs.leq(&rhs),
            SimpleScheduleConstraint::AtMostOneCoursePerWeekForAGivenGroup { group, week },
        )
    }

    fn generate_at_most_one_course_per_week_for_a_given_group_constraints(
        desc: &SimpleScheduleDesc,
    ) -> Vec<(Constraint<SimpleScheduleVariable>, SimpleScheduleConstraint)> {
        let mut output = vec![];

        for group in 0..desc.group_count {
            for week in 0..desc.week_count {
                output.push(Self::generate_at_most_one_course_per_week_for_a_given_group_constraint_for_specific_group_and_week(desc, group, week));
            }
        }

        output
    }

    fn generate_at_most_one_group_per_course_on_a_given_week_constraint_for_specific_week_and_course(
        desc: &SimpleScheduleDesc,
        week: u32,
        course: u32,
    ) -> (Constraint<SimpleScheduleVariable>, SimpleScheduleConstraint) {
        let mut lhs = LinExpr::constant(0.);

        for group in 0..desc.group_count {
            lhs = lhs
                + LinExpr::var(SimpleScheduleVariable {
                    group,
                    week,
                    course,
                });
        }

        let rhs = LinExpr::constant(1.0);

        (
            lhs.leq(&rhs),
            SimpleScheduleConstraint::AtMostOneGroupPerCourseOnAGivenWeek { course, week },
        )
    }

    fn generate_at_most_one_group_per_course_on_a_given_week_constraints(
        desc: &SimpleScheduleDesc,
    ) -> Vec<(Constraint<SimpleScheduleVariable>, SimpleScheduleConstraint)> {
        let mut output = vec![];

        for week in 0..desc.week_count {
            for course in 0..desc.course_count {
                output.push(Self::generate_at_most_one_group_per_course_on_a_given_week_constraint_for_specific_week_and_course(desc, week, course));
            }
        }

        output
    }

    fn generate_each_group_should_attend_each_course_exactly_once_constraint_for_specific_group_and_course(
        desc: &SimpleScheduleDesc,
        group: u32,
        course: u32,
    ) -> (Constraint<SimpleScheduleVariable>, SimpleScheduleConstraint) {
        let mut lhs = LinExpr::constant(0.);

        for week in 0..desc.week_count {
            lhs = lhs
                + LinExpr::var(SimpleScheduleVariable {
                    group,
                    week,
                    course,
                });
        }

        let rhs = LinExpr::constant(1.0);

        (
            lhs.eq(&rhs),
            SimpleScheduleConstraint::EachGroupShouldAttendEachCourseExactlyOnce { course, group },
        )
    }

    fn generate_each_group_should_attend_each_course_exactly_once_constraints(
        desc: &SimpleScheduleDesc,
    ) -> Vec<(Constraint<SimpleScheduleVariable>, SimpleScheduleConstraint)> {
        let mut output = vec![];

        for group in 0..desc.group_count {
            for course in 0..desc.course_count {
                output.push(Self::generate_each_group_should_attend_each_course_exactly_once_constraint_for_specific_group_and_course(desc, group, course));
            }
        }

        output
    }
}

impl ExtraConstraints<SimpleScheduleDesc> for SimpleScheduleConstraints {
    type GeneralConstraintDesc = SimpleScheduleConstraint;
    type StructureConstraintDesc = ();
    type StructureVariable = ();

    fn is_fit_for_problem(&self, _desc: &SimpleScheduleDesc) -> bool {
        true
    }

    fn extra_structure_variables(
        &self,
        _desc: &SimpleScheduleDesc,
    ) -> BTreeMap<Self::StructureVariable, Variable> {
        BTreeMap::new()
    }

    fn extra_structure_constraints(
        &self,
        _desc: &SimpleScheduleDesc,
    ) -> Vec<(
        Constraint<ExtraVariable<SimpleScheduleVariable, (), Self::StructureVariable>>,
        Self::StructureConstraintDesc,
    )> {
        vec![]
    }

    fn reconstruct_extra_structure_variables(
        &self,
        _desc: &SimpleScheduleDesc,
        _config: &ConfigData<BaseVariable<SimpleScheduleVariable, ()>>,
    ) -> ConfigData<Self::StructureVariable> {
        ConfigData::new()
    }

    fn extra_general_constraints(
        &self,
        desc: &SimpleScheduleDesc,
    ) -> Vec<(
        Constraint<ExtraVariable<SimpleScheduleVariable, (), Self::StructureVariable>>,
        Self::GeneralConstraintDesc,
    )> {
        let mut output = vec![];

        output.extend(
            Self::generate_at_most_one_course_per_week_for_a_given_group_constraints(desc)
                .into_iter()
                .map(|(c, d)| (c.into_transmuted(|x| ExtraVariable::BaseMain(x)), d)),
        );
        output.extend(
            Self::generate_at_most_one_group_per_course_on_a_given_week_constraints(desc)
                .into_iter()
                .map(|(c, d)| (c.into_transmuted(|x| ExtraVariable::BaseMain(x)), d)),
        );
        output.extend(
            Self::generate_each_group_should_attend_each_course_exactly_once_constraints(desc)
                .into_iter()
                .map(|(c, d)| (c.into_transmuted(|x| ExtraVariable::BaseMain(x)), d)),
        );

        output
    }
}
