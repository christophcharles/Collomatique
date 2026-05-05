use crate::ids::GlobalWeek;
use crate::native_extras::{MyBundle, subject_interrogation_params};
use crate::types::{ProgressiveConstraint, QualityConstraint};
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::subjects::SubjectPeriodicity;

use super::helpers::{
    all_active_global_weeks, count_interrogations_expr, enrolled_students_for_subject,
    last_global_week, slot_week_pairs_for_subject,
};

pub(super) fn build(env: &VarEnv, mut bundle: MyBundle) -> MyBundle {
    let last_week = last_global_week(env);
    let first_week = GlobalWeek(0);
    let all_active_weeks = all_active_global_weeks(env);

    for (subject_id, subject) in &env.subjects.ordered_subject_list {
        let Some(params) = subject_interrogation_params(env, *subject_id) else {
            continue;
        };
        let SubjectPeriodicity::AmountInYear {
            interrogation_count_in_year,
            minimum_week_separation,
        } = &params.periodicity
        else {
            continue;
        };

        let count_min = *interrogation_count_in_year.start();
        let count_max = *interrogation_count_in_year.end();
        let min_sep = *minimum_week_separation as usize;

        let slot_week_pairs =
            slot_week_pairs_for_subject(env, *subject_id, &subject.excluded_periods);
        let enrolled = enrolled_students_for_subject(env, *subject_id);

        for &student in &enrolled {
            let count_expr =
                count_interrogations_expr(&slot_week_pairs, student, first_week, last_week);

            if count_min == count_max {
                bundle = bundle.with_constraint(
                    count_expr.eq(&IntLinExpr::constant(i64::from(count_min))),
                    ProgressiveConstraint::PeriodicityInterrogationCountExact {
                        student,
                        subject: *subject_id,
                        first_week,
                        last_week,
                        count: count_min,
                    }
                    .into(),
                );
                bundle = bundle.with_constraint(
                    count_expr.leq(&IntLinExpr::constant(i64::from(count_max))),
                    QualityConstraint::PeriodicityInterrogationCountMax {
                        student,
                        subject: *subject_id,
                        first_week,
                        last_week,
                        max_count: count_max,
                    }
                    .into(),
                );
            } else {
                if count_min > 0 {
                    bundle = bundle.with_constraint(
                        count_expr.geq(&IntLinExpr::constant(i64::from(count_min))),
                        ProgressiveConstraint::PeriodicityInterrogationCountMin {
                            student,
                            subject: *subject_id,
                            first_week,
                            last_week,
                            min_count: count_min,
                        }
                        .into(),
                    );
                }
                bundle = bundle.with_constraint(
                    count_expr.leq(&IntLinExpr::constant(i64::from(count_max))),
                    QualityConstraint::PeriodicityInterrogationCountMax {
                        student,
                        subject: *subject_id,
                        first_week,
                        last_week,
                        max_count: count_max,
                    }
                    .into(),
                );
            }

            if min_sep > 0 {
                for window in all_active_weeks.windows(min_sep) {
                    let win_first = window[0];
                    let win_last = window[window.len() - 1];
                    let sep_expr =
                        count_interrogations_expr(&slot_week_pairs, student, win_first, win_last);
                    bundle = bundle.with_constraint(
                        sep_expr.leq(&IntLinExpr::constant(1)),
                        QualityConstraint::PeriodicitySeparation {
                            student,
                            subject: *subject_id,
                            first_week: win_first,
                            last_week: win_last,
                        }
                        .into(),
                    );
                }
            }
        }
    }
    bundle
}
