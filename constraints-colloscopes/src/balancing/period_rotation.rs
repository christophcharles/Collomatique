use crate::extras::{MyBundle, subject_interrogation_params};
use crate::helpers::{
    enrolled_students_for_subject, merge_objectified_weighted, slot_week_pairs_for_subject,
};
use crate::ids::GlobalWeek;
use crate::types::{ConstraintDesc, ExtraVarName, PreferenceConstraint};
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::ids::SubjectId;

use super::helpers::{
    count_student_teacher_expr, effective_balancing_option, slot_week_pairs_for_teacher,
    slot_weeks_in_range, subject_active_weeks, teachers_for_subject, year_interrogation_count,
};

fn period_interrogation_windows(
    env: &VarEnv,
    subject_id: SubjectId,
) -> Vec<(GlobalWeek, GlobalWeek, u32, u32)> {
    let Some(year_nb_interr) = year_interrogation_count(env, subject_id) else {
        return vec![];
    };
    if year_nb_interr == 0 {
        return vec![];
    }

    let Some(subject) = env.subjects.find_subject(subject_id) else {
        return vec![];
    };

    let mut period_active_weeks: Vec<(Vec<GlobalWeek>, u32)> = Vec::new();
    let mut global_week = 0usize;
    for (i, (period_id, period_desc)) in env.periods.ordered_period_list.iter().enumerate() {
        if subject.excluded_periods.contains(period_id) {
            global_week += period_desc.len();
            continue;
        }
        let mut weeks = Vec::new();
        for week_desc in period_desc {
            if week_desc.interrogations {
                weeks.push(GlobalWeek(global_week));
            }
            global_week += 1;
        }
        if !weeks.is_empty() {
            period_active_weeks.push((weeks, (i + 1) as u32));
        }
    }

    let total_active: u64 = period_active_weeks
        .iter()
        .map(|(w, _)| w.len() as u64)
        .sum();
    if total_active == 0 {
        return vec![];
    }

    period_active_weeks
        .iter()
        .map(|(weeks, period)| {
            let first_week = *weeks.first().unwrap();
            let last_week = *weeks.last().unwrap();
            let period_weeks = weeks.len() as u64;
            let nb_interr = std::cmp::max(
                1,
                ((year_nb_interr as u64) * period_weeks + total_active - 1) / total_active,
            ) as u32;
            (first_week, last_week, nb_interr, *period)
        })
        .collect()
}

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut output = MyBundle::new();

    for (subject_id, subject) in &env.subjects.ordered_subject_list {
        let Some(_params) = subject_interrogation_params(env, *subject_id) else {
            continue;
        };
        let Some(sp) =
            effective_balancing_option(env, *subject_id, |opts| &opts.period_teacher_rotation)
        else {
            continue;
        };
        let is_soft = sp.soft;

        let slot_week_pairs =
            slot_week_pairs_for_subject(env, *subject_id, &subject.excluded_periods);

        let windows = period_interrogation_windows(env, *subject_id);
        if windows.is_empty() {
            continue;
        }

        let enrolled = enrolled_students_for_subject(env, *subject_id);
        let teachers = teachers_for_subject(env, *subject_id);

        // Same typical spacing as whole-year teacher rotation (per-period windows
        // are just a slice of it): total active weeks × #teachers / year_nb_interr.
        let total_weeks = subject_active_weeks(&slot_week_pairs).len() as f64;
        let year_n = year_interrogation_count(env, *subject_id)
            .unwrap_or(1)
            .max(1) as f64;
        let t_typical = total_weeks * teachers.len().max(1) as f64 / year_n;

        let mut hard_bundle = MyBundle::new();
        let mut soft_bundle = MyBundle::new();

        for (first_week, last_week, nb_interr, period) in &windows {
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
                    let desc = PreferenceConstraint::BalancingPeriodRotation {
                        student,
                        subject: *subject_id,
                        teacher,
                        period: *period,
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

        output = output
            .merge(merge_objectified_weighted(
                hard_bundle,
                soft_bundle,
                ExtraVarName::BalancingPeriodRotationPenalty {
                    subject: *subject_id,
                },
                move |desc| match desc {
                    ConstraintDesc::Level4(PreferenceConstraint::BalancingPeriodRotation {
                        first_week,
                        last_week,
                        ..
                    }) => {
                        let ws = (last_week.0 - first_week.0 + 1) as f64;
                        crate::weights::window_weight(total_weeks, ws, t_typical)
                    }
                    _ => crate::weights::BASE,
                },
            ))
            .expect("no duplicate extras from balancing period rotation (distinct subjects)");
    }

    output
}
