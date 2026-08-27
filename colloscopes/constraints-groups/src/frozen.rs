//! Placements the ILP polish can be told to keep.
//!
//! In practice these are the greedy's prefill output (see
//! [`GreedyOutcome`](crate::GreedyOutcome)): the whole groups phase one tiled
//! out of single cohorts and then never revised. Nothing here is
//! greedy-specific — the model builder takes any set of seats — but that is
//! the only producer.
//!
//! Not to be confused with `GroupListFilling::Prefilled`, the state-layer
//! filling mode every generated list uses, nor with the plan's
//! [`kept_lists`](crate::GenerationPlan::kept_lists), which are groupings the
//! generation does not touch at all rather than seats inside a list it builds.

use crate::vars::GroupListIdx;
use collomatique_state_colloscopes::StudentId;
use std::collections::BTreeMap;

/// A set of (list, student) → group seats to hold fixed.
///
/// Group indices are the model's: the greedy never compacts its groups
/// (`State::into_group_lists`) and its group count is the model's own, since
/// `VarEnv::group_count` counts the very targets
/// [`targets::balanced_targets`](crate::targets) hands the greedy, so both
/// sides number the same `⌈n / max⌉` groups.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrozenPlacements {
    seats: BTreeMap<(GroupListIdx, StudentId), u32>,
}

impl FrozenPlacements {
    pub fn new(seats: BTreeMap<(GroupListIdx, StudentId), u32>) -> Self {
        FrozenPlacements { seats }
    }

    /// Every seat, as `(list, student, group)`.
    pub fn iter(&self) -> impl Iterator<Item = (GroupListIdx, StudentId, u32)> + '_ {
        self.seats
            .iter()
            .map(|(&(list, student), &group)| (list, student, group))
    }

    /// How many seats are held fixed. A student appearing in three lists
    /// counts three times: a seat is a (list, student) pair.
    pub fn len(&self) -> usize {
        self.seats.len()
    }

    pub fn is_empty(&self) -> bool {
        self.seats.is_empty()
    }
}
