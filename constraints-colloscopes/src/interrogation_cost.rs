use crate::native_extras::{MyBundle, all_slots, extra_var, weeks_for_slot};
use crate::types::ExtraVarName;
use collomatique_binding_colloscopes::vars::VarEnv;
use collomatique_ilp::linexpr::LinExpr;

pub fn build(env: &VarEnv) -> MyBundle {
    let mut expr = LinExpr::constant(0.0);
    let mut has_terms = false;

    for slot in all_slots(env) {
        let Some(slot_data) = env.slots.find_slot(slot) else {
            continue;
        };
        if slot_data.cost == 0 {
            continue;
        }

        for week in weeks_for_slot(env, slot) {
            expr = expr
                + f64::from(slot_data.cost)
                    * LinExpr::var(extra_var(ExtraVarName::InterrogationHasGroups {
                        slot,
                        week,
                    }));
            has_terms = true;
        }
    }

    if has_terms {
        MyBundle::new().with_minimize(1.0, expr)
    } else {
        MyBundle::new()
    }
}
