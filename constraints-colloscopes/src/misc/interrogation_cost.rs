use crate::native_extras::{MyBundle, extra_var, weeks_for_slot};
use crate::types::ExtraVarName;
use collomatique_binding_colloscopes::vars::VarEnv;
use collomatique_ilp::linexpr::LinExpr;

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut expr = LinExpr::constant(0.0);
    let mut has_terms = false;

    for (&subject_id, subject_slots) in &env.slots.subject_map {
        let Some(subject) = env.subjects.find_subject(subject_id) else {
            continue;
        };
        for (slot_id, slot_data) in &subject_slots.ordered_slots {
            let slot = *slot_id;
            if slot_data.cost == 0 {
                continue;
            }

            for week in weeks_for_slot(env, slot_data, &subject.excluded_periods) {
                expr = expr
                    + f64::from(slot_data.cost)
                        * LinExpr::var(extra_var(ExtraVarName::InterrogationHasGroups {
                            slot,
                            week,
                        }));
                has_terms = true;
            }
        }
    }

    if has_terms {
        MyBundle::new().with_minimize(1.0, expr)
    } else {
        MyBundle::new()
    }
}
