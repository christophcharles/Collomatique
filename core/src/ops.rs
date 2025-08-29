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

#[derive(Debug)]
pub enum UpdateOp {
    GeneralPlanning(GeneralPlanningUpdateOp),
    Subjects(SubjectsUpdateOp),
    Teachers(TeachersUpdateOp),
    Students(StudentsUpdateOp),
    Assignments(AssignmentsUpdateOp),
    WeekPatterns(WeekPatternsUpdateOp),
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
        }
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data>>(
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
        }
    }
}
