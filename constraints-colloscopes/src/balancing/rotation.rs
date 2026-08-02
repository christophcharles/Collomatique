use crate::extras::{MyBundle, subject_interrogation_params};
use crate::helpers::{
    count_interrogations_expr, enrolled_students_for_subject, last_global_week,
    merge_objectified_weighted, slot_week_pairs_for_subject,
};
use crate::ids::GlobalWeek;
use crate::types::{ExtraVarName, PreferenceConstraint};
use crate::vars::VarEnv;
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
    let mut output = MyBundle::new();

    let last_week = last_global_week(env);

    for (subject_id, subject) in env.subjects.ordered_subject_list.iter() {
        let subject_id = &subject_id;
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

        let enrolled = enrolled_students_for_subject(env, *subject_id);
        let teachers = teachers_for_subject(env, *subject_id);

        if is_soft {
            // Soft path: cumulative availability-proportional balance. For each
            // (student, teacher) and each prefix boundary week `wᵢ`, the share of
            // the student's interrogations that go to teacher `t` should match
            // teacher `t`'s share of the *available* slot-weeks up to `wᵢ`:
            // `Uₜ/U == Aₜ/A`, cleared of the fraction to `A·Uₜ − Aₜ·U == 0` (with
            // `A`, `Aₜ` availability constants and `U`, `Uₜ` prefix counts). Only
            // weeks `0..=i-1` appear, so — objectified on its own into a penalty
            // keyed by `{subject, student, teacher, wᵢ}` — the penalty's footprint
            // ends at `wᵢ` and enters the incremental objective at the epoch that
            // completes that week (see strategies incremental filter). This also
            // fixes a latent bug: the previous whole-year ramp only linearized the
            // visits actually seen, never pushing toward the teachers on offer.
            let n = active_weeks.len();
            if n >= 2 {
                for &teacher in &teachers {
                    let teacher_pairs =
                        slot_week_pairs_for_teacher(&slot_week_pairs, env, *subject_id, teacher);
                    for &student in &enrolled {
                        for i in 1..=n {
                            let week = active_weeks[i - 1];
                            let a =
                                slot_weeks_in_range(&slot_week_pairs, active_weeks[0], week) as i64;
                            let a_t =
                                slot_weeks_in_range(&teacher_pairs, active_weeks[0], week) as i64;
                            if a == 0 || a_t == 0 {
                                // `a_t == 0` ⇒ `Uₜ ≡ 0` ⇒ constraint is trivially `0 == 0`.
                                continue;
                            }
                            let u = count_interrogations_expr(
                                &slot_week_pairs,
                                student,
                                active_weeks[0],
                                week,
                            );
                            let u_t = count_student_teacher_expr(
                                &teacher_pairs,
                                student,
                                active_weeks[0],
                                week,
                            );
                            let lhs = a * &u_t - a_t * &u;
                            let constraint = lhs.eq(&IntLinExpr::constant(0));
                            let desc = PreferenceConstraint::BalancingRotationRegularity {
                                student,
                                subject: *subject_id,
                                teacher,
                                week,
                            }
                            .into();
                            // `BASE/(n·A)` cancels the `A` forced into the numerator
                            // for integer coefficients, so the penalty reads as
                            // `BASE/n·|Uₜ − (Aₜ/A)·U|` (misplaced-interrogation count),
                            // year-length independent like the other soft families.
                            let weight = crate::weights::BASE / (n as f64 * a as f64);
                            output = merge_objectified_weighted(
                                output,
                                MyBundle::new().with_constraint(constraint, desc),
                                ExtraVarName::BalancingRotationPenalty {
                                    subject: *subject_id,
                                    student,
                                    teacher,
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
                        hard_bundle = hard_bundle.with_constraint(constraint, desc);
                    }
                }
            }
            output = output
                .merge(hard_bundle)
                .expect("no duplicate extras from balancing rotation (distinct subjects)");
        }
    }

    output
}
