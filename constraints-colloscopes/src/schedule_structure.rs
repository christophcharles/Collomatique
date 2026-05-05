mod group_count_per_interrogation;
mod one_interrogation_at_once;

use crate::native_extras::MyBundle;
use crate::vars::VarEnv;

pub fn build(env: &VarEnv) -> MyBundle {
    let bundle = group_count_per_interrogation::build(env);
    bundle
        .merge(one_interrogation_at_once::build(env))
        .expect("no duplicate extras from schedule_structure")
}
