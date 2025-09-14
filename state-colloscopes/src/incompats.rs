//! Incompats submodule
//!
//! This module defines the relevant types to describes the schedule incompatibilities

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

use crate::ids::{ColloscopeIncompatId, ColloscopeSubjectId, ColloscopeWeekPatternId, Id};

/// Description of the schedule incompatibilities
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incompats<IncompatId: Id, SubjectId: Id, WeekPatternId: Id> {
    /// Incompats for subjects
    ///
    /// Each item associates an incompat id to a schedule incompatibility
    pub incompat_map: BTreeMap<IncompatId, Incompatibility<SubjectId, WeekPatternId>>,
}

impl<IncompatId: Id, SubjectId: Id, WeekPatternId: Id> Default
    for Incompats<IncompatId, SubjectId, WeekPatternId>
{
    fn default() -> Self {
        Incompats {
            incompat_map: BTreeMap::new(),
        }
    }
}

/// Description of a single schedule incompat
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incompatibility<SubjectId: Id, WeekPatternId: Id> {
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

impl<SubjectId: Id, WeekPatternId: Id> Incompatibility<SubjectId, WeekPatternId> {
    /// Builds an interrogation slot from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [IncompatibilityExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: IncompatibilityExternalData) -> Self {
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

    pub(crate) fn duplicate_with_id_maps(
        &self,
        subjects_map: &BTreeMap<SubjectId, ColloscopeSubjectId>,
        week_patterns_map: &BTreeMap<WeekPatternId, ColloscopeWeekPatternId>,
    ) -> Option<Incompatibility<ColloscopeSubjectId, ColloscopeWeekPatternId>> {
        let week_pattern_id = match &self.week_pattern_id {
            Some(id) => Some(*week_patterns_map.get(id)?),
            None => None,
        };

        Some(Incompatibility {
            subject_id: *subjects_map.get(&self.subject_id)?,
            name: self.name.clone(),
            slots: self.slots.clone(),
            minimum_free_slots: self.minimum_free_slots.clone(),
            week_pattern_id,
        })
    }
}

impl<IncompatId: Id, SubjectId: Id, WeekPatternId: Id>
    Incompats<IncompatId, SubjectId, WeekPatternId>
{
    /// Builds interrogation slots from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [SlotsExternalData::validate_all] and [SlotsExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: IncompatsExternalData) -> Self {
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

    pub(crate) fn duplicate_with_id_maps(
        &self,
        incompats_map: &BTreeMap<IncompatId, ColloscopeIncompatId>,
        subjects_map: &BTreeMap<SubjectId, ColloscopeSubjectId>,
        week_patterns_map: &BTreeMap<WeekPatternId, ColloscopeWeekPatternId>,
    ) -> Option<Incompats<ColloscopeIncompatId, ColloscopeSubjectId, ColloscopeWeekPatternId>> {
        let mut incompat_map = BTreeMap::new();

        for (incompat_id, incompat) in &self.incompat_map {
            let new_id = incompats_map.get(incompat_id)?;
            let new_incompat = incompat.duplicate_with_id_maps(subjects_map, week_patterns_map)?;
            incompat_map.insert(*new_id, new_incompat);
        }

        Some(Incompats { incompat_map })
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

impl<SubjectId: Id, WeekPatternId: Id> From<Incompatibility<SubjectId, WeekPatternId>>
    for IncompatibilityExternalData
{
    fn from(value: Incompatibility<SubjectId, WeekPatternId>) -> Self {
        IncompatibilityExternalData {
            subject_id: value.subject_id.inner(),
            name: value.name,
            slots: value.slots,
            minimum_free_slots: value.minimum_free_slots,
            week_pattern_id: value.week_pattern_id.map(|x| x.inner()),
        }
    }
}
