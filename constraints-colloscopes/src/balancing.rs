mod helpers;

use crate::native_extras::MyBundle;
use collomatique_binding_colloscopes::vars::VarEnv;

pub fn build(_env: &VarEnv) -> MyBundle {
    MyBundle::new()
}
