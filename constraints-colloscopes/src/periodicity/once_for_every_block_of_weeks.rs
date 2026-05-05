use crate::ids::GlobalWeek;
use crate::native_extras::{MyBundle, subject_interrogation_params};
use crate::types::{InfeasibleConstraint, ProgressiveConstraint, QualityConstraint};
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::subjects::SubjectPeriodicity;

use super::helpers::{
    all_active_global_weeks, count_interrogations_expr, enrolled_students_for_subject,
    slot_week_pairs_for_subject,
};

pub(super) fn build(env: &VarEnv, mut bundle: MyBundle) -> MyBundle {
    let all_active_weeks = all_active_global_weeks(env);

    for (subject_id, subject) in &env.subjects.ordered_subject_list {
        let Some(params) = subject_interrogation_params(env, *subject_id) else {
            continue;
        };
        let SubjectPeriodicity::OnceForEveryBlockOfWeeks {
            weeks_per_block,
            minimum_week_separation,
        } = &params.periodicity
        else {
            continue;
        };

        let wpb = weeks_per_block.get() as usize;
        let min_sep = minimum_week_separation.get() as usize;

        let slot_week_pairs =
            slot_week_pairs_for_subject(env, *subject_id, &subject.excluded_periods);
        let enrolled = enrolled_students_for_subject(env, *subject_id);

        // Per-period block constraints
        let mut global_week_offset = 0usize;
        for (period_id, period_desc) in &env.periods.ordered_period_list {
            let first_global_week = GlobalWeek(global_week_offset);
            let last_global_week =
                GlobalWeek(global_week_offset + period_desc.len().saturating_sub(1));

            let active_weeks_in_period: Vec<GlobalWeek> = period_desc
                .iter()
                .enumerate()
                .filter(|(_, wd)| wd.interrogations)
                .map(|(i, _)| GlobalWeek(global_week_offset + i))
                .collect();

            global_week_offset += period_desc.len();

            if subject.excluded_periods.contains(period_id) {
                continue;
            }

            let period_students = env
                .assignments
                .period_map
                .get(period_id)
                .and_then(|pa| pa.subject_map.get(subject_id));
            let Some(period_students) = period_students else {
                continue;
            };

            let active_count = active_weeks_in_period.len();

            for &student in period_students {
                if active_count == 0 {
                    // Nothing to constrain
                } else if active_count % wpb != 0 {
                    bundle = bundle.with_infeasible(
                        InfeasibleConstraint::PeriodicityOncePerBlockInfeasible {
                            student,
                            subject: *subject_id,
                            first_week: first_global_week,
                            last_week: last_global_week,
                            weeks_per_block: weeks_per_block.get(),
                        }
                        .into(),
                    );
                } else {
                    for chunk in active_weeks_in_period.chunks(wpb) {
                        let block_first = chunk[0];
                        let block_last = chunk[chunk.len() - 1];
                        let count_expr = count_interrogations_expr(
                            &slot_week_pairs,
                            student,
                            block_first,
                            block_last,
                        );
                        bundle = bundle.with_constraint(
                            count_expr.eq(&IntLinExpr::constant(1)),
                            ProgressiveConstraint::PeriodicityInterrogationCountExact {
                                student,
                                subject: *subject_id,
                                first_week: block_first,
                                last_week: block_last,
                                count: 1,
                            }
                            .into(),
                        );
                        bundle = bundle.with_constraint(
                            count_expr.leq(&IntLinExpr::constant(1)),
                            QualityConstraint::PeriodicityInterrogationCountMax {
                                student,
                                subject: *subject_id,
                                first_week: block_first,
                                last_week: block_last,
                                max_count: 1,
                            }
                            .into(),
                        );
                    }
                }
            }
        }

        // Separation constraints (across whole year)
        for &student in &enrolled {
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
    bundle
}
