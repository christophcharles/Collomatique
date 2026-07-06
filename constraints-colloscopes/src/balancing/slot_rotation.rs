use crate::extras::{MyBundle, subject_interrogation_params};
use crate::helpers::{
    enrolled_students_for_subject, last_global_week, merge_objectified_weighted,
    slot_week_pairs_for_subject,
};
use crate::ids::GlobalWeek;
use crate::types::{ConstraintDesc, ExtraVarName, PreferenceConstraint};
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::ids::SlotId;

use super::helpers::{
    count_student_teacher_expr, effective_balancing_option, slot_weeks_in_range,
    subject_active_weeks, year_interrogation_count,
};
use super::rotation::generate_windows;

fn slot_week_pairs_for_slot(
    all_pairs: &[(SlotId, GlobalWeek)],
    slot_id: SlotId,
) -> Vec<(SlotId, GlobalWeek)> {
    all_pairs
        .iter()
        .filter(|(s, _)| *s == slot_id)
        .copied()
        .collect()
}

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut output = MyBundle::new();

    let last_week = last_global_week(env);

    for (subject_id, subject) in &env.subjects.ordered_subject_list {
        let Some(params) = subject_interrogation_params(env, *subject_id) else {
            continue;
        };
        let Some(sp) = effective_balancing_option(env, *subject_id, |opts| &opts.slot_rotation)
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
        let Some(subject_slots) = env.slots.subject_map.get(subject_id) else {
            continue;
        };

        // Typical same-slot spacing: over `total_weeks` a student uses each of
        // `#slots` slots about `year_nb_interr / #slots` times.
        let total_weeks = active_weeks.len() as f64;
        let year_n = year_interrogation_count(env, *subject_id)
            .unwrap_or(1)
            .max(1) as f64;
        let t_typical = total_weeks * subject_slots.ordered_slots.len().max(1) as f64 / year_n;

        let mut hard_bundle = MyBundle::new();
        let mut soft_bundle = MyBundle::new();

        for (first_week, last_week, nb_interr) in &windows {
            let ntot = slot_weeks_in_range(&slot_week_pairs, *first_week, *last_week);
            if ntot == 0 {
                continue;
            }

            for (slot_id, _slot_data) in &subject_slots.ordered_slots {
                let slot_pairs = slot_week_pairs_for_slot(&slot_week_pairs, *slot_id);
                let ns = slot_weeks_in_range(&slot_pairs, *first_week, *last_week);
                let max_count =
                    ((ns as u64) * (*nb_interr as u64) + (ntot as u64) - 1) / (ntot as u64);
                let max_count = max_count as u32;

                for &student in &enrolled {
                    let count =
                        count_student_teacher_expr(&slot_pairs, student, *first_week, *last_week);
                    let constraint = count.leq(&IntLinExpr::constant(i64::from(max_count)));
                    let desc = PreferenceConstraint::BalancingSlotRotation {
                        student,
                        subject: *subject_id,
                        slot: *slot_id,
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
                ExtraVarName::BalancingSlotRotationPenalty {
                    subject: *subject_id,
                },
                move |desc| match desc {
                    ConstraintDesc::Level4(PreferenceConstraint::BalancingSlotRotation {
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
            .expect("no duplicate extras from balancing slot rotation (distinct subjects)");
    }

    output
}
