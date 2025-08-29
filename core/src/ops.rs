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

#[derive(Debug)]
pub enum UpdateOp {
    GeneralPlanning(GeneralPlanningUpdateOp),
}

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error(transparent)]
    GeneralPlanning(#[from] GeneralPlanningUpdateError),
}

impl UpdateOp {
    pub fn get_desc(&self) -> String {
        match self {
            UpdateOp::GeneralPlanning(period_op) => period_op.get_desc(),
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
        }
    }
}
