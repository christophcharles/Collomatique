use collo_ml::traits::FieldConversionError;
use collo_ml::{DatabaseConnection, EvalObject, ExprType, ExprValue};
use collomatique_state_colloscopes::colloscope_params::Parameters;
use std::collections::{BTreeSet, HashMap};

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
    type Cache = ();

    fn objects_with_typ(_env: &Self::Env, _name: &str) -> BTreeSet<Self> {
        BTreeSet::new()
    }

    fn typ_name(&self) -> String {
        match *self {}
    }

    fn type_id_to_name(type_id: std::any::TypeId) -> Result<String, FieldConversionError> {
        Err(FieldConversionError::UnknownTypeId(type_id))
    }

    fn field_access<D: DatabaseConnection>(
        &self,
        _env: &Self::Env,
        _cache: &mut Self::Cache,
        _field: &str,
    ) -> Option<ExprValue<Self, D>> {
        match *self {}
    }

    fn type_schemas() -> HashMap<String, HashMap<String, ExprType>> {
        HashMap::new()
    }
}
