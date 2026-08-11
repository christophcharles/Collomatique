mod avoid_twice_in_a_row;
mod avoid_twice_in_a_row_soft;
mod helpers;
mod period_rotation;
mod rotation;
mod slot_rotation;
mod year_rotation;

use crate::extras::MyBundle;
use crate::vars::VarEnv;

pub fn build(env: &VarEnv) -> MyBundle {
    let bundle = avoid_twice_in_a_row::build(env);
    bundle
        .merge(avoid_twice_in_a_row_soft::build(env))
        .expect("no duplicate extras from balancing")
        .merge(year_rotation::build(env))
        .expect("no duplicate extras from balancing")
        .merge(rotation::build(env))
        .expect("no duplicate extras from balancing")
        .merge(slot_rotation::build(env))
        .expect("no duplicate extras from balancing")
        .merge(period_rotation::build(env))
        .expect("no duplicate extras from balancing")
}
