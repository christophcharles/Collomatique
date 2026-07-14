use crate::extras::{
    MyBundle, V, extra_var, is_student_enrolled, week_to_period_id, weeks_for_week_pattern,
};
use crate::ids::GlobalWeek;
use crate::types::{ExtraVarName, StructuralConstraint};
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::ids::{PeriodId, SlotId, StudentId, SubjectId};
use collomatique_time::SlotWithDuration;

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();

    let all_interrog_slots: Vec<_> = {
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
                        let overlapping = find_overlapping_slots(
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

fn find_overlapping_slots(
    env: &VarEnv,
    all_interrog_slots: &[(
        SlotId,
        SubjectId,
        &collomatique_state_colloscopes::slots::Slot,
        &std::collections::BTreeSet<PeriodId>,
        SlotWithDuration,
    )],
    incompat_swd: &SlotWithDuration,
    student: StudentId,
    week: GlobalWeek,
    period_id: PeriodId,
) -> Vec<SlotId> {
    all_interrog_slots
        .iter()
        .filter(|(_, subj_id, slot_data, excluded, swd)| {
            swd.overlaps_with(incompat_swd)
                && !excluded.contains(&period_id)
                && is_student_enrolled(env, student, *subj_id, week)
                && {
                    let pattern = crate::tools::extract_week_pattern(env, slot_data.week_pattern);
                    pattern.get(week.0).copied().unwrap_or(false)
                }
        })
        .map(|(slot_id, ..)| *slot_id)
        .collect()
}
