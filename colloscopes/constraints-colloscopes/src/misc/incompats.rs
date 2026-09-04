use crate::extras::{
    MyBundle, V, all_interrog_slots, extra_var, overlapping_interrog_slots, week_to_period_id,
    weeks_for_week_pattern,
};
use crate::types::{ExtraVarName, StructuralConstraint};
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();

    let all_interrog_slots = all_interrog_slots(env);

    for (incompat_id, incompat) in env.incompats.incompat_map.iter() {
        let Some(subject) = env.subjects.find_subject(incompat.subject_id) else {
            continue;
        };

        let incompat_weeks =
            weeks_for_week_pattern(env, incompat.week_pattern_id, &subject.excluded_periods);
        let num_incompat_slots = incompat.slots.len();
        let minimum_free_slots = incompat.minimum_free_slots.get() as usize;

        for &week in &incompat_weeks {
            let Some((period_id, _)) = week_to_period_id(env, week) else {
                continue;
            };

            let enrolled_in_subject = env.assignments.students(period_id, incompat.subject_id);
            let Some(enrolled_students) = enrolled_in_subject else {
                continue;
            };

            for &student in enrolled_students {
                if num_incompat_slots == minimum_free_slots {
                    for incompat_swd in &incompat.slots {
                        let overlapping = overlapping_interrog_slots(
                            env,
                            &all_interrog_slots,
                            incompat_swd,
                            student,
                            week,
                            period_id,
                        );
                        for slot in overlapping {
                            let expr = IntLinExpr::<V>::var(extra_var(
                                ExtraVarName::StudentAtInterrogation {
                                    student,
                                    slot,
                                    week,
                                },
                            ));
                            bundle = bundle.with_constraint(
                                expr.leq(&IntLinExpr::constant(0)),
                                StructuralConstraint::IncompatSaturated {
                                    student,
                                    incompat: incompat_id,
                                    subject: incompat.subject_id,
                                    week,
                                }
                                .into(),
                            );
                        }
                    }
                } else {
                    let sum: IntLinExpr<V> = (0..num_incompat_slots)
                        .map(|idx| {
                            IntLinExpr::var(extra_var(ExtraVarName::StudentNotAtIncompatSlot {
                                student,
                                incompat: incompat_id,
                                incompat_slot_index: idx,
                                week,
                            }))
                        })
                        .sum();
                    bundle = bundle.with_constraint(
                        sum.geq(&IntLinExpr::constant(minimum_free_slots as i64)),
                        StructuralConstraint::IncompatNonSaturated {
                            student,
                            incompat: incompat_id,
                            subject: incompat.subject_id,
                            week,
                            minimum_free_slots: incompat.minimum_free_slots.get(),
                        }
                        .into(),
                    );
                }
            }
        }
    }
    bundle
}
