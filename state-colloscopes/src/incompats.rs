//! Incompats submodule
//!
//! This module defines the relevant types to describes the schedule incompatibilities

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use crate::ids::{ColloscopeIncompatId, ColloscopeSubjectId, ColloscopeWeekPatternId, Id};

/// Description of the schedule incompatibilities
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
