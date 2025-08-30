//! Incompats submodule
//!
//! This module defines the relevant types to describes the schedule incompatibilities

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

use crate::ids::{IncompatId, SubjectId, WeekPatternId};

/// Description of the schedule incompatibilities
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Incompats {
    /// Incompats for subjects
    ///
    /// Each item associates an incompat id to a schedule incompatibility
    pub incompat_map: BTreeMap<IncompatId, Incompatibility>,
}

/// Description of a single schedule incompat
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incompatibility {
    /// Subject the incompatibility is linked to
    pub subject_id: SubjectId,
    /// Name of the incompatibility for clarity
    pub name: String,
    /// Slots of time when the students might not be available
    ///
    /// This is given as a weekday, a start time and a duration
    pub slots: Vec<collomatique_time::SlotWithDuration>,
    /// Number of slots to force to be free in the above list
    pub minimum_free_slots: NonZeroU32,
    /// Week pattern for the incompatibility
    ///
    /// If `None`, this means every week
    pub week_pattern_id: Option<WeekPatternId>,
}

impl Incompatibility {
    /// Builds an interrogation slot from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [IncompatibilityExternalData::validate].
    pub(crate) unsafe fn from_external_data(
        external_data: IncompatibilityExternalData,
    ) -> Incompatibility {
        Incompatibility {
            subject_id: unsafe { SubjectId::new(external_data.subject_id) },
            name: external_data.name,
            slots: external_data.slots,
            minimum_free_slots: external_data.minimum_free_slots,
            week_pattern_id: external_data
                .week_pattern_id
                .map(|x| unsafe { WeekPatternId::new(x) }),
        }
    }
}

impl Incompats {
    /// Builds interrogation slots from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [SlotsExternalData::validate_all] and [SlotsExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: IncompatsExternalData) -> Incompats {
        Incompats {
            incompat_map: external_data
                .incompat_map
                .into_iter()
                .map(|(id, incompat)| {
                    (unsafe { IncompatId::new(id) }, unsafe {
                        Incompatibility::from_external_data(incompat)
                    })
                })
                .collect(),
        }
    }
}

/// Description of the schedule incompatibilities but unchecked
///
/// This structure is an unchecked equivalent of [Incompats].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct IncompatsExternalData {
    /// Incompats for subjects
    ///
    /// Each item associates an incompat id to a schedule incompatibility
    pub incompat_map: BTreeMap<u64, IncompatibilityExternalData>,
}

impl IncompatsExternalData {
    /// Checks the validity of all [IncompatsExternalData] in the map.
    pub fn validate_all(
        &self,
        subject_ids: &BTreeSet<u64>,
        week_pattern_ids: &BTreeSet<u64>,
    ) -> bool {
        self.incompat_map
            .iter()
            .all(|(_incompat_id, incompat)| incompat.validate(subject_ids, week_pattern_ids))
    }
}

/// Description of a single schedule incompat but unchecked
///
/// This structure is an unchecked equivalent of [Incompatibility].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncompatibilityExternalData {
    /// Subject the incompatibility is linked to
    pub subject_id: u64,
    /// Name of the incompatibility for clarity
    pub name: String,
    /// Slots of time when the students might not be available
    ///
    /// This is given as a weekday, a start time and a duration
    pub slots: Vec<collomatique_time::SlotWithDuration>,
    /// Number of slots to force to be free in the above list
    pub minimum_free_slots: NonZeroU32,
    /// Week pattern for the incompatibility
    ///
    /// If `None`, this means every week
    pub week_pattern_id: Option<u64>,
}

impl IncompatibilityExternalData {
    /// Checks the validity of a [SlotExternalData]
    pub fn validate(&self, subject_ids: &BTreeSet<u64>, week_pattern_ids: &BTreeSet<u64>) -> bool {
        if !subject_ids.contains(&self.subject_id) {
            return false;
        }
        if let Some(week_pattern_id) = &self.week_pattern_id {
            if !week_pattern_ids.contains(week_pattern_id) {
                return false;
            }
        }
        true
    }
}

impl From<Incompatibility> for IncompatibilityExternalData {
    fn from(value: Incompatibility) -> Self {
        IncompatibilityExternalData {
            subject_id: value.subject_id.inner(),
            name: value.name,
            slots: value.slots,
            minimum_free_slots: value.minimum_free_slots,
            week_pattern_id: value.week_pattern_id.map(|x| x.inner()),
        }
    }
}
