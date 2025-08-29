//! This modules defines all the main constraints to build a colloscope.

use collomatique_time as time;

use std::collections::BTreeSet;
use std::num::NonZeroUsize;
use std::ops::RangeInclusive;

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
