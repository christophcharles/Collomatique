use crate::helpers::{
    enrolled_students_for_subject, last_global_week, merge_objectified, slot_week_pairs_for_subject,
};
use crate::ids::GlobalWeek;
use crate::native_extras::{MyBundle, subject_interrogation_params};
use crate::types::{ExtraVarName, PreferenceConstraint};
use collomatique_binding_colloscopes::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::subjects::SubjectPeriodicity;

use super::helpers::{
    count_student_teacher_expr, effective_balancing_option, rolling_windows,
    slot_week_pairs_for_teacher, slot_weeks_in_range, subject_active_weeks, teachers_for_subject,
};

pub(super) fn generate_windows(
    active_weeks: &[GlobalWeek],
    last_week: GlobalWeek,
    periodicity: &SubjectPeriodicity,
) -> Vec<(GlobalWeek, GlobalWeek, u32)> {
    let total = active_weeks.len();
    match periodicity {
        SubjectPeriodicity::AmountInYear {
            interrogation_count_in_year,
            ..
        } => {
            let nb_interr = *interrogation_count_in_year.end();
            if nb_interr == 0 {
                return vec![];
            }
            vec![(GlobalWeek(0), last_week, nb_interr)]
        }
        SubjectPeriodicity::AmountForEveryArbitraryBlock { blocks, .. } => {
            let nb_interr: u32 = blocks
                .iter()
                .map(|b| *b.interrogation_count_in_block.end())
                .sum();
            if nb_interr == 0 {
                return vec![];
            }
            vec![(GlobalWeek(0), last_week, nb_interr)]
        }
        SubjectPeriodicity::ExactlyPeriodic {
            periodicity_in_weeks,
        } => {
            let p = periodicity_in_weeks.get();
            let mut windows = Vec::new();
            for ws in 2..=total {
                let nb_interr = (ws as u32 + p - 1) / p;
                for (fw, lw) in rolling_windows(active_weeks, ws, 1) {
                    windows.push((fw, lw, nb_interr));
                }
            }
            windows
        }
        SubjectPeriodicity::OnceForEveryBlockOfWeeks {
            weeks_per_block, ..
        } => {
            let b = weeks_per_block.get() as usize;
            let mut windows = Vec::new();
            for k in 2..=(total / b) {
                let nb_interr = k as u32;
                for (fw, lw) in rolling_windows(active_weeks, k * b, b) {
                    windows.push((fw, lw, nb_interr));
                }
            }
            windows
        }
    }
}

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut hard_bundle = MyBundle::new();
    let mut soft_bundle = MyBundle::new();

    let last_week = last_global_week(env);

    for (subject_id, subject) in &env.subjects.ordered_subject_list {
        let Some(params) = subject_interrogation_params(env, *subject_id) else {
            continue;
        };
        let Some(sp) = effective_balancing_option(env, *subject_id, |opts| &opts.teacher_rotation)
        else {
            continue;
        };
        let is_soft = sp.soft;

        let slot_week_pairs =
            slot_week_pairs_for_subject(env, *subject_id, &subject.excluded_periods);
        let active_weeks = subject_active_weeks(&slot_week_pairs);
        if active_weeks.is_empty() {
            continue;
        }

        let windows = generate_windows(&active_weeks, last_week, &params.periodicity);
        if windows.is_empty() {
            continue;
        }

        let enrolled = enrolled_students_for_subject(env, *subject_id);
        let teachers = teachers_for_subject(env, *subject_id);

        for (first_week, last_week, nb_interr) in &windows {
            let ntot = slot_weeks_in_range(&slot_week_pairs, *first_week, *last_week);
            if ntot == 0 {
                continue;
            }

            for &teacher in &teachers {
                let teacher_pairs =
                    slot_week_pairs_for_teacher(&slot_week_pairs, env, *subject_id, teacher);
                let nt = slot_weeks_in_range(&teacher_pairs, *first_week, *last_week);
                let max_count =
                    ((nt as u64) * (*nb_interr as u64) + (ntot as u64) - 1) / (ntot as u64);
                let max_count = max_count as u32;

                for &student in &enrolled {
                    let count = count_student_teacher_expr(
                        &teacher_pairs,
                        student,
                        *first_week,
                        *last_week,
                    );
                    let constraint = count.leq(&IntLinExpr::constant(i64::from(max_count)));
                    let desc = PreferenceConstraint::BalancingRotation {
                        student,
                        subject: *subject_id,
                        teacher,
                        first_week: *first_week,
                        last_week: *last_week,
                        max_count,
                    }
                    .into();
                    if is_soft {
                        soft_bundle = soft_bundle.with_constraint(constraint, desc);
                    } else {
                        hard_bundle = hard_bundle.with_constraint(constraint, desc);
                    }
                }
            }
        }
    }

    merge_objectified(
        hard_bundle,
        soft_bundle,
        ExtraVarName::BalancingRotationPenalty,
    )
}
