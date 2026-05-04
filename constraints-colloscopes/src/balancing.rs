mod avoid_twice_in_a_row;
mod helpers;

use crate::native_extras::MyBundle;
use collomatique_binding_colloscopes::vars::VarEnv;

pub fn build(env: &VarEnv) -> MyBundle {
    avoid_twice_in_a_row::build(env)
}
