use crate::native_extras::{
    MyBundle, all_slots, extra_var, is_student_enrolled, slot_subject,
    subject_interrogation_params, weeks_for_slot,
};
use crate::types::{ConstraintDesc, ReifiedVarName};
use collomatique_binding_colloscopes::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_time::SlotWithDuration;
use std::collections::BTreeSet;

pub fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();

    let all_slot_ids = all_slots(env);

    let slots_with_duration: Vec<_> = all_slot_ids
        .iter()
        .filter_map(|&slot_id| {
            let slot = env.slots.find_slot(slot_id)?;
            let subject = slot_subject(env, slot_id)?;
            let params = subject_interrogation_params(env, subject)?;
            let swd = SlotWithDuration::new(slot.start_time.clone(), params.duration)?;
            Some((slot_id, swd))
        })
        .collect();

    for i in 0..slots_with_duration.len() {
        for j in (i + 1)..slots_with_duration.len() {
            let (slot_a, swd_a) = &slots_with_duration[i];
            let (slot_b, swd_b) = &slots_with_duration[j];

            if !swd_a.overlaps_with(swd_b) {
                continue;
            }

            let weeks_a: BTreeSet<_> = weeks_for_slot(env, *slot_a).into_iter().collect();
            let weeks_b: BTreeSet<_> = weeks_for_slot(env, *slot_b).into_iter().collect();

            for &week in weeks_a.intersection(&weeks_b) {
                for &student in env.students.student_map.keys() {
                    if !is_student_enrolled(env, student, *slot_a, week)
                        || !is_student_enrolled(env, student, *slot_b, week)
                    {
                        continue;
                    }

                    let expr_a =
                        IntLinExpr::var(extra_var(ReifiedVarName::StudentAtInterrogation {
                            student,
                            slot: *slot_a,
                            week,
                        }));
                    let expr_b =
                        IntLinExpr::var(extra_var(ReifiedVarName::StudentAtInterrogation {
                            student,
                            slot: *slot_b,
                            week,
                        }));
                    bundle = bundle.with_constraint(
                        (expr_a + expr_b).leq(&IntLinExpr::constant(1)),
                        ConstraintDesc::OneInterrogationAtOnce {
                            student,
                            slot_a: *slot_a,
                            slot_b: *slot_b,
                            week,
                        },
                    );
                }
            }
        }
    }

    bundle
}
