use crate::extras::{
    MyBundle, V, extra_var, is_at_most_once_per_week, subject_interrogation_params,
};
use crate::helpers::{
    enrolled_students_for_subject, merge_objectified, slot_week_pairs_for_subject,
};
use crate::ids::GlobalWeek;
use crate::types::{ExtraVarName, InfeasibleConstraint, PreferenceConstraint};
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::ids::{StudentId, SubjectId, TeacherId};
use collomatique_state_colloscopes::subjects::SubjectPeriodicity;

use super::helpers::{
    count_student_teacher_expr, effective_balancing_option, rolling_windows,
    slot_week_pairs_for_teacher, subject_active_weeks, teachers_for_subject,
};

fn seen_this_week_expr(
    teacher_slot_week_pairs: &[(crate::ids::SlotId, GlobalWeek)],
    student: StudentId,
    week: GlobalWeek,
) -> IntLinExpr<V> {
    count_student_teacher_expr(teacher_slot_week_pairs, student, week, week)
}

fn build_window_constraints(
    env: &VarEnv,
    subject_id: SubjectId,
    slot_week_pairs: &[(crate::ids::SlotId, GlobalWeek)],
    window_size: usize,
    step_size: usize,
    is_soft: bool,
    hard_bundle: &mut MyBundle,
    soft_bundle: &mut MyBundle,
) {
    let active_weeks = subject_active_weeks(slot_week_pairs);
    let windows = rolling_windows(&active_weeks, window_size, step_size);
    let enrolled = enrolled_students_for_subject(env, subject_id);
    let teachers = teachers_for_subject(env, subject_id);

    for (first_week, last_week) in windows {
        for &student in &enrolled {
            for &teacher in &teachers {
                let teacher_pairs =
                    slot_week_pairs_for_teacher(slot_week_pairs, env, subject_id, teacher);
                let count =
                    count_student_teacher_expr(&teacher_pairs, student, first_week, last_week);
                let constraint = count.leq(&IntLinExpr::constant(1));
                let desc = PreferenceConstraint::BalancingAvoidTwiceInARow {
                    student,
                    subject: subject_id,
                    teacher,
                    first_week,
                    last_week,
                }
                .into();
                if is_soft {
                    *soft_bundle = std::mem::take(soft_bundle).with_constraint(constraint, desc);
                } else {
                    *hard_bundle = std::mem::take(hard_bundle).with_constraint(constraint, desc);
                }
            }
        }
    }
}

fn build_recursive_constraints(
    env: &VarEnv,
    subject_id: SubjectId,
    slot_week_pairs: &[(crate::ids::SlotId, GlobalWeek)],
    is_soft: bool,
    bundle: &mut MyBundle,
    hard_bundle: &mut MyBundle,
    soft_bundle: &mut MyBundle,
) {
    let active_weeks = subject_active_weeks(slot_week_pairs);
    if active_weeks.len() <= 1 {
        return;
    }

    let enrolled = enrolled_students_for_subject(env, subject_id);
    let teachers: Vec<TeacherId> = teachers_for_subject(env, subject_id).into_iter().collect();

    for &student in &enrolled {
        let per_teacher_pairs: Vec<(TeacherId, Vec<(crate::ids::SlotId, GlobalWeek)>)> = teachers
            .iter()
            .map(|&t| {
                (
                    t,
                    slot_week_pairs_for_teacher(slot_week_pairs, env, subject_id, t),
                )
            })
            .collect();

        for &(teacher, ref teacher_pairs) in &per_teacher_pairs {
            for (i, &week) in active_weeks.iter().enumerate() {
                let teacher_pairs = teacher_pairs.clone();
                let subject = subject_id;

                if i == 0 {
                    let seen_expr = seen_this_week_expr(&teacher_pairs, student, week);
                    *bundle = std::mem::take(bundle)
                        .and_reified(
                            ExtraVarName::IsLastTeacherSeen {
                                subject,
                                student,
                                teacher,
                                week,
                            },
                            move || vec![seen_expr.geq(&IntLinExpr::constant(1))],
                        )
                        .expect("no duplicate IsLastTeacherSeen");
                } else {
                    let prev_week = active_weeks[i - 1];
                    let seen_expr = seen_this_week_expr(&teacher_pairs, student, week);
                    let prev_var = IntLinExpr::var(extra_var(ExtraVarName::IsLastTeacherSeen {
                        subject,
                        student,
                        teacher,
                        week: prev_week,
                    }));

                    let others_seen: IntLinExpr<V> = per_teacher_pairs
                        .iter()
                        .filter(|&&(t, _)| t != teacher)
                        .map(|&(_, ref pairs)| seen_this_week_expr(pairs, student, week))
                        .sum();

                    let combined = prev_var + seen_expr.clone() - others_seen;

                    *bundle = std::mem::take(bundle)
                        .and_reified(
                            ExtraVarName::IsLastTeacherSeen {
                                subject,
                                student,
                                teacher,
                                week,
                            },
                            move || vec![combined.geq(&IntLinExpr::constant(1))],
                        )
                        .expect("no duplicate IsLastTeacherSeen");

                    let avoid_constraint = seen_expr.leq(
                        &(IntLinExpr::constant(1)
                            - IntLinExpr::<V>::var(extra_var(ExtraVarName::IsLastTeacherSeen {
                                subject,
                                student,
                                teacher,
                                week: prev_week,
                            }))),
                    );
                    let desc = PreferenceConstraint::BalancingAvoidTwiceInARowRecursive {
                        student,
                        subject,
                        teacher,
                        week,
                    }
                    .into();
                    if is_soft {
                        *soft_bundle =
                            std::mem::take(soft_bundle).with_constraint(avoid_constraint, desc);
                    } else {
                        *hard_bundle =
                            std::mem::take(hard_bundle).with_constraint(avoid_constraint, desc);
                    }
                }
            }
        }
    }
}

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut output = MyBundle::new();

    for (subject_id, subject) in &env.subjects.ordered_subject_list {
        let Some(params) = subject_interrogation_params(env, *subject_id) else {
            continue;
        };
        let Some(sp) =
            effective_balancing_option(env, *subject_id, |opts| &opts.avoid_twice_in_a_row)
        else {
            continue;
        };
        let is_soft = sp.soft;

        let slot_week_pairs =
            slot_week_pairs_for_subject(env, *subject_id, &subject.excluded_periods);

        let mut bundle = MyBundle::new();
        let mut hard_bundle = MyBundle::new();
        let mut soft_bundle = MyBundle::new();

        match &params.periodicity {
            SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks,
            } => {
                let p = periodicity_in_weeks.get() as usize;
                build_window_constraints(
                    env,
                    *subject_id,
                    &slot_week_pairs,
                    2 * p,
                    1,
                    is_soft,
                    &mut hard_bundle,
                    &mut soft_bundle,
                );
            }
            SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                weeks_per_block, ..
            } => {
                let b = weeks_per_block.get() as usize;
                build_window_constraints(
                    env,
                    *subject_id,
                    &slot_week_pairs,
                    2 * b,
                    b,
                    is_soft,
                    &mut hard_bundle,
                    &mut soft_bundle,
                );
            }
            SubjectPeriodicity::AmountInYear { .. }
            | SubjectPeriodicity::AmountForEveryArbitraryBlock { .. } => {
                if !is_at_most_once_per_week(env, *subject_id) {
                    hard_bundle = hard_bundle.with_infeasible(
                        InfeasibleConstraint::BalancingAvoidTwiceUnsupported {
                            subject: *subject_id,
                        }
                        .into(),
                    );
                    bundle = bundle
                        .merge(hard_bundle)
                        .expect("no duplicate extras from balancing avoid_twice hard");
                    output = output.merge(bundle).expect(
                        "no duplicate extras from balancing avoid_twice (distinct subjects)",
                    );
                    continue;
                }
                build_recursive_constraints(
                    env,
                    *subject_id,
                    &slot_week_pairs,
                    is_soft,
                    &mut bundle,
                    &mut hard_bundle,
                    &mut soft_bundle,
                );
            }
        }

        bundle = bundle
            .merge(hard_bundle)
            .expect("no duplicate extras from balancing avoid_twice hard");
        bundle = merge_objectified(
            bundle,
            soft_bundle,
            ExtraVarName::BalancingAvoidTwiceInARowPenalty {
                subject: *subject_id,
            },
        );
        output = output
            .merge(bundle)
            .expect("no duplicate extras from balancing avoid_twice (distinct subjects)");
    }

    output
}
