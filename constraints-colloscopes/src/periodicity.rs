mod amount_for_every_arbitrary_block;
mod amount_in_year;
mod exactly_periodic;
mod helpers;

use crate::native_extras::MyBundle;
use collomatique_binding_colloscopes::vars::VarEnv;

pub fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    bundle = amount_in_year::build(env, bundle);
    bundle = amount_for_every_arbitrary_block::build(env, bundle);
    bundle = exactly_periodic::build(env, bundle);
    bundle
}
