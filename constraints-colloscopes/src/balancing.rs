mod avoid_twice_in_a_row;
mod helpers;
mod rotation;
mod slot_rotation;
mod year_rotation;

use crate::native_extras::MyBundle;
use collomatique_binding_colloscopes::vars::VarEnv;

pub fn build(env: &VarEnv) -> MyBundle {
    let bundle = avoid_twice_in_a_row::build(env);
    bundle
        .merge(year_rotation::build(env))
        .expect("no duplicate extras from balancing")
        .merge(rotation::build(env))
        .expect("no duplicate extras from balancing")
        .merge(slot_rotation::build(env))
        .expect("no duplicate extras from balancing")
}
