use crate::native_extras::{MyBundle, V, extra_var, groups_for_interrogation, weeks_for_slot};
use crate::types::{ExtraVarName, ProgressiveConstraint, QualityConstraint};
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();

    for (&subject_id, subject_slots) in &env.slots.subject_map {
        let Some(subject) = env.subjects.find_subject(subject_id) else {
            continue;
        };
        let Some(params) = subject.parameters.interrogation_parameters.as_ref() else {
            continue;
        };
        let min_groups = params.groups_per_interrogation.start().get();
        let max_groups = params.groups_per_interrogation.end().get();

        for (slot_id, slot_data) in &subject_slots.ordered_slots {
            let slot = *slot_id;
            for week in weeks_for_slot(env, slot_data, &subject.excluded_periods) {
                let groups = groups_for_interrogation(env, subject_id, week);
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
                    ProgressiveConstraint::GroupCountPerInterrogationMin {
                        slot,
                        week,
                        min_groups,
                    }
                    .into(),
                );

                bundle = bundle.with_constraint(
                    sum.leq(&IntLinExpr::constant(max_groups as i64)),
                    QualityConstraint::GroupCountPerInterrogationMax {
                        slot,
                        week,
                        max_groups,
                    }
                    .into(),
                );
            }
        }
    }

    bundle
}
