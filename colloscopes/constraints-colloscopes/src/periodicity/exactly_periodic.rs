use crate::extras::{MyBundle, subject_interrogation_params};
use crate::ids::GlobalWeek;
use crate::types::{InfeasibleConstraint, ProgressiveConstraint, QualityConstraint};
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::ids::{StudentId, SubjectId};
use collomatique_state_colloscopes::subjects::{Subject, SubjectPeriodicity};

use super::helpers::{
    count_interrogations_expr, enrolled_students_for_subject, slot_week_pairs_for_subject,
    span_has_slot_week,
};

struct PeriodRunInfo {
    first_global_week: GlobalWeek,
    last_global_week: GlobalWeek,
    active_weeks: Vec<GlobalWeek>,
}

fn compute_period_runs(
    env: &VarEnv,
    subject_id: SubjectId,
    student: StudentId,
    subject: &Subject,
) -> Vec<PeriodRunInfo> {
    let mut runs = Vec::new();
    let mut current_first: Option<GlobalWeek> = None;
    let mut current_last = GlobalWeek(0);
    let mut current_active_weeks = Vec::new();
    let mut global_week = 0usize;

    for period_id in env.periods.period_ids() {
        let period_len = env.weeks.week_count_for_period(period_id).unwrap_or(0);
        let period_id = &period_id;
        let first_of_period = GlobalWeek(global_week);
        let last_of_period = GlobalWeek(global_week + period_len.saturating_sub(1));

        let is_active = !subject.excluded_periods.contains(period_id)
            && env
                .assignments
                .students(*period_id, subject_id)
                .is_some_and(|students| students.contains(&student));

        if is_active {
            if current_first.is_none() {
                current_first = Some(first_of_period);
            }
            current_last = last_of_period;
            for (week_id, _week_desc) in
                env.weeks.weeks_for_period(*period_id).into_iter().flatten()
            {
                if env.is_week_active(*week_id, subject.week_pattern) {
                    current_active_weeks.push(GlobalWeek(global_week));
                }
                global_week += 1;
            }
        } else {
            if let Some(first) = current_first.take() {
                runs.push(PeriodRunInfo {
                    first_global_week: first,
                    last_global_week: current_last,
                    active_weeks: std::mem::take(&mut current_active_weeks),
                });
            }
            global_week += period_len;
        }
    }
    if let Some(first) = current_first {
        runs.push(PeriodRunInfo {
            first_global_week: first,
            last_global_week: current_last,
            active_weeks: current_active_weeks,
        });
    }

    runs
}

pub(super) fn build(env: &VarEnv, mut bundle: MyBundle) -> MyBundle {
    for (subject_id, subject) in env.subjects.ordered_subject_list.iter() {
        let subject_id = &subject_id;
        let Some(params) = subject_interrogation_params(env, *subject_id) else {
            continue;
        };
        let SubjectPeriodicity::ExactlyPeriodic {
            periodicity_in_weeks,
        } = &params.periodicity
        else {
            continue;
        };

        let periodicity = periodicity_in_weeks.get() as usize;
        let slot_week_pairs = slot_week_pairs_for_subject(env, *subject_id, subject);
        let enrolled = enrolled_students_for_subject(env, *subject_id);

        for &student in &enrolled {
            let runs = compute_period_runs(env, *subject_id, student, subject);

            for run in &runs {
                if run.active_weeks.len() < periodicity {
                    bundle = bundle.with_infeasible(
                        InfeasibleConstraint::PeriodicityExactlyPeriodicInfeasible {
                            student,
                            subject: *subject_id,
                            first_week: run.first_global_week,
                            last_week: run.last_global_week,
                            periodicity: periodicity_in_weeks.get(),
                        }
                        .into(),
                    );
                } else {
                    for window in run.active_weeks.windows(periodicity) {
                        let win_first = window[0];
                        let win_last = window[window.len() - 1];
                        if !span_has_slot_week(&slot_week_pairs, win_first, win_last) {
                            bundle = bundle.with_infeasible(
                                InfeasibleConstraint::NoSlotsForWeekSpan {
                                    student,
                                    subject: *subject_id,
                                    first_week: win_first,
                                    last_week: win_last,
                                    required_count: 1,
                                }
                                .into(),
                            );
                            continue;
                        }
                        let count_expr = count_interrogations_expr(
                            &slot_week_pairs,
                            student,
                            win_first,
                            win_last,
                        );
                        bundle = bundle.with_constraint(
                            count_expr.eq(&IntLinExpr::constant(1)),
                            ProgressiveConstraint::PeriodicityInterrogationCountExact {
                                student,
                                subject: *subject_id,
                                first_week: win_first,
                                last_week: win_last,
                                count: 1,
                            }
                            .into(),
                        );
                        bundle = bundle.with_constraint(
                            count_expr.leq(&IntLinExpr::constant(1)),
                            QualityConstraint::PeriodicityInterrogationCountMax {
                                student,
                                subject: *subject_id,
                                first_week: win_first,
                                last_week: win_last,
                                max_count: 1,
                            }
                            .into(),
                        );
                    }
                }
            }
        }
    }
    bundle
}
