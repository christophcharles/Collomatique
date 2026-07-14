use crate::ids::{GlobalWeek, GroupNum};
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{Var, VarEnv};
use collomatique_ilp::int_linexpr::{IntConstraint, IntLinExpr};
use collomatique_ilp_modeler::bundle::ReifyError;
use collomatique_ilp_modeler::{IntConstraintBundle, Var as ModelerVar};
use collomatique_state_colloscopes::group_lists::GroupList;
use collomatique_state_colloscopes::ids::WeekPatternId;
use collomatique_state_colloscopes::ids::{GroupListId, PeriodId, StudentId, SubjectId};
use collomatique_state_colloscopes::slots::Slot;
use collomatique_time::SlotWithDuration;
use std::collections::BTreeSet;

pub(crate) type V = ModelerVar<Var, ExtraVarName>;
pub(crate) type MyBundle = IntConstraintBundle<
    'static,
    Var,
    ExtraVarName,
    ConstraintDesc,
    VarEnv,
    ReifyError<Var, ExtraVarName>,
>;

pub(crate) fn base_var(v: Var) -> V {
    ModelerVar::Base(v)
}

pub(crate) fn extra_var(v: ExtraVarName) -> V {
    ModelerVar::Extra(v)
}

// ---- Helper functions reading from Parameters ----

pub(crate) fn week_to_period_id(env: &VarEnv, week: GlobalWeek) -> Option<(PeriodId, usize)> {
    crate::tools::week_to_period_id(env, week.0)
}

pub(crate) fn group_list_for_interrogation(
    env: &VarEnv,
    subject: SubjectId,
    week: GlobalWeek,
) -> Option<GroupListId> {
    let (period_id, _) = week_to_period_id(env, week)?;
    env.group_lists
        .subjects_associations
        .get(&(period_id, subject))
        .copied()
}

pub(crate) fn groups_for_interrogation(
    env: &VarEnv,
    subject: SubjectId,
    week: GlobalWeek,
) -> Vec<GroupNum> {
    let Some(gl_id) = group_list_for_interrogation(env, subject, week) else {
        return vec![];
    };
    groups_for_group_list(env, gl_id)
}

pub(crate) fn groups_for_group_list(env: &VarEnv, group_list: GroupListId) -> Vec<GroupNum> {
    GroupNum::enumerate(env, group_list).collect()
}

pub(crate) fn is_student_enrolled(
    env: &VarEnv,
    student: StudentId,
    subject: SubjectId,
    week: GlobalWeek,
) -> bool {
    let Some((period_id, _)) = week_to_period_id(env, week) else {
        return false;
    };
    env.assignments
        .students(period_id, subject)
        .is_some_and(|students| students.contains(&student))
}

pub(crate) fn weeks_for_slot(
    env: &VarEnv,
    slot: &Slot,
    excluded_periods: &BTreeSet<PeriodId>,
) -> Vec<GlobalWeek> {
    crate::tools::enumerate_weeks_for_slot(env, slot, excluded_periods)
        .into_iter()
        .map(GlobalWeek)
        .collect()
}

pub(crate) fn students_for_group_list(env: &VarEnv, gl: &GroupList) -> Vec<StudentId> {
    match &gl.filling {
        collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
            excluded_students,
        } => env
            .students
            .student_map
            .keys()
            .filter(|s| !excluded_students.contains(s))
            .collect(),
        collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled { groups } => {
            groups
                .iter()
                .flat_map(|g| g.students.iter().copied())
                .collect()
        }
    }
}

pub(crate) fn students_for_subject_period_group_list(
    env: &VarEnv,
    gl: &GroupList,
    subject: SubjectId,
    period: PeriodId,
) -> Vec<StudentId> {
    let enrolled = env.assignments.students(period, subject);
    let Some(enrolled) = enrolled else {
        return vec![];
    };
    students_for_group_list(env, gl)
        .into_iter()
        .filter(|s| enrolled.contains(s))
        .collect()
}

pub(crate) fn subject_interrogation_params(
    env: &VarEnv,
    subject: SubjectId,
) -> Option<&collomatique_state_colloscopes::subjects::SubjectInterrogationParameters> {
    env.subjects
        .ordered_subject_list
        .iter()
        .find(|(id, _)| *id == subject)
        .and_then(|(_, s)| s.parameters.interrogation_parameters.as_ref())
}

pub(crate) fn active_slots_for_subject_week(
    env: &VarEnv,
    subject: SubjectId,
    week: GlobalWeek,
) -> Vec<crate::ids::SlotId> {
    let Some(subject_slots) = env.slots.slots_for_subject(subject) else {
        return vec![];
    };
    let Some(subj) = env.subjects.find_subject(subject) else {
        return vec![];
    };
    subject_slots
        .filter(|(_, slot_data)| {
            if subj
                .excluded_periods
                .iter()
                .any(|ep| week_to_period_id(env, week).is_some_and(|(pid, _)| pid == *ep))
            {
                return false;
            }
            let pattern = crate::tools::extract_week_pattern(env, slot_data.week_pattern);
            pattern.get(week.0).copied().unwrap_or(false)
        })
        .map(|(slot_id, _)| *slot_id)
        .collect()
}

pub(crate) fn student_has_interrogation_in_expr(
    env: &VarEnv,
    student: StudentId,
    subject: SubjectId,
    week: GlobalWeek,
) -> IntLinExpr<V> {
    let slots = active_slots_for_subject_week(env, subject, week);
    slots
        .into_iter()
        .map(|slot| {
            IntLinExpr::var(extra_var(ExtraVarName::StudentAtInterrogation {
                student,
                slot,
                week,
            }))
        })
        .sum()
}

pub(crate) fn is_at_most_once_per_week(env: &VarEnv, subject: SubjectId) -> bool {
    use collomatique_state_colloscopes::subjects::SubjectPeriodicity;
    let Some(params) = subject_interrogation_params(env, subject) else {
        return true;
    };
    match &params.periodicity {
        SubjectPeriodicity::ExactlyPeriodic { .. } => true,
        SubjectPeriodicity::OnceForEveryBlockOfWeeks { .. } => true,
        SubjectPeriodicity::AmountInYear {
            minimum_week_separation,
            ..
        } => *minimum_week_separation >= 1,
        SubjectPeriodicity::AmountForEveryArbitraryBlock {
            minimum_week_separation,
            ..
        } => *minimum_week_separation >= 1,
    }
}

pub(crate) fn weeks_for_week_pattern(
    env: &VarEnv,
    week_pattern_id: Option<WeekPatternId>,
    excluded_periods: &BTreeSet<PeriodId>,
) -> Vec<GlobalWeek> {
    let week_pattern = crate::tools::extract_week_pattern(env, week_pattern_id);
    let mut output = Vec::new();
    let mut global_week = 0usize;
    for (period_id, period_desc) in env.periods.ordered_period_list.iter() {
        for week_desc in period_desc {
            if week_desc.interrogations
                && *week_pattern.get(global_week).unwrap_or(&true)
                && !excluded_periods.contains(&period_id)
            {
                output.push(GlobalWeek(global_week));
            }
            global_week += 1;
        }
    }
    output
}

// ---- Reified variable builders (lazy registration) ----

fn build_interrogation_has_groups(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for subject_id in env.slots.subjects_with_slots() {
        let Some(subject) = env.subjects.find_subject(subject_id) else {
            continue;
        };
        for (slot_id, slot_data) in env
            .slots
            .slots_for_subject(subject_id)
            .into_iter()
            .flatten()
        {
            let slot = *slot_id;
            for week in weeks_for_slot(env, slot_data, &subject.excluded_periods) {
                let groups = groups_for_interrogation(env, subject_id, week);
                if groups.is_empty() {
                    continue;
                }
                let var = ExtraVarName::InterrogationHasGroups { slot, week };
                bundle = bundle
                    .and_reified(var, move || {
                        let sum: IntLinExpr<V> = groups
                            .iter()
                            .map(|&group| {
                                IntLinExpr::var(base_var(Var::GroupInInterrogation {
                                    slot,
                                    week,
                                    group,
                                }))
                            })
                            .sum();
                        vec![sum.geq(&IntLinExpr::constant(1))]
                    })
                    .expect("no duplicate extras");
            }
        }
    }
    bundle
}

fn build_student_in_group(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for (group_list, gl) in env.group_lists.group_list_map.iter() {
        let students = students_for_group_list(env, gl);
        for group in GroupNum::enumerate(env, group_list) {
            for &student in &students {
                let var = ExtraVarName::StudentInGroup {
                    student,
                    group_list,
                    group,
                };
                bundle = bundle
                    .and_reified(var, move || {
                        let expr = IntLinExpr::var(base_var(Var::StudentGroup {
                            group_list,
                            student,
                        }));
                        let group_i64: i64 = group
                            .index()
                            .try_into()
                            .expect("group index should fit in i64");
                        vec![expr.eq(&IntLinExpr::constant(group_i64))]
                    })
                    .expect("no duplicate extras");
            }
        }
    }
    bundle
}

fn build_group_has_students(env: &VarEnv) -> MyBundle {
    use collomatique_state_colloscopes::group_lists::GroupListFilling;

    let mut bundle = MyBundle::new();
    for (group_list, gl) in env.group_lists.group_list_map.iter() {
        for group in GroupNum::enumerate(env, group_list) {
            let var = ExtraVarName::GroupHasStudents { group_list, group };
            match &gl.filling {
                GroupListFilling::Prefilled { groups } => {
                    let has_students = groups
                        .get(group.index())
                        .is_some_and(|g| !g.students.is_empty());
                    if has_students {
                        bundle = bundle
                            .and_reified(var, move || {
                                vec![IntLinExpr::constant(0).leq(&IntLinExpr::constant(0))]
                            })
                            .expect("no duplicate extras");
                    } else {
                        bundle = bundle
                            .and_reified(var, move || vec![IntConstraint::infeasible()])
                            .expect("no duplicate extras");
                    }
                }
                GroupListFilling::Automatic { excluded_students } => {
                    let students: Vec<StudentId> = env
                        .students
                        .student_map
                        .keys()
                        .filter(|s| !excluded_students.contains(s))
                        .collect();
                    bundle = bundle
                        .and_reified(var, move || {
                            let sum: IntLinExpr<V> = students
                                .iter()
                                .map(|&student| {
                                    IntLinExpr::var(extra_var(ExtraVarName::StudentInGroup {
                                        student,
                                        group_list,
                                        group,
                                    }))
                                })
                                .sum();
                            vec![sum.geq(&IntLinExpr::constant(1))]
                        })
                        .expect("no duplicate extras");
                }
            }
        }
    }
    bundle
}

fn build_student_at_interrogation_in_group(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for subject_id in env.slots.subjects_with_slots() {
        let Some(subject) = env.subjects.find_subject(subject_id) else {
            continue;
        };
        for (slot_id, slot_data) in env
            .slots
            .slots_for_subject(subject_id)
            .into_iter()
            .flatten()
        {
            let slot = *slot_id;
            for week in weeks_for_slot(env, slot_data, &subject.excluded_periods) {
                let Some(group_list) = group_list_for_interrogation(env, subject_id, week) else {
                    continue;
                };
                let Some(gl) = env.group_lists.group_list_map.get(&group_list) else {
                    continue;
                };
                let students = students_for_group_list(env, gl);
                for &student in &students {
                    for group in GroupNum::enumerate(env, group_list) {
                        let var = ExtraVarName::StudentAtInterrogationInGroup {
                            student,
                            slot,
                            week,
                            group_list,
                            group,
                        };
                        bundle = bundle
                            .and_reified(var, move || {
                                let c1 = IntLinExpr::var(extra_var(ExtraVarName::StudentInGroup {
                                    student,
                                    group_list,
                                    group,
                                }))
                                .geq(&IntLinExpr::constant(1));
                                let c2 = IntLinExpr::var(base_var(Var::GroupInInterrogation {
                                    slot,
                                    week,
                                    group,
                                }))
                                .geq(&IntLinExpr::constant(1));
                                vec![c1, c2]
                            })
                            .expect("no duplicate extras");
                    }
                }
            }
        }
    }
    bundle
}

fn build_student_at_interrogation(env: &VarEnv) -> MyBundle {
    use collomatique_state_colloscopes::group_lists::GroupListFilling;

    let mut bundle = MyBundle::new();
    for subject_id in env.slots.subjects_with_slots() {
        let Some(subject) = env.subjects.find_subject(subject_id) else {
            continue;
        };
        for (slot_id, slot_data) in env
            .slots
            .slots_for_subject(subject_id)
            .into_iter()
            .flatten()
        {
            let slot = *slot_id;
            for week in weeks_for_slot(env, slot_data, &subject.excluded_periods) {
                for student in env.students.student_map.keys() {
                    let var = ExtraVarName::StudentAtInterrogation {
                        student,
                        slot,
                        week,
                    };

                    if !is_student_enrolled(env, student, subject_id, week) {
                        bundle = bundle
                            .and_reified(var, move || vec![IntConstraint::infeasible()])
                            .expect("no duplicate extras");
                        continue;
                    }

                    let Some(group_list) = group_list_for_interrogation(env, subject_id, week)
                    else {
                        bundle = bundle
                            .and_reified(var, move || vec![IntConstraint::infeasible()])
                            .expect("no duplicate extras");
                        continue;
                    };

                    let Some(gl) = env.group_lists.group_list_map.get(&group_list) else {
                        continue;
                    };

                    match &gl.filling {
                        GroupListFilling::Prefilled { groups } => {
                            let group = groups.iter().enumerate().find_map(|(i, g)| {
                                g.students.contains(&student).then(|| {
                                    GroupNum::new(env, group_list, i).expect("valid group index")
                                })
                            });
                            match group {
                                Some(group) => {
                                    bundle = bundle
                                        .and_reified(var, move || {
                                            let expr = IntLinExpr::var(base_var(
                                                Var::GroupInInterrogation { slot, week, group },
                                            ));
                                            vec![expr.geq(&IntLinExpr::constant(1))]
                                        })
                                        .expect("no duplicate extras");
                                }
                                None => {
                                    bundle = bundle
                                        .and_reified(var, move || vec![IntConstraint::infeasible()])
                                        .expect("no duplicate extras");
                                }
                            }
                        }
                        GroupListFilling::Automatic { .. } => {
                            let groups: Vec<GroupNum> =
                                GroupNum::enumerate(env, group_list).collect();
                            bundle = bundle
                                .and_reified(var, move || {
                                    let sum: IntLinExpr<V> = groups
                                        .iter()
                                        .map(|&group| {
                                            IntLinExpr::var(extra_var(
                                                ExtraVarName::StudentAtInterrogationInGroup {
                                                    student,
                                                    slot,
                                                    week,
                                                    group_list,
                                                    group,
                                                },
                                            ))
                                        })
                                        .sum();
                                    vec![sum.geq(&IntLinExpr::constant(1))]
                                })
                                .expect("no duplicate extras");
                        }
                    }
                }
            }
        }
    }
    bundle
}

fn build_student_has_interrogation_in(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for subject_id in env.slots.subjects_with_slots() {
        let Some(subject) = env.subjects.find_subject(subject_id) else {
            continue;
        };
        let active_slot_ids_per_week: Vec<(GlobalWeek, Vec<crate::ids::SlotId>)> = {
            let mut weeks_seen = std::collections::BTreeSet::new();
            let mut result = Vec::new();
            for (_, slot_data) in env
                .slots
                .slots_for_subject(subject_id)
                .into_iter()
                .flatten()
            {
                for week in weeks_for_slot(env, slot_data, &subject.excluded_periods) {
                    if weeks_seen.insert(week) {
                        let slots = active_slots_for_subject_week(env, subject_id, week);
                        if !slots.is_empty() {
                            result.push((week, slots));
                        }
                    }
                }
            }
            result
        };
        for (week, active_slot_ids) in active_slot_ids_per_week {
            for student in env.students.student_map.keys() {
                if !is_student_enrolled(env, student, subject_id, week) {
                    continue;
                }
                let var = ExtraVarName::StudentHasInterrogationIn {
                    student,
                    subject: subject_id,
                    week,
                };
                let slots = active_slot_ids.clone();
                bundle = bundle
                    .and_reified(var, move || {
                        let sum: IntLinExpr<V> = slots
                            .iter()
                            .map(|&slot| {
                                IntLinExpr::var(extra_var(ExtraVarName::StudentAtInterrogation {
                                    student,
                                    slot,
                                    week,
                                }))
                            })
                            .sum();
                        vec![sum.geq(&IntLinExpr::constant(1))]
                    })
                    .expect("no duplicate extras");
            }
        }
    }
    bundle
}

fn build_student_not_at_incompat_slot(env: &VarEnv) -> MyBundle {
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

        for (incompat_slot_index, incompat_swd) in incompat.slots.iter().enumerate() {
            for &week in &incompat_weeks {
                let (period_id, _) = match week_to_period_id(env, week) {
                    Some(p) => p,
                    None => continue,
                };

                let enrolled_in_incompat_subject =
                    env.assignments.students(period_id, incompat.subject_id);
                let Some(enrolled_students) = enrolled_in_incompat_subject else {
                    continue;
                };

                for &student in enrolled_students {
                    let overlapping: Vec<_> = all_interrog_slots
                        .iter()
                        .filter(|(_, subj_id, slot_data, excluded, swd)| {
                            swd.overlaps_with(incompat_swd)
                                && !excluded.contains(&period_id)
                                && is_student_enrolled(env, student, *subj_id, week)
                                && {
                                    let pattern = crate::tools::extract_week_pattern(
                                        env,
                                        slot_data.week_pattern,
                                    );
                                    pattern.get(week.0).copied().unwrap_or(false)
                                }
                        })
                        .map(|(slot_id, ..)| *slot_id)
                        .collect();

                    let var = ExtraVarName::StudentNotAtIncompatSlot {
                        student,
                        incompat: incompat_id,
                        incompat_slot_index,
                        week,
                    };
                    if overlapping.is_empty() {
                        bundle = bundle
                            .and_reified(var, move || vec![])
                            .expect("no duplicate extras");
                    } else {
                        bundle = bundle
                            .and_reified(var, move || {
                                overlapping
                                    .iter()
                                    .map(|&slot| {
                                        IntLinExpr::<V>::var(extra_var(
                                            ExtraVarName::StudentAtInterrogation {
                                                student,
                                                slot,
                                                week,
                                            },
                                        ))
                                        .leq(&IntLinExpr::constant(0))
                                    })
                                    .collect()
                            })
                            .expect("no duplicate extras");
                    }
                }
            }
        }
    }
    bundle
}

// ---- Public API ----

pub fn build_extras(env: &VarEnv) -> MyBundle {
    let bundle = build_interrogation_has_groups(env);
    let bundle = bundle
        .merge(build_student_in_group(env))
        .expect("no duplicate extras");
    let bundle = bundle
        .merge(build_group_has_students(env))
        .expect("no duplicate extras");
    let bundle = bundle
        .merge(build_student_at_interrogation_in_group(env))
        .expect("no duplicate extras");
    let bundle = bundle
        .merge(build_student_at_interrogation(env))
        .expect("no duplicate extras");
    let bundle = bundle
        .merge(build_student_not_at_incompat_slot(env))
        .expect("no duplicate extras");
    bundle
        .merge(build_student_has_interrogation_in(env))
        .expect("no duplicate extras")
}
