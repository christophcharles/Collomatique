//! Problem-description types. Both enums are deliberately empty in phase A:
//! piece 7 adds the reified extra variables, piece 8 the constraint
//! families. No severity tiers, ever — the tier machinery of the colloscope
//! crate feeds a gtk4 warning display that has no counterpart here.

/// Names of the reified extra variables (`StudentInGroup`,
/// `GroupHasStudents`, `PairInGroup`, `SharedPair` — piece 7).
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ExtraVarName {}

/// One variant per constraint family (max size, conditional min size,
/// ascending fill order — piece 8).
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ConstraintDesc {}
