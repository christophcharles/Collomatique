//! assignments submodule
//!
//! This module defines the assignments entry for the JSON description
//!
use super::*;

use std::collections::{BTreeMap, BTreeSet};

/// JSON desc of assignments
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Map {
    /// map between period ids and their assignments
    ///
    /// For each period, we have a period assignment described by [PeriodAssignment]
    #[serde(with = "serde_with::rust::maps_duplicate_key_is_error")]
    pub period_map: BTreeMap<u64, PeriodAssignments>,
}

/// JSON desc of assignments for a specific period
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodAssignments {
    pub subject_map: BTreeMap<u64, SubjectAssignments>,
}

/// JSON desc of assignments for a specific subject on a given period
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectAssignments {
    pub assigned_students: BTreeSet<u64>,
}

impl From<&collomatique_state_colloscopes::assignments::PeriodAssignments> for PeriodAssignments {
    fn from(value: &collomatique_state_colloscopes::assignments::PeriodAssignments) -> Self {
        PeriodAssignments {
            subject_map: value
                .subject_map
                .iter()
                .map(|(subject_id, assigned_students)| {
                    (
                        subject_id.inner(),
                        SubjectAssignments {
                            assigned_students: assigned_students
                                .iter()
                                .map(|x| x.inner())
                                .collect(),
                        },
                    )
                })
                .collect(),
        }
    }
}

impl From<PeriodAssignments>
    for collomatique_state_colloscopes::assignments::PeriodAssignmentsExternalData
{
    fn from(value: PeriodAssignments) -> Self {
        collomatique_state_colloscopes::assignments::PeriodAssignmentsExternalData {
            subject_map: value
                .subject_map
                .into_iter()
                .map(|(subject_id, subject_assignments)| {
                    (subject_id, subject_assignments.assigned_students)
                })
                .collect(),
        }
    }
}
