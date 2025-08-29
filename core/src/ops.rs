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

pub type Desc = String;

#[derive(Debug)]
pub enum UpdateOp {
    GeneralPlanning(GeneralPlanningUpdateOp),
    Subjects(SubjectsUpdateOp),
    Teachers(TeachersUpdateOp),
    Students(StudentsUpdateOp),
    Assignments(AssignmentsUpdateOp),
    WeekPatterns(WeekPatternsUpdateOp),
    Slots(SlotsUpdateOp),
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
}

#[derive(Debug)]
pub enum UpdateWarning {
    GeneralPlanning(GeneralPlanningUpdateWarning),
    Subjects(SubjectsUpdateWarning),
    Teachers(TeachersUpdateWarning),
    Students(StudentsUpdateWarning),
    Assignments(AssignmentsUpdateWarning),
    WeekPatterns(WeekPatternsUpdateWarning),
    Slots(SlotsUpdateWarning),
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

impl UpdateWarning {
    pub fn build_desc<T: collomatique_state::traits::Manager<Data = Data, Desc = String>>(
        &self,
        data: &T,
    ) -> String {
        match self {
            UpdateWarning::GeneralPlanning(w) => w.build_desc(data),
            UpdateWarning::Subjects(w) => w.build_desc(data),
            UpdateWarning::Teachers(w) => w.build_desc(data),
            UpdateWarning::Students(w) => w.build_desc(data),
            UpdateWarning::Assignments(w) => w.build_desc(data),
            UpdateWarning::WeekPatterns(w) => w.build_desc(data),
            UpdateWarning::Slots(w) => w.build_desc(data),
        }
    }

    fn from_iter<T: Into<UpdateWarning>>(iter: impl IntoIterator<Item = T>) -> Vec<UpdateWarning> {
        iter.into_iter().map(|x| x.into()).collect()
    }
}

impl UpdateOp {
    pub fn get_desc(&self) -> String {
        match self {
            UpdateOp::GeneralPlanning(period_op) => period_op.get_desc(),
            UpdateOp::Subjects(subject_op) => subject_op.get_desc(),
            UpdateOp::Teachers(teacher_op) => teacher_op.get_desc(),
            UpdateOp::Students(student_op) => student_op.get_desc(),
            UpdateOp::Assignments(assignment_op) => assignment_op.get_desc(),
            UpdateOp::WeekPatterns(week_pattern_op) => week_pattern_op.get_desc(),
            UpdateOp::Slots(slot_op) => slot_op.get_desc(),
        }
    }

    pub fn get_warnings<T: collomatique_state::traits::Manager<Data = Data, Desc = String>>(
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
        }
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data, Desc = String>>(
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
        }
    }
}
