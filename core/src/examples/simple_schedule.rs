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

/// Basic description of the simple scheduling problem
///
/// As described in the module documentation (see [self]),
/// there are only three parameters: the number of courses,
/// the numbers of groups, and the number of weeks.
/// These are given by [Self::course_count], [Self::group_count] and
/// [Self::week_count] respectively.
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
