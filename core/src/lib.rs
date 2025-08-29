//! Collomatique-core
//! ---
//!
//! This crate contains the main logic of Collomatique for solving colloscopes.
//! The goal for this crate is to fulfill the role of a translator. It takes a description
//! of a colloscope (or at least the various constraints of a colloscope) and returns
//! an ILP problem as described by the crate [collomatique-ilp].
//!
//! Similarly, it can translate a solution of an ILP problem into the description of
//! an actual colloscope and conversly, it can take the description of a colloscope
//! and turn it into an ILP configuration. This is useful to check in real time if
//! a colloscope satisfies all the constraints and helps when constructing a colloscope
//! incrementally.
//!
//! This crate however does not expose a user-friendly interface. The reason is, to
//! make the translation algorithm as thin as possible, and its verification as easy as
//! possible, I strive to make the colloscopes constraints and the actual colloscopes
//! representation the least redundant possible.
//!
//! Also to keep this part lean, a lot of information is not represented as it is not
//! needed to build the constraint system. For instance, the name of the students or
//! the name of the teachers are not stored in the structures of this modules. Students
//! and teachers are represented with numbers and that's it. It is the job of other crates
//! from collomatique to provide necessary utilities to make working the algorithm
//! somewhat pleasant.
//!

pub mod time;

use std::collections::BTreeSet;
use std::ops::RangeInclusive;
use std::num::NonZeroUsize;

/// Description of the information pertinent to a single student
/// for the colloscope.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Student {
    /// List of times the student is not available for interrogations or tutorial
    ///
    /// Using BTreeSet garantees that there are no duplications.
    pub incompatibilities: BTreeSet<time::SlotWithDuration>,

    /// Whether the student can have two consecutive interrogations.
    pub non_consecutive_interrogations: bool,
}

/// Description of the constraints for a group list
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GroupList {
    /// Range of allowed number of students per group
    pub students_per_group: RangeInclusive<NonZeroUsize>,
    /// Students that should be included in the list
    pub students: BTreeSet<usize>,
    /// Range of allowed number of groups in the list
    pub group_count: RangeInclusive<usize>,
}

/// Description of all the data describing a colloscope constraints
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ColloscopeConstraints {
    /// List of students with their relevant constraints
    pub students: Vec<Student>,
    /// List of groups constraints
    pub group_lists: Vec<GroupList>,
}
