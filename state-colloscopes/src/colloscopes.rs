//! Colloscopes submodule
//!
//! This module defines the relevant types to describes the colloscopes

use std::collections::BTreeMap;

use crate::ids::ColloscopeId;

/// Description of the colloscopes
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Colloscopes {
    /// List of colloscopes
    ///
    /// Each item associates an id to a colloscope description
    pub colloscope_map: BTreeMap<ColloscopeId, Colloscope>,
}

/// Description of a single colloscope
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Colloscope {
    /// Name for the colloscope
    pub name: String,
}
