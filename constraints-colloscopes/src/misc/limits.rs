use crate::extras::{MyBundle, V, extra_var};
use crate::ids::GlobalWeek;
use crate::types::{ExtraVarName, PreferenceConstraint};
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::ids::{PeriodId, SlotId, StudentId};
use collomatique_state_colloscopes::settings::SoftParam;
use collomatique_time::Weekday;
use std::num::NonZeroU32;

fn all_interrogation_weeks(env: &VarEnv) -> Vec<(GlobalWeek, PeriodId)> {
    let mut result = Vec::new();
    for (global_week, (period_id, _week_id, week_desc)) in env.walk_weeks().enumerate() {
        if week_desc.interrogations {
            result.push((GlobalWeek(global_week), period_id));
        }
    }
    result
}

fn effective_max_per_day(env: &VarEnv, student: StudentId) -> Option<&SoftParam<NonZeroU32>> {
    env.settings
        .limits_for(student)
        .max_interrogations_per_day
        .as_ref()
}

fn effective_max_per_week(env: &VarEnv, student: StudentId) -> Option<&SoftParam<u32>> {
    env.settings
        .limits_for(student)
        .interrogations_per_week_max
        .as_ref()
}

fn effective_min_per_week(env: &VarEnv, student: StudentId) -> Option<&SoftParam<u32>> {
    env.settings
        .limits_for(student)
        .interrogations_per_week_min
        .as_ref()
}

fn counted_slots_for_student_week(
    env: &VarEnv,
    student: StudentId,
    week: GlobalWeek,
    period: PeriodId,
) -> Vec<(SlotId, Weekday)> {
    let mut result = Vec::new();
    for subject_id in env.slots.subjects_with_slots() {
        let Some(subject) = env.subjects.find_subject(subject_id) else {
            continue;
        };
        let Some(params) = subject.parameters.interrogation_parameters.as_ref() else {
            continue;
        };
        if !params.take_duration_into_account {
            continue;
        }
        if subject.excluded_periods.contains(&period) {
            continue;
        }
        let enrolled = env
            .assignments
            .students(period, subject_id)
            .is_some_and(|students| students.contains(&student));
        if !enrolled {
            continue;
        }
        for (slot_id, slot_data) in env
            .slots
            .slots_for_subject(subject_id)
            .into_iter()
            .flatten()
        {
            let active = crate::tools::extract_week_pattern(env, slot_data.week_pattern);
            if !active.get(week.0).copied().unwrap_or(false) {
                continue;
            }
            result.push((*slot_id, slot_data.start_time.weekday));
        }
    }
    result
}

fn student_at_interrogation_sum(
    student: StudentId,
    week: GlobalWeek,
    slots: &[SlotId],
) -> IntLinExpr<V> {
    slots
        .iter()
        .map(|&slot| {
            IntLinExpr::var(extra_var(ExtraVarName::StudentAtInterrogation {
                student,
                slot,
                week,
            }))
        })
        .sum()
}

use crate::helpers::merge_objectified_weighted;

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut hard_max_per_day = MyBundle::new();
    let mut hard_max_per_week = MyBundle::new();
    let mut hard_min_per_week = MyBundle::new();
    let mut soft_output = MyBundle::new();

    let interrogation_weeks = all_interrogation_weeks(env);

    for (student, student_data) in env.students.student_map.iter() {
        let max_per_day = effective_max_per_day(env, student);
        let max_per_week = effective_max_per_week(env, student);
        let min_per_week = effective_min_per_week(env, student);

        if max_per_day.is_none() && max_per_week.is_none() && min_per_week.is_none() {
            continue;
        }

        for &(week, period) in &interrogation_weeks {
            if student_data.excluded_periods.contains(&period) {
                continue;
            }

            let counted_slots = counted_slots_for_student_week(env, student, week, period);
            if counted_slots.is_empty() {
                continue;
            }

            if let Some(sp) = max_per_day {
                let max = sp.value.get();
                for day in Weekday::iter() {
                    let day_slots: Vec<SlotId> = counted_slots
                        .iter()
                        .filter(|(_, d)| *d == day)
                        .map(|(s, _)| *s)
                        .collect();
                    if day_slots.is_empty() {
                        continue;
                    }
                    let sum = student_at_interrogation_sum(student, week, &day_slots);
                    let constraint = sum.leq(&IntLinExpr::constant(i64::from(max)));
                    let desc = PreferenceConstraint::MaxInterrogationsPerDay {
                        student,
                        week,
                        day,
                        max,
                    }
                    .into();
                    if sp.soft {
                        soft_output = merge_objectified_weighted(
                            soft_output,
                            MyBundle::new().with_constraint(constraint, desc),
                            ExtraVarName::LimitsMaxPerDayPenalty { student, week, day },
                            |_| crate::weights::BASE,
                        );
                    } else {
                        hard_max_per_day = hard_max_per_day.with_constraint(constraint, desc);
                    }
                }
            }

            if let Some(sp) = max_per_week {
                let max = sp.value;
                let all_slot_ids: Vec<SlotId> = counted_slots.iter().map(|(s, _)| *s).collect();
                let sum = student_at_interrogation_sum(student, week, &all_slot_ids);
                let constraint = sum.leq(&IntLinExpr::constant(i64::from(max)));
                let desc =
                    PreferenceConstraint::MaxInterrogationsPerWeek { student, week, max }.into();
                if sp.soft {
                    soft_output = merge_objectified_weighted(
                        soft_output,
                        MyBundle::new().with_constraint(constraint, desc),
                        ExtraVarName::LimitsMaxPerWeekPenalty { student, week },
                        |_| crate::weights::BASE,
                    );
                } else {
                    hard_max_per_week = hard_max_per_week.with_constraint(constraint, desc);
                }
            }

            if let Some(sp) = min_per_week {
                let min = sp.value;
                let all_slot_ids: Vec<SlotId> = counted_slots.iter().map(|(s, _)| *s).collect();
                let sum = student_at_interrogation_sum(student, week, &all_slot_ids);
                let constraint = sum.geq(&IntLinExpr::constant(i64::from(min)));
                let desc =
                    PreferenceConstraint::MinInterrogationsPerWeek { student, week, min }.into();
                if sp.soft {
                    soft_output = merge_objectified_weighted(
                        soft_output,
                        MyBundle::new().with_constraint(constraint, desc),
                        ExtraVarName::LimitsMinPerWeekPenalty { student, week },
                        |_| crate::weights::BASE,
                    );
                } else {
                    hard_min_per_week = hard_min_per_week.with_constraint(constraint, desc);
                }
            }
        }
    }

    let mut bundle = hard_max_per_day;
    bundle = bundle
        .merge(hard_max_per_week)
        .expect("no duplicate extras from limits");
    bundle = bundle
        .merge(hard_min_per_week)
        .expect("no duplicate extras from limits");

    bundle = bundle
        .merge(soft_output)
        .expect("no duplicate extras from limits soft penalties");

    bundle
}
