//! General settings submodule
//!
//! This module defines the relevant types to describes general settings

use crate::{ids::Id, ColloscopeStudentId};
use std::collections::BTreeMap;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

/// Description of the general settings
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings<StudentId: Id> {
    /// Global limits to impose during resolution
    pub global: Limits,
    /// Optional limits per students
    pub students: BTreeMap<StudentId, Limits>,
}

impl<StudentId: Id> Default for Settings<StudentId> {
    fn default() -> Self {
        Settings {
            global: Limits::default(),
            students: BTreeMap::new(),
        }
    }
}

impl<StudentId: Id> Settings<StudentId> {
    pub(crate) fn duplicate_with_id_maps(
        &self,
        students_map: &BTreeMap<StudentId, ColloscopeStudentId>,
    ) -> Option<Settings<ColloscopeStudentId>> {
        let mut students = BTreeMap::new();

        for (student_id, limits) in &self.students {
            let new_id = students_map.get(student_id)?;
            students.insert(*new_id, limits.clone());
        }

        Some(Settings {
            global: self.global.clone(),
            students,
        })
    }
}

/// Strict limits in resolution
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Limits {
    /// Number of interrogations for each student per week
    pub interrogations_per_week_min: Option<SoftParam<u32>>,
    /// Number of interrogations for each student per week
    pub interrogations_per_week_max: Option<SoftParam<u32>>,
    /// maximum number of interrogation in a single day for each student
    pub max_interrogations_per_day: Option<SoftParam<NonZeroU32>>,
}

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
