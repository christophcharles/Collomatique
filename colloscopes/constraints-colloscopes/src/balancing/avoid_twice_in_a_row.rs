use crate::extras::{
    MyBundle, V, extra_var, is_at_most_once_per_week, subject_interrogation_params,
};
use crate::helpers::{
    enrolled_students_for_subject, merge_objectified_weighted, slot_week_pairs_for_subject,
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

/// How [`build_window_constraints`] emits each `count <= 1` row: as a hard
/// constraint, or objectified into the permanent soft objective with the given
/// per-window weight.
#[derive(Clone, Copy)]
pub(super) enum WindowMode {
    Hard,
    Soft { weight: f64 },
}

/// Same distinction for the avoid rows of the `IsLastTeacherSeen` chain in
/// [`build_recursive_constraints`]. The chain reification itself always stays
/// hard — it *defines* the variables.
#[derive(Clone, Copy)]
pub(super) enum RecursiveMode {
    Hard,
    Soft,
}

pub(super) fn build_window_constraints(
    env: &VarEnv,
    subject_id: SubjectId,
    slot_week_pairs: &[(crate::ids::SlotId, GlobalWeek)],
    windows: &[(GlobalWeek, GlobalWeek)],
    mode: WindowMode,
    bundle: &mut MyBundle,
) {
    let enrolled = enrolled_students_for_subject(env, subject_id);
    let teachers = teachers_for_subject(env, subject_id);

    for &(first_week, last_week) in windows {
        for &student in &enrolled {
            for &teacher in &teachers {
                let teacher_pairs =
                    slot_week_pairs_for_teacher(slot_week_pairs, env, subject_id, teacher);
                let count =
                    count_student_teacher_expr(&teacher_pairs, student, first_week, last_week);
                let constraint = count.leq(&IntLinExpr::constant(1));
                match mode {
                    WindowMode::Hard => {
                        let desc = PreferenceConstraint::BalancingAvoidTwiceInARow {
                            student,
                            subject: subject_id,
                            teacher,
                            first_week,
                            last_week,
                        }
                        .into();
                        *bundle = std::mem::take(bundle).with_constraint(constraint, desc);
                    }
                    WindowMode::Soft { weight } => {
                        let desc = PreferenceConstraint::BalancingAvoidTwiceInARowSoft {
                            student,
                            subject: subject_id,
                            teacher,
                            first_week,
                            last_week,
                        }
                        .into();
                        *bundle = merge_objectified_weighted(
                            std::mem::take(bundle),
                            MyBundle::new().with_constraint(constraint, desc),
                            ExtraVarName::AvoidTwiceInARowPenalty {
                                subject: subject_id,
                                student,
                                teacher,
                                first_week,
                                last_week,
                            },
                            move |_desc| weight,
                        );
                    }
                }
            }
        }
    }
}

pub(super) fn build_recursive_constraints(
    env: &VarEnv,
    subject_id: SubjectId,
    slot_week_pairs: &[(crate::ids::SlotId, GlobalWeek)],
    mode: RecursiveMode,
    bundle: &mut MyBundle,
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
                    match mode {
                        RecursiveMode::Hard => {
                            let desc = PreferenceConstraint::BalancingAvoidTwiceInARowRecursive {
                                student,
                                subject,
                                teacher,
                                week,
                            }
                            .into();
                            *bundle =
                                std::mem::take(bundle).with_constraint(avoid_constraint, desc);
                        }
                        RecursiveMode::Soft => {
                            let desc =
                                PreferenceConstraint::BalancingAvoidTwiceInARowRecursiveSoft {
                                    student,
                                    subject,
                                    teacher,
                                    week,
                                }
                                .into();
                            *bundle = merge_objectified_weighted(
                                std::mem::take(bundle),
                                MyBundle::new().with_constraint(avoid_constraint, desc),
                                ExtraVarName::AvoidTwiceInARowPenalty {
                                    subject,
                                    student,
                                    teacher,
                                    first_week: prev_week,
                                    last_week: week,
                                },
                                |_desc| crate::weights::BASE,
                            );
                        }
                    }
                }
            }
        }
    }
}

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut output = MyBundle::new();

    for (subject_id, subject) in env.subjects.ordered_subject_list.iter() {
        let subject_id = &subject_id;
        let Some(params) = subject_interrogation_params(env, *subject_id) else {
            continue;
        };
        // Hard path only: the goal must be on *and* strict. `None` (off) and
        // `Some { soft: true }` (soft objective, built by
        // [`super::avoid_twice_in_a_row_soft`]) both skip the subject.
        let strict = matches!(
            effective_balancing_option(env, *subject_id, |opts| &opts.avoid_twice_in_a_row),
            Some(param) if !param.soft
        );
        if !strict {
            continue;
        }

        let slot_week_pairs = slot_week_pairs_for_subject(env, *subject_id, subject);

        let mut bundle = MyBundle::new();

        match &params.periodicity {
            SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks,
            } => {
                let p = periodicity_in_weeks.get() as usize;
                let active_weeks = subject_active_weeks(&slot_week_pairs);
                let windows = rolling_windows(&active_weeks, 2 * p, 1);
                build_window_constraints(
                    env,
                    *subject_id,
                    &slot_week_pairs,
                    &windows,
                    WindowMode::Hard,
                    &mut bundle,
                );
            }
            SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                weeks_per_block, ..
            } => {
                let b = weeks_per_block.get() as usize;
                let active_weeks = subject_active_weeks(&slot_week_pairs);
                let windows = rolling_windows(&active_weeks, 2 * b, b);
                build_window_constraints(
                    env,
                    *subject_id,
                    &slot_week_pairs,
                    &windows,
                    WindowMode::Hard,
                    &mut bundle,
                );
            }
            SubjectPeriodicity::AmountInYear { .. }
            | SubjectPeriodicity::AmountForEveryArbitraryBlock { .. } => {
                if !is_at_most_once_per_week(env, *subject_id) {
                    bundle = bundle.with_infeasible(
                        InfeasibleConstraint::BalancingAvoidTwiceUnsupported {
                            subject: *subject_id,
                        }
                        .into(),
                    );
                    output = output.merge(bundle).expect(
                        "no duplicate extras from balancing avoid_twice (distinct subjects)",
                    );
                    continue;
                }
                build_recursive_constraints(
                    env,
                    *subject_id,
                    &slot_week_pairs,
                    RecursiveMode::Hard,
                    &mut bundle,
                );
            }
        }

        output = output
            .merge(bundle)
            .expect("no duplicate extras from balancing avoid_twice (distinct subjects)");
    }

    output
}
