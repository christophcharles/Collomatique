mod amount_for_every_arbitrary_block;
mod amount_in_year;
mod exactly_periodic;
mod helpers;
mod once_for_every_block_of_weeks;

use crate::native_extras::MyBundle;
use crate::vars::VarEnv;

pub fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    bundle = amount_in_year::build(env, bundle);
    bundle = amount_for_every_arbitrary_block::build(env, bundle);
    bundle = exactly_periodic::build(env, bundle);
    bundle = once_for_every_block_of_weeks::build(env, bundle);
    bundle
}
