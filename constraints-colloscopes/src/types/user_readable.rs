use super::*;

impl InfeasibleConstraint {
    pub fn user_readable(
        &self,
        env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    ) -> String {
        match self {
            InfeasibleConstraint::PeriodicityExactlyPeriodicInfeasible {
                student,
                subject,
                first_week,
                last_week,
                periodicity,
            } => {
                let s_name = student_name(env, *student);
                let subj_name = subject_name(env, *subject);
                format!(
                    "Pas assez de semaines actives pour une périodicité exacte de {} semaine(s) en {} pour {} ({})",
                    periodicity,
                    subj_name,
                    s_name,
                    week_range_text(*first_week, *last_week),
                )
            }
            InfeasibleConstraint::PeriodicityOncePerBlockInfeasible {
                student,
                subject,
                first_week,
                last_week,
                weeks_per_block,
            } => {
                let s_name = student_name(env, *student);
                let subj_name = subject_name(env, *subject);
                format!(
                    "Le nombre de semaines actives n'est pas un multiple de {} pour {} en {} ({})",
                    weeks_per_block,
                    s_name,
                    subj_name,
                    week_range_text(*first_week, *last_week),
                )
            }
            InfeasibleConstraint::BalancingAvoidTwiceUnsupported { subject } => {
                let subj_name = subject_name(env, *subject);
                format!(
                    "L'option \"pas deux fois de suite\" n'est pas supportée pour la matière {} (séparation minimale < 1 semaine)",
                    subj_name,
                )
            }
        }
    }
}

impl StructuralConstraint {
    pub fn user_readable(
        &self,
        env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    ) -> String {
        match self {
            StructuralConstraint::OneInterrogationAtOnce {
                student,
                slot_a,
                slot_b,
                week,
            } => {
                let s_name = student_name(env, *student);
                let sa_name = slot_name(env, *slot_a);
                let sb_name = slot_name(env, *slot_b);
                format!(
                    "L'élève {} ne peut avoir les colles des créneaux {} et {} en même temps la semaine {}",
                    s_name,
                    sa_name,
                    sb_name,
                    week.0 + 1,
                )
            }
            StructuralConstraint::IncompatSaturated {
                student,
                incompat,
                subject,
                week,
            } => {
                let s_name = student_name(env, *student);
                let i_name = incompat_name(env, *incompat);
                let subj_name = subject_name(env, *subject);
                format!(
                    "L'élève {} ne doit pas avoir de colle la semaine {} pendant l'incompatibilité {} (matière {})",
                    s_name,
                    week.0 + 1,
                    i_name,
                    subj_name,
                )
            }
            StructuralConstraint::IncompatNonSaturated {
                student,
                incompat,
                subject,
                week,
                minimum_free_slots,
            } => {
                let s_name = student_name(env, *student);
                let i_name = incompat_name(env, *incompat);
                let subj_name = subject_name(env, *subject);
                format!(
                    "Au moins {} créneau(x) disponible(s) pour l'élève {} la semaine {} (incompatibilité {}, matière {})",
                    minimum_free_slots,
                    s_name,
                    week.0 + 1,
                    i_name,
                    subj_name,
                )
            }
            StructuralConstraint::StudentHasGroup {
                student,
                group_list,
            } => {
                let s_name = student_name(env, *student);
                let gl_name = group_list_name(env, *group_list);
                format!(
                    "L'élève {} doit avoir un groupe dans la liste {}",
                    s_name, gl_name
                )
            }
            StructuralConstraint::ForbiddenGroup {
                group_list,
                group,
                subject,
                ..
            } => {
                let gl_name = group_list_name(env, *group_list);
                let g_name = group_name(env, *group_list, *group);
                let subj_name = subject_name(env, *subject);
                format!(
                    "Le groupe {} de la liste {} ne peut avoir de colle dans la matière {} sans élève associé",
                    g_name, gl_name, subj_name,
                )
            }
            StructuralConstraint::SlotPairingUsedImpliesNotUsed { rule, week } => {
                let (subj, ant, con) = slot_pairing_info(env, *rule);
                format!(
                    "{} semaine {} : si le créneau ({}) est utilisé, le créneau ({}) ne doit pas être utilisé",
                    subj,
                    week.0 + 1,
                    ant,
                    con,
                )
            }
            StructuralConstraint::SlotPairingNotUsedImpliesNotUsed { rule, week } => {
                let (subj, ant, con) = slot_pairing_info(env, *rule);
                format!(
                    "{} semaine {} : si le créneau ({}) n'est pas utilisé, le créneau ({}) ne doit pas être utilisé non plus",
                    subj,
                    week.0 + 1,
                    ant,
                    con,
                )
            }
        }
    }
}

impl QualityConstraint {
    pub fn user_readable(
        &self,
        env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    ) -> String {
        match self {
            QualityConstraint::StudentsPerGroupMax {
                group_list,
                group,
                max_students,
            } => {
                let gl_name = group_list_name(env, *group_list);
                let g_name = group_name(env, *group_list, *group);
                format!(
                    "Au plus {} élèves dans le groupe {} de la liste {}",
                    max_students, g_name, gl_name
                )
            }
            QualityConstraint::StudentsPerGroupForSubjectMax {
                group_list,
                group,
                subject,
                period,
                max_students,
            } => {
                let gl_name = group_list_name(env, *group_list);
                let g_name = group_name(env, *group_list, *group);
                let subj_name = subject_name(env, *subject);
                let period_num = period_position(env, *period);
                format!(
                    "Au plus {} élèves dans le groupe {} de la liste {} pour la matière {} sur la période {}",
                    max_students, g_name, gl_name, subj_name, period_num
                )
            }
            QualityConstraint::GroupCountPerInterrogationMax {
                slot,
                week,
                max_groups,
            } => {
                let s_name = slot_name(env, *slot);
                format!(
                    "Maximum de {} groupe(s) pour la colle du créneau {} de la semaine {}",
                    max_groups,
                    s_name,
                    week.0 + 1,
                )
            }
            QualityConstraint::PeriodicityInterrogationCountMax {
                student,
                subject,
                first_week,
                last_week,
                max_count,
            } => {
                let s_name = student_name(env, *student);
                let subj_name = subject_name(env, *subject);
                let plural = if *max_count > 1 { "s" } else { "" };
                format!(
                    "{} doit avoir au plus {} colle{} en {} pour {}",
                    s_name,
                    max_count,
                    plural,
                    subj_name,
                    week_range_text(*first_week, *last_week),
                )
            }
            QualityConstraint::PeriodicitySeparation {
                student,
                subject,
                first_week,
                last_week,
            } => {
                let s_name = student_name(env, *student);
                let subj_name = subject_name(env, *subject);
                format!(
                    "{} ne doit pas avoir plus d'une colle en {} pour {}",
                    s_name,
                    subj_name,
                    week_range_text(*first_week, *last_week),
                )
            }
        }
    }
}

impl ProgressiveConstraint {
    pub fn user_readable(
        &self,
        env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    ) -> String {
        match self {
            ProgressiveConstraint::StudentsPerGroupMin {
                group_list,
                group,
                min_students,
            } => {
                let gl_name = group_list_name(env, *group_list);
                let g_name = group_name(env, *group_list, *group);
                format!(
                    "Au moins {} élèves dans le groupe {} de la liste {}",
                    min_students, g_name, gl_name
                )
            }
            ProgressiveConstraint::StudentsPerGroupForSubjectMin {
                group_list,
                group,
                subject,
                period,
                min_students,
            } => {
                let gl_name = group_list_name(env, *group_list);
                let g_name = group_name(env, *group_list, *group);
                let subj_name = subject_name(env, *subject);
                let period_num = period_position(env, *period);
                format!(
                    "Au moins {} élèves dans le groupe {} de la liste {} pour la matière {} sur la période {}",
                    min_students, g_name, gl_name, subj_name, period_num
                )
            }
            ProgressiveConstraint::GroupCountPerInterrogationMin {
                slot,
                week,
                min_groups,
            } => {
                let s_name = slot_name(env, *slot);
                format!(
                    "Minimum de {} groupe(s) pour la colle du créneau {} de la semaine {}",
                    min_groups,
                    s_name,
                    week.0 + 1,
                )
            }
            ProgressiveConstraint::PeriodicityInterrogationCountMin {
                student,
                subject,
                first_week,
                last_week,
                min_count,
            } => {
                let s_name = student_name(env, *student);
                let subj_name = subject_name(env, *subject);
                let plural = if *min_count > 1 { "s" } else { "" };
                format!(
                    "{} doit avoir au moins {} colle{} en {} pour {}",
                    s_name,
                    min_count,
                    plural,
                    subj_name,
                    week_range_text(*first_week, *last_week),
                )
            }
            ProgressiveConstraint::PeriodicityInterrogationCountExact {
                student,
                subject,
                first_week,
                last_week,
                count,
            } => {
                let s_name = student_name(env, *student);
                let subj_name = subject_name(env, *subject);
                if *count == 0 {
                    format!(
                        "{} ne doit pas avoir de colle en {} pour {}",
                        s_name,
                        subj_name,
                        week_range_text(*first_week, *last_week),
                    )
                } else {
                    let plural = if *count > 1 { "s" } else { "" };
                    format!(
                        "{} doit avoir exactement {} colle{} en {} pour {}",
                        s_name,
                        count,
                        plural,
                        subj_name,
                        week_range_text(*first_week, *last_week),
                    )
                }
            }
            ProgressiveConstraint::SlotPairingUsedImpliesUsed { rule, week } => {
                let (subj, ant, con) = slot_pairing_info(env, *rule);
                format!(
                    "{} semaine {} : si le créneau ({}) est utilisé, le créneau ({}) doit aussi être utilisé",
                    subj,
                    week.0 + 1,
                    ant,
                    con,
                )
            }
            ProgressiveConstraint::SlotPairingNotUsedImpliesUsed { rule, week } => {
                let (subj, ant, con) = slot_pairing_info(env, *rule);
                format!(
                    "{} semaine {} : si le créneau ({}) n'est pas utilisé, le créneau ({}) doit être utilisé",
                    subj,
                    week.0 + 1,
                    ant,
                    con,
                )
            }
        }
    }
}

impl PreferenceConstraint {
    pub fn user_readable(
        &self,
        env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    ) -> String {
        match self {
            PreferenceConstraint::GroupFilledByAscendingOrder { group_list, group } => {
                let gl_name = group_list_name(env, *group_list);
                let g_name = group_name(env, *group_list, *group);
                let next_g_name =
                    group_name(env, *group_list, group.next().expect("not the last group"));
                format!(
                    "Le groupe {} de la liste {} doit être rempli avant le groupe {}",
                    g_name, gl_name, next_g_name,
                )
            }
            PreferenceConstraint::PairingHavingImpliesHaving {
                student,
                week,
                rule,
            } => {
                let s_name = student_name(env, *student);
                let (ant_subj, con_subj) = pairing_subject_names(env, *rule);
                format!(
                    "Si l'élève {} a une colle en {} semaine {}, il doit aussi en avoir en {}",
                    s_name,
                    ant_subj,
                    week.0 + 1,
                    con_subj,
                )
            }
            PreferenceConstraint::PairingHavingImpliesNotHaving {
                student,
                week,
                rule,
            } => {
                let s_name = student_name(env, *student);
                let (ant_subj, con_subj) = pairing_subject_names(env, *rule);
                format!(
                    "Si l'élève {} a une colle en {} semaine {}, il ne doit pas en avoir en {}",
                    s_name,
                    ant_subj,
                    week.0 + 1,
                    con_subj,
                )
            }
            PreferenceConstraint::PairingNotHavingImpliesHaving {
                student,
                week,
                rule,
            } => {
                let s_name = student_name(env, *student);
                let (ant_subj, con_subj) = pairing_subject_names(env, *rule);
                format!(
                    "Si l'élève {} n'a pas de colle en {} semaine {}, il doit en avoir en {}",
                    s_name,
                    ant_subj,
                    week.0 + 1,
                    con_subj,
                )
            }
            PreferenceConstraint::PairingNotHavingImpliesNotHaving {
                student,
                week,
                rule,
            } => {
                let s_name = student_name(env, *student);
                let (ant_subj, con_subj) = pairing_subject_names(env, *rule);
                format!(
                    "Si l'élève {} n'a pas de colle en {} semaine {}, il ne doit pas en avoir en {}",
                    s_name,
                    ant_subj,
                    week.0 + 1,
                    con_subj,
                )
            }
            PreferenceConstraint::MaxInterrogationsPerDay {
                student,
                week,
                day,
                max,
            } => {
                let s_name = student_name(env, *student);
                format!(
                    "Au maximum {} colle(s) le {} de la semaine {} pour l'élève {}",
                    max,
                    day,
                    week.0 + 1,
                    s_name,
                )
            }
            PreferenceConstraint::MaxInterrogationsPerWeek { student, week, max } => {
                let s_name = student_name(env, *student);
                format!(
                    "Au maximum {} colle(s) la semaine {} pour l'élève {}",
                    max,
                    week.0 + 1,
                    s_name,
                )
            }
            PreferenceConstraint::MinInterrogationsPerWeek { student, week, min } => {
                let s_name = student_name(env, *student);
                format!(
                    "Au minimum {} colle(s) la semaine {} pour l'élève {}",
                    min,
                    week.0 + 1,
                    s_name,
                )
            }
            PreferenceConstraint::BalancingAvoidTwiceInARow {
                student,
                subject,
                teacher,
                first_week,
                last_week,
            } => {
                let s_name = student_name(env, *student);
                let subj_name = subject_name(env, *subject);
                let t_name = teacher_name(env, *teacher);
                format!(
                    "{} ne doit pas être collé(e) deux fois de suite par {} en {} ({})",
                    s_name,
                    t_name,
                    subj_name,
                    week_range_text(*first_week, *last_week),
                )
            }
            PreferenceConstraint::BalancingAvoidTwiceInARowRecursive {
                student,
                subject,
                teacher,
                week,
            } => {
                let s_name = student_name(env, *student);
                let subj_name = subject_name(env, *subject);
                let t_name = teacher_name(env, *teacher);
                format!(
                    "{} ne doit pas être collé(e) deux fois de suite par {} en {} (semaine {})",
                    s_name,
                    t_name,
                    subj_name,
                    week.0 + 1,
                )
            }
            PreferenceConstraint::BalancingYearRotation {
                student,
                subject,
                teacher,
                max_count,
            } => {
                let s_name = student_name(env, *student);
                let subj_name = subject_name(env, *subject);
                let t_name = teacher_name(env, *teacher);
                let plural = if *max_count > 1 { "s" } else { "" };
                format!(
                    "{} ne doit pas avoir plus de {} colle{} avec {} en {} sur l'année",
                    s_name, max_count, plural, t_name, subj_name,
                )
            }
            PreferenceConstraint::BalancingRotation {
                student,
                subject,
                teacher,
                first_week,
                last_week,
                max_count,
            } => {
                let s_name = student_name(env, *student);
                let subj_name = subject_name(env, *subject);
                let t_name = teacher_name(env, *teacher);
                let plural = if *max_count > 1 { "s" } else { "" };
                format!(
                    "{} ne doit pas avoir plus de {} colle{} avec {} en {} ({})",
                    s_name,
                    max_count,
                    plural,
                    t_name,
                    subj_name,
                    week_range_text(*first_week, *last_week),
                )
            }
            PreferenceConstraint::BalancingRotationRegularity {
                student,
                subject,
                teacher,
                week,
            } => {
                let s_name = student_name(env, *student);
                let subj_name = subject_name(env, *subject);
                let t_name = teacher_name(env, *teacher);
                format!(
                    "Les colles de {} avec {} en {} devraient être réparties régulièrement (semaine {})",
                    s_name,
                    t_name,
                    subj_name,
                    week.0 + 1,
                )
            }
            PreferenceConstraint::BalancingSlotRotation {
                student,
                subject: _,
                slot,
                first_week,
                last_week,
                max_count,
            } => {
                let s_name = student_name(env, *student);
                let sl_name = slot_name(env, *slot);
                let plural = if *max_count > 1 { "s" } else { "" };
                format!(
                    "{} ne doit pas avoir plus de {} colle{} dans le créneau {} ({})",
                    s_name,
                    max_count,
                    plural,
                    sl_name,
                    week_range_text(*first_week, *last_week),
                )
            }
            PreferenceConstraint::BalancingSlotRotationRegularity {
                student,
                subject: _,
                slot,
                week,
            } => {
                let s_name = student_name(env, *student);
                let sl_name = slot_name(env, *slot);
                format!(
                    "Les colles de {} dans le créneau {} devraient être réparties régulièrement (semaine {})",
                    s_name,
                    sl_name,
                    week.0 + 1,
                )
            }
            PreferenceConstraint::BalancingPeriodRotation {
                student,
                subject,
                teacher,
                period,
                first_week,
                last_week,
                max_count,
            } => {
                let s_name = student_name(env, *student);
                let subj_name = subject_name(env, *subject);
                let t_name = teacher_name(env, *teacher);
                let plural = if *max_count > 1 { "s" } else { "" };
                format!(
                    "{} ne doit pas avoir plus de {} colle{} avec {} en {} sur la période {} ({})",
                    s_name,
                    max_count,
                    plural,
                    t_name,
                    subj_name,
                    period,
                    week_range_text(*first_week, *last_week),
                )
            }
        }
    }
}

impl ConstraintDesc {
    pub fn user_readable(
        &self,
        env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    ) -> String {
        match self {
            ConstraintDesc::Level0(c) => c.user_readable(env),
            ConstraintDesc::Level1(c) => c.user_readable(env),
            ConstraintDesc::Level2(c) => c.user_readable(env),
            ConstraintDesc::Level3(c) => c.user_readable(env),
            ConstraintDesc::Level4(c) => c.user_readable(env),
        }
    }
}

fn week_range_text(first_week: GlobalWeek, last_week: GlobalWeek) -> String {
    if first_week == last_week {
        format!("la semaine {}", first_week.0 + 1)
    } else {
        format!("les semaines {} à {}", first_week.0 + 1, last_week.0 + 1,)
    }
}

fn slot_pairing_info(
    env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    rule: SlotPairingRuleId,
) -> (String, String, String) {
    let rule_data = env.slot_pairings.slot_pairing_rule_map.get(&rule);
    match rule_data {
        Some(r) => {
            let subj = env
                .slots
                .find_slot_subject_and_position(r.antecedent.slot_id)
                .and_then(|(subj_id, _)| {
                    env.subjects
                        .ordered_subject_list
                        .iter()
                        .find(|(id, _)| *id == subj_id)
                        .map(|(_, s)| s.parameters.name.clone())
                })
                .unwrap_or_else(|| format!("{:?}", rule));
            (
                subj,
                slot_teacher_and_time(env, r.antecedent.slot_id),
                slot_teacher_and_time(env, r.consequent.slot_id),
            )
        }
        None => (
            format!("{:?}", rule),
            format!("{:?}", rule),
            format!("{:?}", rule),
        ),
    }
}

fn slot_teacher_and_time(
    env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    slot: SlotId,
) -> String {
    match env.slots.find_slot(slot) {
        Some(data) => format!(
            "{}, {}",
            data.start_time,
            teacher_name(env, data.teacher_id)
        ),
        None => format!("{:?}", slot),
    }
}

fn pairing_subject_names(
    env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    rule: PairingRuleId,
) -> (String, String) {
    let rule_data = env.pairings.pairing_rule_map.get(&rule);
    match rule_data {
        Some(r) => (
            subject_name(env, r.antecedent.subject_id),
            subject_name(env, r.consequent.subject_id),
        ),
        None => (format!("{:?}", rule), format!("{:?}", rule)),
    }
}

fn incompat_name(
    env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    incompat: IncompatId,
) -> String {
    env.incompats
        .incompat_map
        .get(&incompat)
        .map(|i| i.name.clone())
        .unwrap_or_else(|| format!("{:?}", incompat))
}

fn student_name(
    env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    student: StudentId,
) -> String {
    env.students
        .student_map
        .get(&student)
        .map(|s| format!("{} {}", s.desc.firstname, s.desc.surname))
        .unwrap_or_else(|| format!("{:?}", student))
}

fn subject_name(
    env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    subject: SubjectId,
) -> String {
    env.subjects
        .ordered_subject_list
        .iter()
        .find(|(id, _)| *id == subject)
        .map(|(_, s)| s.parameters.name.clone())
        .unwrap_or_else(|| format!("{:?}", subject))
}

fn period_position(
    env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    period: PeriodId,
) -> usize {
    env.periods
        .find_period_position(period)
        .map(|p| p + 1)
        .unwrap_or(0)
}

fn slot_name(
    env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    slot: SlotId,
) -> String {
    let slot_data = env.slots.find_slot(slot);
    let subject = env
        .slots
        .find_slot_subject_and_position(slot)
        .and_then(|(subj_id, _)| env.subjects.find_subject(subj_id))
        .map(|s| s.parameters.name.as_str());
    match (subject, slot_data) {
        (Some(subj), Some(data)) => format!("{} ({})", subj, data.start_time),
        (Some(subj), None) => subj.to_string(),
        _ => format!("{:?}", slot),
    }
}

fn teacher_name(
    env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    teacher_id: collomatique_state_colloscopes::ids::TeacherId,
) -> String {
    env.teachers
        .teacher_map
        .get(&teacher_id)
        .map(|t| format!("{} {}", t.desc.firstname, t.desc.surname))
        .unwrap_or_else(|| format!("{:?}", teacher_id))
}

fn group_list_name(
    env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    group_list: GroupListId,
) -> String {
    env.group_lists
        .group_list_map
        .get(&group_list)
        .map(|gl| gl.params.name.clone())
        .unwrap_or_else(|| format!("{:?}", group_list))
}

fn group_name(
    env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    group_list: GroupListId,
    group: GroupNum,
) -> String {
    let number = group.index() + 1;
    let name = env
        .group_lists
        .group_list_map
        .get(&group_list)
        .and_then(|gl| gl.params.group_names.get(group.index()))
        .and_then(|name| name.as_ref());
    match name {
        Some(name) => format!("{} ({})", number, name),
        None => format!("{}", number),
    }
}
