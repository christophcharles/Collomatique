//! Balancing submodule
//!
//! This module defines the relevant types to describe balancing requirements
//! for interrogation scheduling (teacher rotation, avoiding same teacher twice in a row).

use crate::ids::SubjectId;
use crate::soft_param::SoftParam;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Description of the balancing configuration
///
/// Contains global balancing options and optional per-subject overrides.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Balancing {
    /// Global balancing options
    pub global: BalancingOptions,
    /// Optional per-subject overrides
    pub subjects: BTreeMap<SubjectId, BalancingOptions>,
}

impl Default for Balancing {
    fn default() -> Self {
        Self {
            global: BalancingOptions::default(),
            subjects: BTreeMap::new(),
        }
    }
}

/// Options for balancing interrogations
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalancingOptions {
    /// Whether to rotate teachers across groups
    pub teacher_rotation: Option<SoftParam<()>>,
    /// Whether to avoid having the same teacher twice in a row for a group
    pub avoid_twice_in_a_row: Option<SoftParam<()>>,
}

impl Default for BalancingOptions {
    fn default() -> Self {
        Self {
            teacher_rotation: Some(SoftParam {
                soft: true,
                value: (),
            }),
            avoid_twice_in_a_row: Some(SoftParam {
                soft: true,
                value: (),
            }),
        }
    }
}
