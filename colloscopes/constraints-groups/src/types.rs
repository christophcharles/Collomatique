//! Problem-description types.

use crate::vars::GroupListIdx;
use collomatique_state_colloscopes::StudentId;

/// Names of the extra variables. Both families exist to linearize the
/// *collision objective* — the square of a partner distribution — and both are
/// defined by one-sided rows rather than a full equivalence; see
/// `crate::extras` for why that is sound under a maximize.
///
/// The whole enumeration comes from one table, `crate::pairs::PairData`: what
/// is declared here, what the objective weighs and what a warm start valuates
/// are three readings of it.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ExtraVarName {
    /// 1 ⟺ the pair (with `a < b`) sits in group `group` of `list` — a
    /// *site*, in the vocabulary of `crate::pairs`. This is the `z_site` of
    /// the expansion, and the linear part of the square is one term per
    /// declared site.
    ///
    /// Declared only for a group both students belong to the list of, of
    /// target size at least 2, in a list that serves at least one (period,
    /// subject) pair. Anything else carries mass 0 in both directions and is
    /// filtered out of the enumeration (F1).
    Together {
        a: StudentId,
        b: StudentId,
        list: GroupListIdx,
        group: u32,
    },
    /// 1 ⟺ the pair meets in a group of target `target1` of `list1` **and**
    /// in a group of target `target2` of `list2` (`list1 < list2`). This is
    /// the quadratic part of the expansion: `(c + Σ m_i z_i)²` needs the
    /// products `z_i z_j`, and one variable per couple of *tiers* covers them
    /// all — the mass of a site depends on its group only through the target
    /// size, and the sites of one tier are mutually exclusive, so the sum of a
    /// tier's site binaries is itself 0/1.
    ///
    /// Never declared for two tiers of the same list: one group per student
    /// per list makes their product identically zero.
    Coincide {
        a: StudentId,
        b: StudentId,
        list1: GroupListIdx,
        target1: u32,
        list2: GroupListIdx,
        target2: u32,
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
