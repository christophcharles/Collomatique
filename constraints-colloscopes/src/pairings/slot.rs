use crate::extras::{
    MyBundle, V, base_var, extra_var, groups_for_interrogation, subject_interrogation_params,
    weeks_for_slot,
};
use crate::helpers::merge_objectified;
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

    for (&rule_id, rule) in &env.slot_pairings.slot_pairing_rule_map {
        let ant_slot_id = rule.antecedent.slot_id;
        let con_slot_id = rule.consequent.slot_id;

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

        let Some(subject_slots) = env.slots.subject_map.get(&subject_id) else {
            continue;
        };
        let Some(ant_slot_data) = subject_slots.find_slot(ant_slot_id) else {
            continue;
        };
        let Some(con_slot_data) = subject_slots.find_slot(con_slot_id) else {
            continue;
        };

        let combined_excluded: BTreeSet<_> = subject
            .excluded_periods
            .union(&rule.excluded_periods)
            .copied()
            .collect();
        let ant_weeks: BTreeSet<_> = weeks_for_slot(env, ant_slot_data, &combined_excluded)
            .into_iter()
            .collect();
        let con_weeks: BTreeSet<_> = weeks_for_slot(env, con_slot_data, &combined_excluded)
            .into_iter()
            .collect();

        let mut hard_bundle = MyBundle::new();
        let mut soft_bundle = MyBundle::new();

        for &week in ant_weeks.intersection(&con_weeks) {
            let target = if rule.soft {
                &mut soft_bundle
            } else {
                &mut hard_bundle
            };

            match (rule.antecedent.should_have, rule.consequent.should_have) {
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
        }

        output = output
            .merge(merge_objectified(
                hard_bundle,
                soft_bundle,
                ExtraVarName::SlotPairingsPenalty { rule: rule_id },
            ))
            .expect("no duplicate extras from slot pairings (distinct rules)");
    }

    output
}
