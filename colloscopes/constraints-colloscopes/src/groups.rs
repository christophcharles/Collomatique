mod forbidden_groups;
mod groups_filled_by_ascending_order;
mod students_have_groups;
mod students_per_group;
mod students_per_group_for_subject;

use crate::extras::MyBundle;
use crate::vars::VarEnv;

pub fn build(env: &VarEnv) -> MyBundle {
    let bundle = students_have_groups::build(env);
    let bundle = bundle
        .merge(students_per_group::build(env))
        .expect("no duplicate extras from groups");
    let bundle = bundle
        .merge(students_per_group_for_subject::build(env))
        .expect("no duplicate extras from groups");
    let bundle = bundle
        .merge(forbidden_groups::build(env))
        .expect("no duplicate extras from groups");
    bundle
        .merge(groups_filled_by_ascending_order::build(env))
        .expect("no duplicate extras from groups")
}
