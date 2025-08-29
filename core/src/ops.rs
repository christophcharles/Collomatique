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

use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::Data;

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
}

#[derive(Debug, Clone)]
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
}

#[derive(Debug, Error)]
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
}

#[derive(Debug, Clone)]
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

impl UpdateWarning {
    pub fn build_desc<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            UpdateWarning::GeneralPlanning(w) => w.build_desc(data),
            UpdateWarning::Subjects(w) => w.build_desc(data),
            UpdateWarning::Teachers(w) => w.build_desc(data),
            UpdateWarning::Students(w) => w.build_desc(data),
            UpdateWarning::Assignments(w) => w.build_desc(data),
            UpdateWarning::WeekPatterns(w) => w.build_desc(data),
            UpdateWarning::Slots(w) => w.build_desc(data),
            UpdateWarning::Incompatibilities(w) => w.build_desc(data),
            UpdateWarning::GroupLists(w) => w.build_desc(data),
            UpdateWarning::Rules(w) => w.build_desc(data),
        }
    }

    fn from_iter<T: Into<UpdateWarning>>(iter: impl IntoIterator<Item = T>) -> Vec<UpdateWarning> {
        iter.into_iter().map(|x| x.into()).collect()
    }
}

#[derive(Clone, Debug)]
struct CleaningOp<T: Clone + std::fmt::Debug> {
    warning_desc: String,
    warning: T,
    ops: Vec<UpdateOp>,
}

impl<T: Clone + std::fmt::Debug + Into<UpdateWarning>> CleaningOp<T> {
    fn into_general_warning(self) -> CleaningOp<UpdateWarning> {
        CleaningOp {
            warning_desc: self.warning_desc,
            warning: self.warning.into(),
            ops: self.ops,
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
            UpdateOp::GroupLists(group_list_op) => group_list_op.get_desc(),
            UpdateOp::Rules(rule_op) => rule_op.get_desc(),
        }
    }

    pub fn get_warnings<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &T,
    ) -> Vec<UpdateWarning> {
        match self {
            UpdateOp::GeneralPlanning(period_op) => {
                UpdateWarning::from_iter(period_op.get_warnings(data))
            }
            UpdateOp::Subjects(subject_op) => {
                UpdateWarning::from_iter(subject_op.get_warnings(data))
            }
            UpdateOp::Teachers(teacher_op) => {
                UpdateWarning::from_iter(teacher_op.get_warnings(data))
            }
            UpdateOp::Students(student_op) => {
                UpdateWarning::from_iter(student_op.get_warnings(data))
            }
            UpdateOp::Assignments(assignment_op) => {
                UpdateWarning::from_iter(assignment_op.get_warnings(data))
            }
            UpdateOp::WeekPatterns(week_pattern_op) => {
                UpdateWarning::from_iter(week_pattern_op.get_warnings(data))
            }
            UpdateOp::Slots(slot_op) => UpdateWarning::from_iter(slot_op.get_warnings(data)),
            UpdateOp::Incompatibilities(incompat_op) => {
                UpdateWarning::from_iter(incompat_op.get_warnings(data))
            }
            UpdateOp::GroupLists(group_list_op) => {
                UpdateWarning::from_iter(group_list_op.get_warnings(data))
            }
            UpdateOp::Rules(rule_op) => UpdateWarning::from_iter(rule_op.get_warnings(data)),
        }
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::NewId>, UpdateError> {
        match self {
            UpdateOp::GeneralPlanning(period_op) => {
                let result = period_op.apply(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Subjects(subject_op) => {
                let result = subject_op.apply(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Teachers(teacher_op) => {
                let result = teacher_op.apply(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Students(student_op) => {
                let result = student_op.apply(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Assignments(assignment_op) => {
                assignment_op.apply(data)?;
                Ok(None)
            }
            UpdateOp::WeekPatterns(week_pattern_op) => {
                let result = week_pattern_op.apply(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Slots(slot_op) => {
                let result = slot_op.apply(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Incompatibilities(incompat_op) => {
                let result = incompat_op.apply(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::GroupLists(group_list_op) => {
                let result = group_list_op.apply(data)?;
                Ok(result.map(|x| x.into()))
            }
            UpdateOp::Rules(rule_op) => {
                let result = rule_op.apply(data)?;
                Ok(result.map(|x| x.into()))
            }
        }
    }
}
