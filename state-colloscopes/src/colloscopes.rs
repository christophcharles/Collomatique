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
    ) -> Result<(), super::ColloscopeError> {
        self.id_maps.validate_source_ids(main_params)?;
        self.id_maps.validate_new_ids(&self.params)?;
        self.params.check_invariants()?;

        Ok(())
    }
}

/// Description of actual colloscope data
///
/// the ids should be valid with respect to the corresponding params
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeData {
    pub period_map: BTreeMap<ColloscopePeriodId, ColloscopePeriod>,
    pub group_lists: BTreeMap<ColloscopeGroupListId, ColloscopeGroupList>,
}

/// Description of a single period in a colloscope
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopePeriod {
    /// Map between subjects and interrogations for these subjects
    ///
    /// Only subjects with interrogations are represented here
    pub subject_map: BTreeMap<ColloscopeSubjectId, ColloscopeSubject>,
}

/// Description of a single subject in a period in a colloscope
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeSubject {
    /// Map between slots and list of interrogations
    ///
    /// Each relevant slot are mapped to vec containing a cell
    /// for each week in the period. If there cannot be an interrogation
    /// there is is still a cell, but the option is set to None.
    ///
    /// If however there is a possible interrogation, it will be a Some
    /// value, even if no group is actually assigned. This will rather
    /// be described within the [ColloscopeInterrogation] struct.
    pub slots: BTreeMap<ColloscopeSlotId, Vec<Option<ColloscopeInterrogation>>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeInterrogation {
    /// List of groups assigned to the interrogation
    pub assigned_groups: Vec<u32>,
}

/// Description of a group list in a colloscope
///
/// This is basically map between students that are in the group lists
/// and actual group numbers
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeGroupList {
    pub groups_for_students: BTreeMap<ColloscopeStudentId, Option<u32>>,
}
