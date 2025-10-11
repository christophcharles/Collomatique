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

use collomatique_state::AppSession;
use collomatique_state_colloscopes::Data;

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

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
pub mod group_lists;
pub use group_lists::*;
pub mod rules;
pub use rules::*;
pub mod settings;
pub use settings::*;
pub mod colloscopes;
pub use colloscopes::*;

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
    GroupLists,
    Rules,
    Settings,
    Colloscopes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateOp {
    GeneralPlanning(GeneralPlanningUpdateOp),
    Subjects(SubjectsUpdateOp),
    Teachers(TeachersUpdateOp),
    Students(StudentsUpdateOp),
    Assignments(AssignmentsUpdateOp),
    WeekPatterns(WeekPatternsUpdateOp),
    Slots(SlotsUpdateOp),
    Incompatibilities(IncompatibilitiesUpdateOp),
    GroupLists(GroupListsUpdateOp),
    Rules(RulesUpdateOp),
    Settings(SettingsUpdateOp),
    Colloscopes(ColloscopesUpdateOp),
}

#[derive(Debug, Error, Serialize, Deserialize)]
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
    GroupLists(#[from] GroupListsUpdateError),
    #[error(transparent)]
    Rules(#[from] RulesUpdateError),
    #[error(transparent)]
    Settings(#[from] SettingsUpdateError),
    #[error(transparent)]
    Colloscopes(#[from] ColloscopesUpdateError),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum UpdateWarning {
    GeneralPlanning(GeneralPlanningUpdateWarning),
    Subjects(SubjectsUpdateWarning),
    Teachers(TeachersUpdateWarning),
    Students(StudentsUpdateWarning),
    Assignments(AssignmentsUpdateWarning),
    WeekPatterns(WeekPatternsUpdateWarning),
    Slots(SlotsUpdateWarning),
    Incompatibilities(IncompatibilitiesUpdateWarning),
    GroupLists(GroupListsUpdateWarning),
    Rules(RulesUpdateWarning),
    Settings(SettingsUpdateWarning),
    Colloscopes(ColloscopesUpdateWarning),
}

impl From<GeneralPlanningUpdateWarning> for UpdateWarning {
    fn from(value: GeneralPlanningUpdateWarning) -> Self {
        UpdateWarning::GeneralPlanning(value)
    }
}

impl From<SubjectsUpdateWarning> for UpdateWarning {
    fn from(value: SubjectsUpdateWarning) -> Self {
        UpdateWarning::Subjects(value)
    }
}

impl From<TeachersUpdateWarning> for UpdateWarning {
    fn from(value: TeachersUpdateWarning) -> Self {
        UpdateWarning::Teachers(value)
    }
}

impl From<StudentsUpdateWarning> for UpdateWarning {
    fn from(value: StudentsUpdateWarning) -> Self {
        UpdateWarning::Students(value)
    }
}

impl From<AssignmentsUpdateWarning> for UpdateWarning {
    fn from(value: AssignmentsUpdateWarning) -> Self {
        UpdateWarning::Assignments(value)
    }
}

impl From<WeekPatternsUpdateWarning> for UpdateWarning {
    fn from(value: WeekPatternsUpdateWarning) -> Self {
        UpdateWarning::WeekPatterns(value)
    }
}

impl From<SlotsUpdateWarning> for UpdateWarning {
    fn from(value: SlotsUpdateWarning) -> Self {
        UpdateWarning::Slots(value)
    }
}

impl From<IncompatibilitiesUpdateWarning> for UpdateWarning {
    fn from(value: IncompatibilitiesUpdateWarning) -> Self {
        UpdateWarning::Incompatibilities(value)
    }
}

impl From<GroupListsUpdateWarning> for UpdateWarning {
    fn from(value: GroupListsUpdateWarning) -> Self {
        UpdateWarning::GroupLists(value)
    }
}

impl From<RulesUpdateWarning> for UpdateWarning {
    fn from(value: RulesUpdateWarning) -> Self {
        UpdateWarning::Rules(value)
    }
}

impl From<SettingsUpdateWarning> for UpdateWarning {
    fn from(value: SettingsUpdateWarning) -> Self {
        UpdateWarning::Settings(value)
    }
}

impl From<ColloscopesUpdateWarning> for UpdateWarning {
    fn from(value: ColloscopesUpdateWarning) -> Self {
        UpdateWarning::Colloscopes(value)
    }
}

impl UpdateWarning {
    fn build_desc_from_data<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            UpdateWarning::GeneralPlanning(w) => w.build_desc_from_data(data),
            UpdateWarning::Subjects(w) => w.build_desc_from_data(data),
            UpdateWarning::Teachers(w) => w.build_desc_from_data(data),
            UpdateWarning::Students(w) => w.build_desc_from_data(data),
            UpdateWarning::Assignments(w) => w.build_desc_from_data(data),
            UpdateWarning::WeekPatterns(w) => w.build_desc_from_data(data),
            UpdateWarning::Slots(w) => w.build_desc_from_data(data),
            UpdateWarning::Incompatibilities(w) => w.build_desc_from_data(data),
            UpdateWarning::GroupLists(w) => w.build_desc_from_data(data),
            UpdateWarning::Rules(w) => w.build_desc_from_data(data),
            UpdateWarning::Settings(w) => w.build_desc_from_data(data),
            UpdateWarning::Colloscopes(w) => w.build_desc_from_data(data),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CleaningOp<T: Clone + std::fmt::Debug> {
    warning: T,
    op: UpdateOp,
}

impl<T: Clone + std::fmt::Debug + Into<UpdateWarning>> CleaningOp<T> {
    fn into_general_warning(self) -> CleaningOp<UpdateWarning> {
        CleaningOp {
            warning: self.warning.into(),
            op: self.op,
        }
    }
}

impl CleaningOp<UpdateWarning> {
    fn downcast<T: Clone + std::fmt::Debug + Into<UpdateWarning>>(
        x: Option<CleaningOp<T>>,
    ) -> Option<Self> {
        x.map(|x| x.into_general_warning())
    }
}

#[derive(Clone, Debug)]
pub struct RecApplyResult {
    pub warnings: BTreeSet<(UpdateWarning, String)>,
    pub new_id: Option<collomatique_state_colloscopes::NewId>,
}

pub struct DryResult<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>> {
    pub rec_apply_result: RecApplyResult,
    pub new_state: T,
}

impl UpdateOp {
    fn get_next_cleaning_op<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &T,
    ) -> Option<CleaningOp<UpdateWarning>> {
        match self {
            UpdateOp::GeneralPlanning(period_op) => {
                CleaningOp::downcast(period_op.get_next_cleaning_op(data))
            }
            UpdateOp::Subjects(subject_op) => {
                CleaningOp::downcast(subject_op.get_next_cleaning_op(data))
            }
            UpdateOp::Teachers(teacher_op) => {
                CleaningOp::downcast(teacher_op.get_next_cleaning_op(data))
            }
            UpdateOp::Students(student_op) => {
                CleaningOp::downcast(student_op.get_next_cleaning_op(data))
            }
            UpdateOp::Assignments(assignment_op) => {
                CleaningOp::downcast(assignment_op.get_next_cleaning_op(data))
            }
            UpdateOp::WeekPatterns(week_pattern_op) => {
                CleaningOp::downcast(week_pattern_op.get_next_cleaning_op(data))
            }
            UpdateOp::Slots(slot_op) => CleaningOp::downcast(slot_op.get_next_cleaning_op(data)),
            UpdateOp::Incompatibilities(incompat_op) => {
                CleaningOp::downcast(incompat_op.get_next_cleaning_op(data))
            }
            UpdateOp::GroupLists(group_list_op) => {
                CleaningOp::downcast(group_list_op.get_next_cleaning_op(data))
            }
            UpdateOp::Rules(rule_op) => CleaningOp::downcast(rule_op.get_next_cleaning_op(data)),
            UpdateOp::Settings(settings_op) => {
                CleaningOp::downcast(settings_op.get_next_cleaning_op(data))
            }
            UpdateOp::Colloscopes(colloscopes_op) => {
                CleaningOp::downcast(colloscopes_op.get_next_cleaning_op(data))
            }
        }
    }

    fn apply_no_cleaning<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::NewId>, UpdateError> {
        match self {
            UpdateOp::GeneralPlanning(period_op) => {
                let result = period_op.apply_no_cleaning(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Subjects(subject_op) => {
                let result = subject_op.apply_no_cleaning(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Teachers(teacher_op) => {
                let result = teacher_op.apply_no_cleaning(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Students(student_op) => {
                let result = student_op.apply_no_cleaning(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Assignments(assignment_op) => {
                assignment_op.apply_no_cleaning(data)?;
                Ok(None)
            }
            UpdateOp::WeekPatterns(week_pattern_op) => {
                let result = week_pattern_op.apply_no_cleaning(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Slots(slot_op) => {
                let result = slot_op.apply_no_cleaning(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Incompatibilities(incompat_op) => {
                let result = incompat_op.apply_no_cleaning(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::GroupLists(group_list_op) => {
                let result = group_list_op.apply_no_cleaning(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Rules(rule_op) => {
                let result = rule_op.apply_no_cleaning(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Settings(settings_op) => {
                settings_op.apply_no_cleaning(data)?;
                Ok(None)
            }
            UpdateOp::Colloscopes(colloscope_op) => {
                let result = colloscope_op.apply_no_cleaning(data)?;
                Ok(result.map(|x| x.into()))
            }
        }
    }

    fn rec_apply_no_session<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &mut T,
    ) -> Result<RecApplyResult, UpdateError> {
        let mut warnings = BTreeSet::new();

        while let Some(cleaning_op) = self.get_next_cleaning_op(data) {
            let warning_desc = cleaning_op
                .warning
                .build_desc_from_data(data)
                .expect("Warning should have a desc when applied on same state");
            warnings.insert((cleaning_op.warning, warning_desc));

            let result = cleaning_op.op.rec_apply_no_session(data)?;
            warnings.extend(result.warnings);
        }

        let new_id = self.apply_no_cleaning(data)?;

        Ok(RecApplyResult { warnings, new_id })
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
            UpdateOp::GroupLists(group_list_op) => group_list_op.get_desc(),
            UpdateOp::Rules(rule_op) => rule_op.get_desc(),
            UpdateOp::Settings(settings_op) => settings_op.get_desc(),
            UpdateOp::Colloscopes(colloscope_op) => colloscope_op.get_desc(),
        }
    }

    pub fn dry_apply<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &T,
    ) -> Result<DryResult<T>, UpdateError> {
        let mut session = AppSession::new(data.clone());

        let rec_apply_result = self.rec_apply_no_session(&mut session)?;

        Ok(DryResult {
            rec_apply_result,
            new_state: session.commit(self.get_desc()),
        })
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::NewId>, UpdateError> {
        let dry_result = self.dry_apply(data)?;

        *data = dry_result.new_state;

        Ok(dry_result.rec_apply_result.new_id)
    }
}
