mod incompats;
mod interrogation_cost;
mod limits;

use crate::native_extras::MyBundle;
use crate::vars::VarEnv;

pub fn build(env: &VarEnv) -> MyBundle {
    let bundle = incompats::build(env);
    let bundle = bundle
        .merge(limits::build(env))
        .expect("no duplicate extras from misc");
    bundle
        .merge(interrogation_cost::build(env))
        .expect("no duplicate extras from misc")
}
