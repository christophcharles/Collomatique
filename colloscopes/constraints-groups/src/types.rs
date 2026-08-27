//! Problem-description types.

use crate::vars::{GroupListIdx, RefGroupIdx, SizeClassIdx};
use collomatique_state_colloscopes::StudentId;

/// Names of the extra variables (piece 7). The live families are
/// objective-only, and both are defined by one-sided rows rather than a full
/// equivalence — see `crate::extras` for why that is sound here.
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
    /// 1 ⟺ some student of reference group `ref_group` sits in group `group`
    /// of `list`. Summed over the list's groups, this is the number of pieces
    /// the list breaks that reference group into — 1 when the list keeps it
    /// whole. That sum is the template term of the objective.
    ///
    /// This is what [`ExtraVarName::SharedPair`] cannot say. `SharedPair` is
    /// a step: once a pair has met anywhere in its size class, every further
    /// meeting is free, so nine lists agreeing on one grouping and one list
    /// differing cost exactly as much as five and five. A piece count is paid
    /// per list instead, so the cheapest plan is the one where every list
    /// reuses the same reference grouping.
    ///
    /// Declared only when the plan has a template, and only for (list,
    /// reference group) pairs that actually intersect — elsewhere it would be
    /// a vacuous 0.
    RefGroupInGroup {
        list: GroupListIdx,
        ref_group: RefGroupIdx,
        group: u32,
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
    /// `Σ_s StudentInGroup(list, s, group) == target` — the group holds
    /// exactly its balanced target
    /// ([`targets`](crate::targets)), which the spec's size range and the
    /// exact group count
    /// ([`VarEnv::group_count`](crate::vars::VarEnv::group_count)) leave no
    /// room to argue with. Replaces the min/max pair the model used to carry.
    GroupSize {
        list: GroupListIdx,
        group: u32,
        target: u32,
    },
    /// `StudentInGroup(list, student, group) == 1` — a seat the caller asked
    /// to hold fixed, in practice one the greedy's prefill phase froze.
    ///
    /// One row, not a whole column: `StudentInOneGroup` already forces the
    /// student's other groups to 0 once this one is 1.
    FrozenPlacement {
        list: GroupListIdx,
        student: StudentId,
        group: u32,
    },
}
