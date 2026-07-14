use crate::extras::{MyBundle, extra_var, is_student_enrolled, weeks_for_slot};
use crate::types::{ExtraVarName, StructuralConstraint};
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_time::SlotWithDuration;
use std::collections::BTreeSet;

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();

    let slots_with_duration: Vec<_> = {
        let mut result = Vec::new();
        for subject_id in env.slots.subjects_with_slots() {
            let Some(subject) = env.subjects.find_subject(subject_id) else {
                continue;
            };
            let Some(params) = subject.parameters.interrogation_parameters.as_ref() else {
                continue;
            };
            for (slot_id, slot_data) in env
                .slots
                .slots_for_subject(subject_id)
                .into_iter()
                .flatten()
            {
                let Some(swd) =
                    SlotWithDuration::new(slot_data.start_time.clone(), params.duration)
                else {
                    continue;
                };
                result.push((
                    *slot_id,
                    subject_id,
                    slot_data,
                    &subject.excluded_periods,
                    swd,
                ));
            }
        }
        result
    };

    for i in 0..slots_with_duration.len() {
        for j in (i + 1)..slots_with_duration.len() {
            let (slot_a, subject_a, slot_data_a, excluded_a, swd_a) = &slots_with_duration[i];
            let (slot_b, subject_b, slot_data_b, excluded_b, swd_b) = &slots_with_duration[j];

            if !swd_a.overlaps_with(swd_b) {
                continue;
            }

            let weeks_a: BTreeSet<_> = weeks_for_slot(env, slot_data_a, excluded_a)
                .into_iter()
                .collect();
            let weeks_b: BTreeSet<_> = weeks_for_slot(env, slot_data_b, excluded_b)
                .into_iter()
                .collect();

            for &week in weeks_a.intersection(&weeks_b) {
                for student in env.students.student_map.keys() {
                    if !is_student_enrolled(env, student, *subject_a, week)
                        || !is_student_enrolled(env, student, *subject_b, week)
                    {
                        continue;
                    }

                    let expr_a = IntLinExpr::var(extra_var(ExtraVarName::StudentAtInterrogation {
                        student,
                        slot: *slot_a,
                        week,
                    }));
                    let expr_b = IntLinExpr::var(extra_var(ExtraVarName::StudentAtInterrogation {
                        student,
                        slot: *slot_b,
                        week,
                    }));
                    bundle = bundle.with_constraint(
                        (expr_a + expr_b).leq(&IntLinExpr::constant(1)),
                        StructuralConstraint::OneInterrogationAtOnce {
                            student,
                            slot_a: *slot_a,
                            slot_b: *slot_b,
                            week,
                        }
                        .into(),
                    );
                }
            }
        }
    }

    bundle
}
