use crate::extras::{MyBundle, V, base_var, extra_var, groups_for_interrogation, weeks_for_slot};
use crate::types::{ExtraVarName, ProgressiveConstraint, QualityConstraint};
use crate::vars::{Var, VarEnv};
use collomatique_ilp::int_linexpr::IntLinExpr;

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();

    for subject_id in env.slots.subjects_with_slots() {
        let Some(subject) = env.subjects.find_subject(subject_id) else {
            continue;
        };
        let Some(params) = subject.parameters.interrogation_parameters.as_ref() else {
            continue;
        };
        let min_groups = params.groups_per_interrogation.start().get();
        let max_groups = params.groups_per_interrogation.end().get();

        for (slot_id, slot_data) in env
            .slots
            .slots_for_subject(subject_id)
            .into_iter()
            .flatten()
        {
            let slot = *slot_id;
            for week in weeks_for_slot(env, slot_data, &subject.excluded_periods) {
                let groups = groups_for_interrogation(env, subject_id, week);
                if groups.is_empty() {
                    continue;
                }

                let sum: IntLinExpr<V> = groups
                    .iter()
                    .map(|&group| {
                        IntLinExpr::var(base_var(Var::GroupInInterrogation { slot, week, group }))
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
