use crate::extras::{MyBundle, subject_interrogation_params};
use crate::helpers::{enrolled_students_for_subject, slot_week_pairs_for_subject};
use crate::ids::GlobalWeek;
use crate::types::PreferenceConstraint;
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::ids::SubjectId;

use super::helpers::{
    count_student_teacher_expr, effective_balancing_flag, slot_week_pairs_for_teacher,
    slot_weeks_in_range, teachers_for_subject, year_interrogation_count,
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
    for (i, period_id) in env.periods.period_ids().enumerate() {
        if subject.excluded_periods.contains(&period_id) {
            global_week += env
                .periods
                .week_count_of(period_id)
                .expect("period id from period_ids is valid");
            continue;
        }
        let mut weeks = Vec::new();
        for week_desc in env
            .periods
            .weeks_of(period_id)
            .expect("period id from period_ids is valid")
        {
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

    for (subject_id, subject) in env.subjects.ordered_subject_list.iter() {
        let subject_id = &subject_id;
        let Some(_params) = subject_interrogation_params(env, *subject_id) else {
            continue;
        };
        if !effective_balancing_flag(env, *subject_id, |opts| opts.period_teacher_rotation) {
            continue;
        }

        let slot_week_pairs =
            slot_week_pairs_for_subject(env, *subject_id, &subject.excluded_periods);

        let windows = period_interrogation_windows(env, *subject_id);
        if windows.is_empty() {
            continue;
        }

        let enrolled = enrolled_students_for_subject(env, *subject_id);
        let teachers = teachers_for_subject(env, *subject_id);

        let mut hard_bundle = MyBundle::new();

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
                    hard_bundle = hard_bundle.with_constraint(constraint, desc);
                }
            }
        }

        output = output
            .merge(hard_bundle)
            .expect("no duplicate extras from balancing period rotation (distinct subjects)");
    }

    output
}
