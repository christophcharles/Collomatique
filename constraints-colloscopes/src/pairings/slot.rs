use crate::extras::{
    MyBundle, V, base_var, extra_var, groups_for_interrogation, subject_interrogation_params,
    weeks_for_slot,
};
use crate::helpers::merge_objectified_weighted;
use crate::ids::GlobalWeek;
use crate::types::{ExtraVarName, ProgressiveConstraint, StructuralConstraint};
use crate::vars::{Var, VarEnv};
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::ids::SlotId;
use std::collections::BTreeSet;

fn slot_group_count_expr(
    env: &VarEnv,
    slot: SlotId,
    subject_id: collomatique_state_colloscopes::ids::SubjectId,
    week: GlobalWeek,
) -> IntLinExpr<V> {
    let groups = groups_for_interrogation(env, subject_id, week);
    groups
        .into_iter()
        .map(|group| IntLinExpr::var(base_var(Var::GroupInInterrogation { slot, week, group })))
        .sum()
}

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut output = MyBundle::new();

    for (rule_id, rule) in env.slot_pairings.slot_pairing_rule_map.iter() {
        let ant_slot_id = rule.antecedent().slot_id;
        let con_slot_id = rule.consequent().slot_id;

        let Some((subject_id, _)) = env.slots.find_slot_subject_and_position(ant_slot_id) else {
            continue;
        };
        let Some(subject) = env.subjects.find_subject(subject_id) else {
            continue;
        };
        let Some(params) = subject_interrogation_params(env, subject_id) else {
            continue;
        };
        let max_groups = i64::from(params.groups_per_interrogation.end().get());

        let Some(ant_slot_data) = env.slots.find_slot(ant_slot_id) else {
            continue;
        };
        let Some((con_subject_id, con_slot_data)) = env.slots.find_slot_with_subject(con_slot_id)
        else {
            continue;
        };
        if con_subject_id != subject_id {
            continue;
        }

        let combined_excluded: BTreeSet<_> = subject
            .excluded_periods
            .union(rule.excluded_periods())
            .copied()
            .collect();
        let ant_weeks: BTreeSet<_> = weeks_for_slot(env, ant_slot_data, &combined_excluded)
            .into_iter()
            .collect();
        let con_weeks: BTreeSet<_> = weeks_for_slot(env, con_slot_data, &combined_excluded)
            .into_iter()
            .collect();

        let mut hard_bundle = MyBundle::new();
        let mut soft_output = MyBundle::new();

        for &week in ant_weeks.intersection(&con_weeks) {
            // Only weeks with a group list associated for this (period, subject) declare the
            // InterrogationHasGroups extra and give the group-count sums any variables (see
            // extras.rs); without an association the subject is not interrogated that week, so
            // the pairing has nothing to constrain. Mirrors interrogation_cost.rs /
            // group_count_per_interrogation.rs.
            if groups_for_interrogation(env, subject_id, week).is_empty() {
                continue;
            }
            let mut single = MyBundle::new();
            let target = if rule.soft() {
                &mut single
            } else {
                &mut hard_bundle
            };

            match (rule.antecedent().should_have, rule.consequent().should_have) {
                (true, true) => {
                    let ant_count = slot_group_count_expr(env, ant_slot_id, subject_id, week);
                    let con_count = slot_group_count_expr(env, con_slot_id, subject_id, week);
                    *target = std::mem::take(target).with_constraint(
                        ant_count.leq(&(max_groups * con_count)),
                        ProgressiveConstraint::SlotPairingUsedImpliesUsed {
                            rule: rule_id,
                            week,
                        }
                        .into(),
                    );
                }
                (true, false) => {
                    let ant_count = slot_group_count_expr(env, ant_slot_id, subject_id, week);
                    let con_count = slot_group_count_expr(env, con_slot_id, subject_id, week);
                    if max_groups == 1 {
                        *target = std::mem::take(target).with_constraint(
                            con_count.leq(&(max_groups * (1i64 - ant_count))),
                            StructuralConstraint::SlotPairingUsedImpliesNotUsed {
                                rule: rule_id,
                                week,
                            }
                            .into(),
                        );
                    } else {
                        let has_groups =
                            IntLinExpr::<V>::var(extra_var(ExtraVarName::InterrogationHasGroups {
                                slot: ant_slot_id,
                                week,
                            }));
                        *target = std::mem::take(target).with_constraint(
                            con_count.leq(&(max_groups * (1i64 - has_groups))),
                            StructuralConstraint::SlotPairingUsedImpliesNotUsed {
                                rule: rule_id,
                                week,
                            }
                            .into(),
                        );
                    }
                }
                (false, true) => {
                    let ant_count = slot_group_count_expr(env, ant_slot_id, subject_id, week);
                    let con_count = slot_group_count_expr(env, con_slot_id, subject_id, week);
                    *target = std::mem::take(target).with_constraint(
                        (ant_count + con_count).geq(&IntLinExpr::constant(1)),
                        ProgressiveConstraint::SlotPairingNotUsedImpliesUsed {
                            rule: rule_id,
                            week,
                        }
                        .into(),
                    );
                }
                (false, false) => {
                    let ant_count = slot_group_count_expr(env, ant_slot_id, subject_id, week);
                    let con_count = slot_group_count_expr(env, con_slot_id, subject_id, week);
                    *target = std::mem::take(target).with_constraint(
                        con_count.leq(&(max_groups * ant_count)),
                        StructuralConstraint::SlotPairingNotUsedImpliesNotUsed {
                            rule: rule_id,
                            week,
                        }
                        .into(),
                    );
                }
            }

            if rule.soft() {
                soft_output = merge_objectified_weighted(
                    soft_output,
                    single,
                    ExtraVarName::SlotPairingsPenalty {
                        rule: rule_id,
                        week,
                    },
                    |_| crate::weights::BASE,
                );
            }
        }

        output = output
            .merge(hard_bundle)
            .expect("no duplicate extras from slot pairings (distinct rules)");
        output = output
            .merge(soft_output)
            .expect("no duplicate extras from slot pairings soft penalties");
    }

    output
}
