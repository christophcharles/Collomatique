//! Problem-description types.

use crate::vars::GroupListIdx;
use collomatique_state_colloscopes::StudentId;

/// Names of the reified extra variables (piece 7). All reifications are
/// full equivalences (roadmap §2.2): several solve strategies strip the
/// objective, and the values must stay correct there.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ExtraVarName {
    /// 1 ⟺ `StudentGroup { list, student } == group`.
    StudentInGroup {
        list: GroupListIdx,
        student: StudentId,
        group: u32,
    },
    /// 1 ⟺ at least one student sits in group `group` of list `list`.
    GroupHasStudents { list: GroupListIdx, group: u32 },
    /// 1 ⟺ `a` and `b` (with `a < b`) both sit in group `group` of list
    /// `list`. Declared for every pair of students sharing the list's spec.
    PairInGroup {
        a: StudentId,
        b: StudentId,
        list: GroupListIdx,
        group: u32,
    },
    /// 1 ⟺ the pair (with `a < b`) shares some group in some list.
    /// Declared only for pairs that co-occur in at least one spec; fixed
    /// to 1 when the pair is pinned by a kept list.
    SharedPair { a: StudentId, b: StudentId },
}

/// One variant per constraint family (piece 8). A flat enum, no severity
/// tiers — the tier machinery of the colloscope crate feeds a gtk4 warning
/// display that has no counterpart here.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ConstraintDesc {
    /// `Σ_s StudentInGroup(list, s, group) <= max_students`.
    StudentsPerGroupMax {
        list: GroupListIdx,
        group: u32,
        max_students: u32,
    },
    /// `Σ_s StudentInGroup(list, s, group) >= min_students ·
    /// GroupHasStudents(list, group)` — the minimum binds only non-empty
    /// groups.
    StudentsPerGroupMin {
        list: GroupListIdx,
        group: u32,
        min_students: u32,
    },
}
