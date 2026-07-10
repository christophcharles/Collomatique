use crate::extras::{
    MyBundle, V, active_slots_for_subject_week, extra_var, is_at_most_once_per_week,
    student_has_interrogation_in_expr,
};
use crate::helpers::merge_objectified_weighted;
use crate::ids::GlobalWeek;
use crate::types::{ExtraVarName, PreferenceConstraint};
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::ids::{PairingRuleId, StudentId, SubjectId};

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut output = MyBundle::new();

    for (&rule_id, rule) in &env.pairings.pairing_rule_map {
        let ant_subject = rule.antecedent.subject_id;
        let con_subject = rule.consequent.subject_id;

        let Some(ant_subj) = env.subjects.find_subject(ant_subject) else {
            continue;
        };
        let Some(con_subj) = env.subjects.find_subject(con_subject) else {
            continue;
        };

        let mut hard_bundle = MyBundle::new();
        let mut soft_output = MyBundle::new();

        let mut global_week_offset = 0usize;
        for (period_id, period_desc) in &env.periods.ordered_period_list {
            if rule.excluded_periods.contains(period_id)
                || ant_subj.excluded_periods.contains(period_id)
                || con_subj.excluded_periods.contains(period_id)
            {
                global_week_offset += period_desc.len();
                continue;
            }

            let ant_enrolled = env
                .assignments
                .period_map
                .get(period_id)
                .and_then(|pa| pa.subject_map.get(&ant_subject));
            let con_enrolled = env
                .assignments
                .period_map
                .get(period_id)
                .and_then(|pa| pa.subject_map.get(&con_subject));
            let (Some(ant_set), Some(con_set)) = (ant_enrolled, con_enrolled) else {
                global_week_offset += period_desc.len();
                continue;
            };

            let both_students: Vec<StudentId> = ant_set.intersection(con_set).copied().collect();

            for (local_idx, week_desc) in period_desc.iter().enumerate() {
                if !week_desc.interrogations {
                    continue;
                }
                let week = GlobalWeek(global_week_offset + local_idx);

                for &student in &both_students {
                    if rule.soft {
                        let mut single = MyBundle::new();
                        emit_pairing_constraint(
                            env,
                            &mut single,
                            rule_id,
                            student,
                            ant_subject,
                            con_subject,
                            rule.antecedent.should_have,
                            rule.consequent.should_have,
                            week,
                        );
                        soft_output = merge_objectified_weighted(
                            soft_output,
                            single,
                            ExtraVarName::PairingsPenalty {
                                rule: rule_id,
                                student,
                                week,
                            },
                            |_| crate::weights::BASE,
                        );
                    } else {
                        emit_pairing_constraint(
                            env,
                            &mut hard_bundle,
                            rule_id,
                            student,
                            ant_subject,
                            con_subject,
                            rule.antecedent.should_have,
                            rule.consequent.should_have,
                            week,
                        );
                    }
                }
            }

            global_week_offset += period_desc.len();
        }

        output = output
            .merge(hard_bundle)
            .expect("no duplicate extras from pairings (distinct rules)");
        output = output
            .merge(soft_output)
            .expect("no duplicate extras from pairings soft penalties");
    }

    output
}

#[allow(clippy::too_many_arguments)]
fn emit_pairing_constraint(
    env: &VarEnv,
    bundle: &mut MyBundle,
    rule_id: PairingRuleId,
    student: StudentId,
    ant_subject: SubjectId,
    con_subject: SubjectId,
    ant_should_have: bool,
    con_should_have: bool,
    week: GlobalWeek,
) {
    match (ant_should_have, con_should_have) {
        (true, true) => {
            let ant_expr = student_has_interrogation_in_expr(env, student, ant_subject, week);
            let con_expr = student_has_interrogation_in_expr(env, student, con_subject, week);
            let max_ant = active_slots_for_subject_week(env, ant_subject, week).len() as i64;
            *bundle = std::mem::take(bundle).with_constraint(
                ant_expr.leq(&(max_ant * con_expr)),
                PreferenceConstraint::PairingHavingImpliesHaving {
                    student,
                    week,
                    rule: rule_id,
                }
                .into(),
            );
        }
        (true, false) => {
            let con_expr = student_has_interrogation_in_expr(env, student, con_subject, week);
            let max_con = active_slots_for_subject_week(env, con_subject, week).len() as i64;
            if is_at_most_once_per_week(env, ant_subject) {
                let ant_expr = student_has_interrogation_in_expr(env, student, ant_subject, week);
                *bundle = std::mem::take(bundle).with_constraint(
                    con_expr.leq(&(max_con * (1i64 - ant_expr))),
                    PreferenceConstraint::PairingHavingImpliesNotHaving {
                        student,
                        week,
                        rule: rule_id,
                    }
                    .into(),
                );
            } else {
                let reif =
                    IntLinExpr::<V>::var(extra_var(ExtraVarName::StudentHasInterrogationIn {
                        student,
                        subject: ant_subject,
                        week,
                    }));
                *bundle = std::mem::take(bundle).with_constraint(
                    con_expr.leq(&(max_con * (1i64 - reif))),
                    PreferenceConstraint::PairingHavingImpliesNotHaving {
                        student,
                        week,
                        rule: rule_id,
                    }
                    .into(),
                );
            }
        }
        (false, true) => {
            let ant_expr = student_has_interrogation_in_expr(env, student, ant_subject, week);
            let con_expr = student_has_interrogation_in_expr(env, student, con_subject, week);
            *bundle = std::mem::take(bundle).with_constraint(
                (ant_expr + con_expr).geq(&IntLinExpr::constant(1)),
                PreferenceConstraint::PairingNotHavingImpliesHaving {
                    student,
                    week,
                    rule: rule_id,
                }
                .into(),
            );
        }
        (false, false) => {
            let ant_expr = student_has_interrogation_in_expr(env, student, ant_subject, week);
            let con_expr = student_has_interrogation_in_expr(env, student, con_subject, week);
            let max_con = active_slots_for_subject_week(env, con_subject, week).len() as i64;
            *bundle = std::mem::take(bundle).with_constraint(
                con_expr.leq(&(max_con * ant_expr)),
                PreferenceConstraint::PairingNotHavingImpliesNotHaving {
                    student,
                    week,
                    rule: rule_id,
                }
                .into(),
            );
        }
    }
}
