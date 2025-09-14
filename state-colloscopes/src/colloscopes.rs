//! Colloscopes submodule
//!
//! This module defines the relevant types to describes the colloscopes

use std::collections::BTreeMap;

use crate::ids::{
    ColloscopeGroupListId, ColloscopeId, ColloscopeIncompatId, ColloscopePeriodId,
    ColloscopeRuleId, ColloscopeSlotId, ColloscopeStudentId, ColloscopeSubjectId,
    ColloscopeTeacherId, ColloscopeWeekPatternId,
};
use crate::ids::{
    GroupListId, IncompatId, PeriodId, RuleId, SlotId, StudentId, SubjectId, TeacherId,
    WeekPatternId,
};

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
    /// Parameters at the time of the colloscope creation
    pub params: super::colloscope_params::ColloscopeParameters<
        ColloscopePeriodId,
        ColloscopeSubjectId,
        ColloscopeTeacherId,
        ColloscopeStudentId,
        ColloscopeWeekPatternId,
        ColloscopeSlotId,
        ColloscopeIncompatId,
        ColloscopeGroupListId,
        ColloscopeRuleId,
    >,
    /// ids map between the old ids and the one in the parameters copy
    pub id_maps: super::colloscope_params::ColloscopeIdMaps<
        PeriodId,
        SubjectId,
        TeacherId,
        StudentId,
        WeekPatternId,
        SlotId,
        IncompatId,
        GroupListId,
        RuleId,
    >,
}

impl Colloscope {
    pub(crate) fn check_invariants(
        &self,
        main_params: &super::colloscope_params::ColloscopeParameters<
            PeriodId,
            SubjectId,
            TeacherId,
            StudentId,
            WeekPatternId,
            SlotId,
            IncompatId,
            GroupListId,
            RuleId,
        >,
    ) {
        self.id_maps
            .validate_source_ids(main_params)
            .expect("Source ids in colloscope should be valid");
        self.id_maps
            .validate_new_ids(&self.params)
            .expect("Destination id should always be valid in colloscope id maps");
    }
}
