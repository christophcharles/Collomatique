use crate::native_extras::{
    MyBundle, V, all_slots, extra_var, groups_for_interrogation, slot_subject,
    subject_interrogation_params, weeks_for_slot,
};
use crate::types::{ConstraintDesc, ExtraVarName};
use collomatique_binding_colloscopes::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;

pub fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();

    for slot in all_slots(env) {
        let Some(subject) = slot_subject(env, slot) else {
            continue;
        };
        let Some(params) = subject_interrogation_params(env, subject) else {
            continue;
        };
        let min_groups = params.groups_per_interrogation.start().get();
        let max_groups = params.groups_per_interrogation.end().get();

        for week in weeks_for_slot(env, slot) {
            let groups = groups_for_interrogation(env, slot, week);
            if groups.is_empty() {
                continue;
            }

            let sum: IntLinExpr<V> = groups
                .iter()
                .map(|&group| {
                    IntLinExpr::var(extra_var(ExtraVarName::GroupInInterrogation {
                        slot,
                        week,
                        group,
                    }))
                })
                .sum();

            let has_groups = IntLinExpr::var(extra_var(ExtraVarName::InterrogationHasGroups {
                slot,
                week,
            }));
            bundle = bundle.with_constraint(
                sum.clone().geq(&(min_groups as i64 * has_groups)),
                ConstraintDesc::GroupCountPerInterrogationMin {
                    slot,
                    week,
                    min_groups,
                },
            );

            bundle = bundle.with_constraint(
                sum.leq(&IntLinExpr::constant(max_groups as i64)),
                ConstraintDesc::GroupCountPerInterrogationMax {
                    slot,
                    week,
                    max_groups,
                },
            );
        }
    }

    bundle
}
