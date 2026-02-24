//! Soft parameter submodule
//!
//! This module defines the [SoftParam] generic type used across
//! settings and balancing modules.

use serde::{Deserialize, Serialize};

/// Useful structure for parameters that might be enforced stricly or loosely (softly)
///
/// Some limits should be stricts (that is exactly followed), some should only be
/// a goal that should be optimized for. This structure encodes just that. We have
/// a goal stored in [Self::value] and whether this goal is a soft or hard one in [Self::soft].
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SoftParam<T> {
    /// If `true`, the goal is only softly enforced as part of an optimization function
    /// If `false`, a strict constraint will be associated to the goal
    pub soft: bool,
    /// Actual value for the goal
    pub value: T,
}
