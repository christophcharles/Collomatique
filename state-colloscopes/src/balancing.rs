//! Balancing submodule
//!
//! This module defines the relevant types to describe balancing requirements
//! for interrogation scheduling (teacher rotation, avoiding same teacher twice in a row).

use crate::ids::SubjectId;
use crate::soft_param::SoftParam;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

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
    /// Whether to rotate time slots across groups
    pub slot_rotation: Option<SoftParam<()>>,
    /// Whether to avoid having the same teacher twice in a row for a group
    pub avoid_twice_in_a_row: bool,
    /// Whether to enforce fair teacher distribution over the entire year
    pub year_teacher_rotation: bool,
    /// Whether to enforce fair teacher distribution within each period
    pub period_teacher_rotation: bool,
}

impl Default for BalancingOptions {
    fn default() -> Self {
        Self {
            teacher_rotation: Some(SoftParam {
                soft: true,
                value: (),
            }),
            slot_rotation: None,
            avoid_twice_in_a_row: true,
            year_teacher_rotation: false,
            period_teacher_rotation: false,
        }
    }
}

/// Errors for balancing operations
///
/// These errors can be returned when trying to modify [crate::Data] with a balancing op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum BalancingError {
    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),
    /// Subject does not have interrogations
    #[error("subject id ({0:?}) does not have interrogations")]
    SubjectHasNoInterrogation(SubjectId),
}
