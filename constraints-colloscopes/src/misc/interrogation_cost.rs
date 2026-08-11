use crate::extras::{MyBundle, extra_var, groups_for_interrogation, weeks_for_slot};
use crate::types::ExtraVarName;
use crate::vars::VarEnv;
use collomatique_ilp::linexpr::LinExpr;

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut expr = LinExpr::constant(0.0);
    let mut has_terms = false;

    for subject_id in env.slots.subjects_with_slots() {
        let Some(subject) = env.subjects.find_subject(subject_id) else {
            continue;
        };
        for (slot_id, slot_data) in env
            .slots
            .slots_for_subject(subject_id)
            .into_iter()
            .flatten()
        {
            let slot = *slot_id;
            if slot_data.cost == 0 {
                continue;
            }

            for week in weeks_for_slot(env, slot_data, &subject.excluded_periods) {
                // Only weeks with a group list associated for this (period, subject) declare
                // the InterrogationHasGroups extra (see extras.rs); reference it only there,
                // matching group_count_per_interrogation.rs. Without an association the subject
                // is not interrogated that week, so the slot contributes zero cost.
                if groups_for_interrogation(env, subject_id, week).is_empty() {
                    continue;
                }
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
