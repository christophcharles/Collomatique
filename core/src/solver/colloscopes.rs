//! Colloscope Solver module
//!
//! This module contains the translation code
//! from [collomatique_state_colloscopes] to [collomatique_solver_colloscopes].

use collomatique_solver_colloscopes::base::ColloscopeProblem;
use collomatique_state_colloscopes::{Data, GroupListId, SlotId, StudentId, SubjectId};

type Problem = ColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>;

pub fn data_to_colloscope_problem(data: &Data) -> Problem {
    todo!()
}
