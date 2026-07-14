use crate::extras::{MyBundle, subject_interrogation_params};
use crate::helpers::{
    count_interrogations_expr, enrolled_students_for_subject, last_global_week,
    merge_objectified_weighted, slot_week_pairs_for_subject,
};
use crate::ids::GlobalWeek;
use crate::types::{ExtraVarName, PreferenceConstraint};
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::ids::SlotId;

use super::helpers::{
    count_student_teacher_expr, effective_balancing_option, slot_weeks_in_range,
    subject_active_weeks,
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

    for (subject_id, subject) in env.subjects.ordered_subject_list.iter() {
        let subject_id = &subject_id;
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

        let enrolled = enrolled_students_for_subject(env, *subject_id);
        let Some(subject_slots) = env.slots.subject_map.get(subject_id) else {
            continue;
        };

        if is_soft {
            // Soft path: cumulative availability-proportional balance, per
            // (student, slot). For each prefix boundary week `wᵢ`, the share of the
            // student's interrogations that go to this slot should match the slot's
            // share of the *available* slot-weeks up to `wᵢ`: `Uₛ/U == Aₛ/A`, cleared
            // to `A·Uₛ − Aₛ·U == 0`. Only weeks `0..=i-1` appear, so — objectified on
            // its own into a penalty keyed by `{subject, student, slot, wᵢ}` — the
            // footprint ends at `wᵢ` and enters the incremental objective when that
            // week's epoch completes. See rotation.rs for the full rationale.
            let n = active_weeks.len();
            if n >= 2 {
                for (slot_id, _slot_data) in &subject_slots.ordered_slots {
                    let slot_pairs = slot_week_pairs_for_slot(&slot_week_pairs, *slot_id);
                    for &student in &enrolled {
                        for i in 1..=n {
                            let week = active_weeks[i - 1];
                            let a =
                                slot_weeks_in_range(&slot_week_pairs, active_weeks[0], week) as i64;
                            let a_s =
                                slot_weeks_in_range(&slot_pairs, active_weeks[0], week) as i64;
                            if a == 0 || a_s == 0 {
                                // `a_s == 0` ⇒ `Uₛ ≡ 0` ⇒ constraint is trivially `0 == 0`.
                                continue;
                            }
                            let u = count_interrogations_expr(
                                &slot_week_pairs,
                                student,
                                active_weeks[0],
                                week,
                            );
                            let u_s = count_student_teacher_expr(
                                &slot_pairs,
                                student,
                                active_weeks[0],
                                week,
                            );
                            let lhs = a * &u_s - a_s * &u;
                            let constraint = lhs.eq(&IntLinExpr::constant(0));
                            let desc = PreferenceConstraint::BalancingSlotRotationRegularity {
                                student,
                                subject: *subject_id,
                                slot: *slot_id,
                                week,
                            }
                            .into();
                            let weight = crate::weights::BASE / (n as f64 * a as f64);
                            output = merge_objectified_weighted(
                                output,
                                MyBundle::new().with_constraint(constraint, desc),
                                ExtraVarName::BalancingSlotRotationPenalty {
                                    subject: *subject_id,
                                    student,
                                    slot: *slot_id,
                                    week,
                                },
                                move |_desc| weight,
                            );
                        }
                    }
                }
            }
        } else {
            // Hard path: periodicity-based density upper-bound windows (unchanged).
            let mut hard_bundle = MyBundle::new();
            let windows = generate_windows(&active_weeks, last_week, &params.periodicity);
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
                        let count = count_student_teacher_expr(
                            &slot_pairs,
                            student,
                            *first_week,
                            *last_week,
                        );
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
                        hard_bundle = hard_bundle.with_constraint(constraint, desc);
                    }
                }
            }
            output = output
                .merge(hard_bundle)
                .expect("no duplicate extras from balancing slot rotation (distinct subjects)");
        }
    }

    output
}
