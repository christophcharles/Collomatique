//! Problem-description types.

use crate::vars::{GroupListIdx, SizeClassIdx};
use collomatique_state_colloscopes::StudentId;

/// Names of the extra variables (piece 7). The single family left is
/// objective-only, and it is defined by one-sided rows rather than a full
/// equivalence — see [`crate::extras`] for why that is sound here.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ExtraVarName {
    /// 1 if the pair (with `a < b`) shares some group in some list *of this
    /// size class*, and what the minimizing objective drives to 0 otherwise.
    /// Declared only for pairs that co-occur in at least one spec of the
    /// class; pinned to 1 when a kept list of the same size range already
    /// groups the pair.
    ///
    /// Split per class so that a cheap meeting in a tutorial group of twenty
    /// — where everyone meets everyone whatever the model does — cannot
    /// pre-pay, and thereby free, a colle pair.
    SharedPair {
        a: StudentId,
        b: StudentId,
        class: SizeClassIdx,
    },
}

/// One variant per constraint family (piece 8). A flat enum, no severity
/// tiers — the tier machinery of the colloscope crate feeds a gtk4 warning
/// display that has no counterpart here.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ConstraintDesc {
    /// `Σ_g StudentInGroup(list, student, g) == 1` — every student of a
    /// spec sits in exactly one group. Used to be enforced by the integer
    /// domain of the retired `Var::StudentGroup`.
    StudentInOneGroup {
        list: GroupListIdx,
        student: StudentId,
    },
    /// `Σ_s StudentInGroup(list, s, group) <= max_students`.
    StudentsPerGroupMax {
        list: GroupListIdx,
        group: u32,
        max_students: u32,
    },
    /// `Σ_s StudentInGroup(list, s, group) >= min_students` — every group
    /// of the list must reach the minimum. The count is exact
    /// ([`VarEnv::group_count`](crate::vars::VarEnv::group_count)), so no
    /// group may stay empty.
    StudentsPerGroupMin {
        list: GroupListIdx,
        group: u32,
        min_students: u32,
    },
    /// `Σ_g StudentInGhostGroup(student, g) == 1` — the template's own
    /// "exactly one group per student".
    GhostStudentInOneGroup { student: StudentId },
    /// `Σ_s StudentInGhostGroup(s, group) <= max_students`.
    GhostStudentsPerGroupMax { group: u32, max_students: u32 },
    /// `Σ_s StudentInGhostGroup(s, group) >= min_students`. The template is
    /// held to the same size discipline as a real list: left free of it, the
    /// objective would collapse it into a few degenerate groups and the
    /// grouping the real lists are asked to resemble would mean nothing.
    GhostStudentsPerGroupMin { group: u32, min_students: u32 },
}
