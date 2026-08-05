//! Ascending fill order (roadmap §2.3): a group may be non-empty only if
//! the previous one is. Empty groups form a suffix — which doubles as
//! symmetry breaking, and lets `convert.rs` compact defensively rather than
//! search.

use crate::extras::{MyBundle, V, extra_var};
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for list in env.lists() {
        // `slot_count` is always at least 1, so the subtraction is safe and
        // a single-slot list yields the empty range — no `< 2` guard is
        // needed, unlike the colloscope crate's group iterator.
        for group in 0..env.slot_count(list) - 1 {
            let current =
                IntLinExpr::<V>::var(extra_var(ExtraVarName::GroupHasStudents { list, group }));
            let next = IntLinExpr::<V>::var(extra_var(ExtraVarName::GroupHasStudents {
                list,
                group: group + 1,
            }));
            bundle = bundle.with_constraint(
                current.geq(&next),
                ConstraintDesc::GroupFilledByAscendingOrder { list, group },
            );
        }
    }
    bundle
}
