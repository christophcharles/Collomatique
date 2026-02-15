use collo_ml::{EvalObject, ExprType};
use collomatique_state_colloscopes::colloscope_params::Parameters;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Env {
    pub(crate) params: Parameters,
}

impl Env {
    pub fn get_params(&self) -> &Parameters {
        &self.params
    }
}

impl From<Parameters> for Env {
    fn from(value: Parameters) -> Self {
        Env { params: value }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObjectId {}

impl EvalObject for ObjectId {
    type Env = Env;

    fn type_schemas() -> HashMap<String, HashMap<String, ExprType>> {
        HashMap::new()
    }
}
