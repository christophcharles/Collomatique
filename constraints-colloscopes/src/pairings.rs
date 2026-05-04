mod slot;
mod subject;

use crate::native_extras::MyBundle;
use collomatique_binding_colloscopes::vars::VarEnv;

pub fn build(env: &VarEnv) -> MyBundle {
    let bundle = subject::build(env);
    bundle
        .merge(slot::build(env))
        .expect("no duplicate extras from pairings")
}
