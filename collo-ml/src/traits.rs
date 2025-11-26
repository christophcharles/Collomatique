use crate::eval::ExprValue;
use crate::semantics::ExprType;
use collomatique_ilp::UsableData;
use std::collections::{BTreeSet, HashMap};

pub trait EvalObject: UsableData {
    type Env;

    fn objects_with_typ(env: &Self::Env, name: &str) -> BTreeSet<Self>;
    fn typ_name(&self, env: &Self::Env) -> String;
    fn field_access(&self, env: &Self::Env, field: &str) -> Option<ExprValue<Self>>;
    fn type_schemas() -> HashMap<String, HashMap<String, ExprType>>;
    fn pretty_print(&self, _env: &Self::Env) -> Option<String> {
        None
    }
}

pub trait ViewObject {
    type ObjectId: EvalObject;

    fn field_schema() -> HashMap<String, ExprType>;
    fn get_field(&self, field: &str) -> Option<ExprValue<Self::ObjectId>>;
    fn pretty_print(&self) -> Option<String> {
        None
    }
}

pub trait ViewBuilder<Env, Id> {
    type Object: ViewObject;

    fn build(env: &Env, id: &Id) -> Option<Self::Object>;
    fn enumerate(env: &Env) -> BTreeSet<Id>;
}
