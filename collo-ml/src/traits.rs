use super::eval::ExprValue;
use collomatique_ilp::UsableData;
use std::collections::BTreeSet;

pub trait EvalObject: UsableData {
    type Env;

    fn objects_with_typ(env: &Self::Env, name: &str) -> BTreeSet<Self>;
    fn typ_name(&self, env: &Self::Env) -> String;
    fn field_access(&self, env: &Self::Env, field: &str) -> Option<ExprValue<Self>>;
    fn pretty_print(&self, _env: &Self::Env) -> Option<String> {
        None
    }
}
