//! Ops module
//!
//! This modules defines all modification operations that can
//! be done *in UI*. These are *natural* oeprations that a user
//! might want to do rather than elementary operations that appear
//! in [collomatique_state_colloscopes] and that are assembled into
//! more complete operations.
//!
//! Concretly any op defined here is consistituted of [collomatique_state_colloscopes::Op]
//! but these are more *natural* operations that will correspond
//! to a simple command in a cli or a click of a button in a gui.
//!

use collomatique_state_colloscopes::Data;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod cascade;
pub use cascade::*;

// Not glob-re-exported: the renderers are a named vocabulary, called as
// `collomatique_ops::rendering::render_week(…)` — by the warning texts here and
// by gtk4, so a warning and the UI describe the same entity with the same
// words.
pub mod rendering;

// Private: the crate's only rendering door for warnings is
// [cascade::CascadeWarning::text].
mod warning_text;

#[cfg(test)]
mod test_utils;

pub mod general_planning;
pub use general_planning::*;
pub mod subjects;
pub use subjects::*;
pub mod teachers;
pub use teachers::*;
pub mod students;
pub use students::*;
pub mod assignments;
pub use assignments::*;
pub mod week_patterns;
pub use week_patterns::*;
pub mod slots;
pub use slots::*;
pub mod incompatibilities;
pub use incompatibilities::*;
pub mod pairings;
pub use pairings::*;
pub mod slot_pairings;
pub use slot_pairings::*;
pub mod group_lists;
pub use group_lists::*;
pub mod settings;
pub use settings::*;
pub mod balancing;
pub use balancing::*;
pub mod colloscope;
pub use colloscope::*;
pub mod export_config;
pub use export_config::*;

pub type Desc = (OpCategory, String);

#[derive(Debug, Clone)]
pub enum OpCategory {
    None,
    GeneralPlanning,
    Subjects,
    Teachers,
    Students,
    Assignments,
    WeekPatterns,
    Slots,
    Incompatibilities,
    Pairings,
    SlotPairings,
    GroupLists,
    Settings,
    Balancing,
    Colloscope,
    ExportConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateOp {
    GeneralPlanning(GeneralPlanningUpdateOp),
    Subjects(SubjectsUpdateOp),
    Teachers(TeachersUpdateOp),
    Students(StudentsUpdateOp),
    Assignments(AssignmentsUpdateOp),
    WeekPatterns(WeekPatternsUpdateOp),
    Slots(SlotsUpdateOp),
    Incompatibilities(IncompatibilitiesUpdateOp),
    Pairings(PairingsUpdateOp),
    SlotPairings(SlotPairingsUpdateOp),
    GroupLists(GroupListsUpdateOp),
    Settings(SettingsUpdateOp),
    Balancing(BalancingUpdateOp),
    Colloscope(ColloscopeUpdateOp),
    ExportConfig(ExportConfigUpdateOp),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateError {
    #[error(transparent)]
    GeneralPlanning(#[from] GeneralPlanningUpdateError),
    #[error(transparent)]
    Subjects(#[from] SubjectsUpdateError),
    #[error(transparent)]
    Teachers(#[from] TeachersUpdateError),
    #[error(transparent)]
    Students(#[from] StudentsUpdateError),
    #[error(transparent)]
    Assignments(#[from] AssignmentsUpdateError),
    #[error(transparent)]
    WeekPatterns(#[from] WeekPatternsUpdateError),
    #[error(transparent)]
    Slots(#[from] SlotsUpdateError),
    #[error(transparent)]
    Incompatibilities(#[from] IncompatibilitiesUpdateError),
    #[error(transparent)]
    Pairings(#[from] PairingsUpdateError),
    #[error(transparent)]
    SlotPairings(#[from] SlotPairingsUpdateError),
    #[error(transparent)]
    GroupLists(#[from] GroupListsUpdateError),
    #[error(transparent)]
    Settings(#[from] SettingsUpdateError),
    #[error(transparent)]
    Balancing(#[from] BalancingUpdateError),
    #[error(transparent)]
    Colloscope(#[from] ColloscopeUpdateError),
    #[error(transparent)]
    ExportConfig(#[from] ExportConfigUpdateError),
}

impl UpdateOp {
    /// Applies the op to `session`: the fifteen families all know how to write
    /// themselves as elementary ops on a [CascadeSession], and this is the
    /// dispatch that reaches the right one.
    ///
    /// The id the op created, if any, comes back widened to a
    /// [collomatique_state_colloscopes::NewId] — the families answer with their
    /// own id type.
    fn apply_to_session<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<Option<collomatique_state_colloscopes::NewId>, UpdateError> {
        match self {
            UpdateOp::GeneralPlanning(period_op) => {
                let result = period_op.apply_to_session(session)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Subjects(subject_op) => {
                let result = subject_op.apply_to_session(session)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Teachers(teacher_op) => {
                let result = teacher_op.apply_to_session(session)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Students(student_op) => {
                let result = student_op.apply_to_session(session)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Assignments(assignment_op) => {
                assignment_op.apply_to_session(session)?;
                Ok(None)
            }
            UpdateOp::WeekPatterns(week_pattern_op) => {
                let result = week_pattern_op.apply_to_session(session)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Slots(slot_op) => {
                let result = slot_op.apply_to_session(session)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Incompatibilities(incompat_op) => {
                let result = incompat_op.apply_to_session(session)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Pairings(pairing_op) => {
                let result = pairing_op.apply_to_session(session)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::SlotPairings(slot_pairing_op) => {
                let result = slot_pairing_op.apply_to_session(session)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::GroupLists(group_list_op) => {
                let result = group_list_op.apply_to_session(session)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Settings(settings_op) => {
                settings_op.apply_to_session(session)?;
                Ok(None)
            }
            UpdateOp::Balancing(balancing_op) => {
                balancing_op.apply_to_session(session)?;
                Ok(None)
            }
            UpdateOp::Colloscope(colloscope_op) => {
                colloscope_op.apply_to_session(session)?;
                Ok(None)
            }
            UpdateOp::ExportConfig(export_config_op) => {
                export_config_op.apply_to_session(session)?;
                Ok(None)
            }
        }
    }
}

impl UpdateOp {
    pub fn get_desc(&self) -> (OpCategory, String) {
        match self {
            UpdateOp::GeneralPlanning(period_op) => period_op.get_desc(),
            UpdateOp::Subjects(subject_op) => subject_op.get_desc(),
            UpdateOp::Teachers(teacher_op) => teacher_op.get_desc(),
            UpdateOp::Students(student_op) => student_op.get_desc(),
            UpdateOp::Assignments(assignment_op) => assignment_op.get_desc(),
            UpdateOp::WeekPatterns(week_pattern_op) => week_pattern_op.get_desc(),
            UpdateOp::Slots(slot_op) => slot_op.get_desc(),
            UpdateOp::Incompatibilities(incompat_op) => incompat_op.get_desc(),
            UpdateOp::Pairings(pairing_op) => pairing_op.get_desc(),
            UpdateOp::SlotPairings(slot_pairing_op) => slot_pairing_op.get_desc(),
            UpdateOp::GroupLists(group_list_op) => group_list_op.get_desc(),
            UpdateOp::Settings(settings_op) => settings_op.get_desc(),
            UpdateOp::Balancing(balancing_op) => balancing_op.get_desc(),
            UpdateOp::Colloscope(colloscope_op) => colloscope_op.get_desc(),
            UpdateOp::ExportConfig(export_config_op) => export_config_op.get_desc(),
        }
    }

    /// Applies the op on a copy of `data` and hands the outcome back *without*
    /// installing it: the caller sees the repairs the cascade had to make
    /// ([CascadeResult::warnings]) before deciding whether to keep the new
    /// state. That is what the gui does — it shows them and lets the user
    /// cancel.
    ///
    /// The whole update — the composite's own elementary ops and every repair
    /// they cascaded — lands as a single history slot on
    /// [CascadeResult::new_state], so one undo takes the document back to where
    /// the op found it.
    ///
    /// On `Err` there is nothing to unwind: the session owns a clone of `data`
    /// and is dropped with it.
    pub fn dry_apply<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &T,
    ) -> Result<CascadeResult<T>, UpdateError> {
        let mut session = CascadeSession::new(data.clone());

        let new_id = self.apply_to_session(&mut session)?;

        let (new_state, warnings) = session.commit(self.get_desc());

        Ok(CascadeResult {
            warnings,
            new_id,
            new_state,
        })
    }

    /// Applies the op to `data` in place, dropping the warnings — for callers
    /// that have no way of showing them (the scripting api).
    /// [UpdateOp::dry_apply] is the one that keeps them.
    pub fn apply<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::NewId>, UpdateError> {
        let result = self.dry_apply(data)?;

        *data = result.new_state;

        Ok(result.new_id)
    }
}
