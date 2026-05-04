use crate::helpers::{
    enrolled_students_for_subject, last_global_week, merge_objectified, slot_week_pairs_for_subject,
};
use crate::ids::GlobalWeek;
use crate::native_extras::{MyBundle, subject_interrogation_params};
use crate::types::{ExtraVarName, PreferenceConstraint};
use collomatique_binding_colloscopes::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;

use super::helpers::{
    count_student_teacher_expr, effective_balancing_option, slot_week_pairs_for_teacher,
    slot_weeks_in_range, teachers_for_subject, year_interrogation_count,
};

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut hard_bundle = MyBundle::new();
    let mut soft_bundle = MyBundle::new();

    let first_week = GlobalWeek(0);
    let last_week = last_global_week(env);

    for (subject_id, subject) in &env.subjects.ordered_subject_list {
        let Some(_params) = subject_interrogation_params(env, *subject_id) else {
            continue;
        };
        let Some(sp) =
            effective_balancing_option(env, *subject_id, |opts| &opts.year_teacher_rotation)
        else {
            continue;
        };
        let is_soft = sp.soft;

        let Some(nb_interr) = year_interrogation_count(env, *subject_id) else {
            continue;
        };
        if nb_interr == 0 {
            continue;
        }

        let slot_week_pairs =
            slot_week_pairs_for_subject(env, *subject_id, &subject.excluded_periods);
        let ntot = slot_weeks_in_range(&slot_week_pairs, first_week, last_week);
        if ntot == 0 {
            continue;
        }

        let enrolled = enrolled_students_for_subject(env, *subject_id);
        let teachers = teachers_for_subject(env, *subject_id);

        for &teacher in &teachers {
            let teacher_pairs =
                slot_week_pairs_for_teacher(&slot_week_pairs, env, *subject_id, teacher);
            let nt = slot_weeks_in_range(&teacher_pairs, first_week, last_week);
            let max_count = ((nt as u64) * (nb_interr as u64) + (ntot as u64) - 1) / (ntot as u64);
            let max_count = max_count as u32;

            for &student in &enrolled {
                let count =
                    count_student_teacher_expr(&teacher_pairs, student, first_week, last_week);
                let constraint = count.leq(&IntLinExpr::constant(i64::from(max_count)));
                let desc = PreferenceConstraint::BalancingYearRotation {
                    student,
                    subject: *subject_id,
                    teacher,
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

    merge_objectified(
        hard_bundle,
        soft_bundle,
        ExtraVarName::BalancingYearRotationPenalty,
    )
}
