use crate::ids::{
    GlobalWeek, GroupListId, GroupNum, IncompatId, PairingRuleId, PeriodId, SlotId,
    SlotPairingRuleId, StudentId, SubjectId,
};
use collo_ml::SqliteDatabaseConnection;
use collo_ml::eval::Origin;
use collo_ml::script_feeder::ReifiedVar;
use derivative::Derivative;

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    Hash(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub enum ExtraVarName {
    Script(ReifiedVar<SqliteDatabaseConnection>),
    GroupInInterrogation {
        slot: SlotId,
        week: GlobalWeek,
        group: GroupNum,
    },
    InterrogationHasGroups {
        slot: SlotId,
        week: GlobalWeek,
    },
    StudentInGroup {
        student: StudentId,
        group_list: GroupListId,
        group: GroupNum,
    },
    GroupHasStudents {
        group_list: GroupListId,
        group: GroupNum,
    },
    GroupHasStudentsForSubject {
        group_list: GroupListId,
        group: GroupNum,
        subject: SubjectId,
        period: PeriodId,
    },
    StudentAtInterrogationInGroup {
        student: StudentId,
        slot: SlotId,
        week: GlobalWeek,
        group_list: GroupListId,
        group: GroupNum,
    },
    StudentAtInterrogation {
        student: StudentId,
        slot: SlotId,
        week: GlobalWeek,
    },
    StudentNotAtIncompatSlot {
        student: StudentId,
        incompat: IncompatId,
        incompat_slot_index: usize,
        week: GlobalWeek,
    },
    StudentHasInterrogationIn {
        student: StudentId,
        subject: SubjectId,
        week: GlobalWeek,
    },
    PairingsPenalty,
    SlotPairingsPenalty,
    LimitsMaxPerDayPenalty,
    LimitsMaxPerWeekPenalty,
    LimitsMinPerWeekPenalty,
}

impl From<ReifiedVar<SqliteDatabaseConnection>> for ExtraVarName {
    fn from(v: ReifiedVar<SqliteDatabaseConnection>) -> Self {
        ExtraVarName::Script(v)
    }
}

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    Hash(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub enum ConstraintDesc {
    Script(Option<Origin<SqliteDatabaseConnection>>),
    StudentsPerGroupMin {
        group_list: GroupListId,
        group: GroupNum,
        min_students: u32,
    },
    StudentsPerGroupMax {
        group_list: GroupListId,
        group: GroupNum,
        max_students: u32,
    },
    StudentHasGroup {
        student: StudentId,
        group_list: GroupListId,
    },
    StudentsPerGroupForSubjectMin {
        group_list: GroupListId,
        group: GroupNum,
        subject: SubjectId,
        period: PeriodId,
        min_students: u32,
    },
    StudentsPerGroupForSubjectMax {
        group_list: GroupListId,
        group: GroupNum,
        subject: SubjectId,
        period: PeriodId,
        max_students: u32,
    },
    GroupFilledByAscendingOrder {
        group_list: GroupListId,
        group: GroupNum,
    },
    ForbiddenGroup {
        group_list: GroupListId,
        group: GroupNum,
        slot: SlotId,
        week: GlobalWeek,
        subject: SubjectId,
    },
    GroupCountPerInterrogationMin {
        slot: SlotId,
        week: GlobalWeek,
        min_groups: u32,
    },
    GroupCountPerInterrogationMax {
        slot: SlotId,
        week: GlobalWeek,
        max_groups: u32,
    },
    OneInterrogationAtOnce {
        student: StudentId,
        slot_a: SlotId,
        slot_b: SlotId,
        week: GlobalWeek,
    },
    MaxInterrogationsPerDay {
        student: StudentId,
        week: GlobalWeek,
        day: collomatique_time::Weekday,
        max: u32,
    },
    MaxInterrogationsPerWeek {
        student: StudentId,
        week: GlobalWeek,
        max: u32,
    },
    MinInterrogationsPerWeek {
        student: StudentId,
        week: GlobalWeek,
        min: u32,
    },
    IncompatSaturated {
        student: StudentId,
        incompat: IncompatId,
        week: GlobalWeek,
    },
    IncompatNonSaturated {
        student: StudentId,
        incompat: IncompatId,
        week: GlobalWeek,
        minimum_free_slots: u32,
    },
    PairingHavingImpliesHaving {
        student: StudentId,
        week: GlobalWeek,
        rule: PairingRuleId,
    },
    PairingHavingImpliesNotHaving {
        student: StudentId,
        week: GlobalWeek,
        rule: PairingRuleId,
    },
    PairingNotHavingImpliesHaving {
        student: StudentId,
        week: GlobalWeek,
        rule: PairingRuleId,
    },
    PairingNotHavingImpliesNotHaving {
        student: StudentId,
        week: GlobalWeek,
        rule: PairingRuleId,
    },
    SlotPairingUsedImpliesUsed {
        rule: SlotPairingRuleId,
        week: GlobalWeek,
    },
    SlotPairingUsedImpliesNotUsed {
        rule: SlotPairingRuleId,
        week: GlobalWeek,
    },
    SlotPairingNotUsedImpliesUsed {
        rule: SlotPairingRuleId,
        week: GlobalWeek,
    },
    SlotPairingNotUsedImpliesNotUsed {
        rule: SlotPairingRuleId,
        week: GlobalWeek,
    },
    PeriodicityInterrogationCountExact {
        student: StudentId,
        subject: SubjectId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        count: u32,
    },
    PeriodicityInterrogationCountMin {
        student: StudentId,
        subject: SubjectId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        min_count: u32,
    },
    PeriodicityInterrogationCountMax {
        student: StudentId,
        subject: SubjectId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        max_count: u32,
    },
    PeriodicitySeparation {
        student: StudentId,
        subject: SubjectId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
    },
    PeriodicityExactlyPeriodicInfeasible {
        student: StudentId,
        subject: SubjectId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        periodicity: u32,
    },
}

impl ConstraintDesc {
    pub fn user_readable(
        &self,
        env: &collomatique_state_colloscopes::colloscope_params::Parameters,
    ) -> String {
        match self {
            ConstraintDesc::Script(Some(origin)) => origin.to_string(),
            ConstraintDesc::Script(None) => "Script (origine inconnue)".to_string(),
            ConstraintDesc::StudentsPerGroupMin {
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
            ConstraintDesc::StudentsPerGroupMax {
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
            ConstraintDesc::StudentHasGroup {
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
            ConstraintDesc::StudentsPerGroupForSubjectMin {
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
            ConstraintDesc::StudentsPerGroupForSubjectMax {
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
            ConstraintDesc::GroupFilledByAscendingOrder { group_list, group } => {
                let gl_name = group_list_name(env, *group_list);
                let g_name = group_name(env, *group_list, *group);
                let next_g_name = group_name(env, *group_list, group.next());
                format!(
                    "Le groupe {} de la liste {} doit être rempli avant le groupe {}",
                    g_name, gl_name, next_g_name,
                )
            }
            ConstraintDesc::ForbiddenGroup {
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
            ConstraintDesc::GroupCountPerInterrogationMin {
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
            ConstraintDesc::GroupCountPerInterrogationMax {
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
            ConstraintDesc::OneInterrogationAtOnce {
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
            ConstraintDesc::MaxInterrogationsPerDay {
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
            ConstraintDesc::MaxInterrogationsPerWeek { student, week, max } => {
                let s_name = student_name(env, *student);
                format!(
                    "Au maximum {} colle(s) la semaine {} pour l'élève {}",
                    max,
                    week.0 + 1,
                    s_name,
                )
            }
            ConstraintDesc::MinInterrogationsPerWeek { student, week, min } => {
                let s_name = student_name(env, *student);
                format!(
                    "Au minimum {} colle(s) la semaine {} pour l'élève {}",
                    min,
                    week.0 + 1,
                    s_name,
                )
            }
            ConstraintDesc::IncompatSaturated {
                student,
                incompat,
                week,
            } => {
                let s_name = student_name(env, *student);
                let i_name = incompat_name(env, *incompat);
                format!(
                    "L'élève {} ne doit pas avoir de colle la semaine {} pendant l'incompatibilité {}",
                    s_name,
                    week.0 + 1,
                    i_name,
                )
            }
            ConstraintDesc::IncompatNonSaturated {
                student,
                incompat,
                week,
                minimum_free_slots,
            } => {
                let s_name = student_name(env, *student);
                let i_name = incompat_name(env, *incompat);
                format!(
                    "Au moins {} créneau(x) disponible(s) pour l'élève {} la semaine {} (incompatibilité {})",
                    minimum_free_slots,
                    s_name,
                    week.0 + 1,
                    i_name,
                )
            }
            ConstraintDesc::PairingHavingImpliesHaving {
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
            ConstraintDesc::PairingHavingImpliesNotHaving {
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
            ConstraintDesc::PairingNotHavingImpliesHaving {
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
            ConstraintDesc::PairingNotHavingImpliesNotHaving {
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
            ConstraintDesc::SlotPairingUsedImpliesUsed { rule, week } => {
                let (subj, ant, con) = slot_pairing_info(env, *rule);
                format!(
                    "{} semaine {} : si le créneau ({}) est utilisé, le créneau ({}) doit aussi être utilisé",
                    subj,
                    week.0 + 1,
                    ant,
                    con,
                )
            }
            ConstraintDesc::SlotPairingUsedImpliesNotUsed { rule, week } => {
                let (subj, ant, con) = slot_pairing_info(env, *rule);
                format!(
                    "{} semaine {} : si le créneau ({}) est utilisé, le créneau ({}) ne doit pas être utilisé",
                    subj,
                    week.0 + 1,
                    ant,
                    con,
                )
            }
            ConstraintDesc::SlotPairingNotUsedImpliesUsed { rule, week } => {
                let (subj, ant, con) = slot_pairing_info(env, *rule);
                format!(
                    "{} semaine {} : si le créneau ({}) n'est pas utilisé, le créneau ({}) doit être utilisé",
                    subj,
                    week.0 + 1,
                    ant,
                    con,
                )
            }
            ConstraintDesc::SlotPairingNotUsedImpliesNotUsed { rule, week } => {
                let (subj, ant, con) = slot_pairing_info(env, *rule);
                format!(
                    "{} semaine {} : si le créneau ({}) n'est pas utilisé, le créneau ({}) ne doit pas être utilisé non plus",
                    subj,
                    week.0 + 1,
                    ant,
                    con,
                )
            }
            ConstraintDesc::PeriodicityInterrogationCountExact {
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
                        "{} ne doit pas avoir de colle en {} entre la semaine {} et la semaine {}",
                        s_name,
                        subj_name,
                        first_week.0 + 1,
                        last_week.0 + 1,
                    )
                } else {
                    let plural = if *count > 1 { "s" } else { "" };
                    format!(
                        "{} doit avoir exactement {} colle{} en {} entre la semaine {} et la semaine {}",
                        s_name,
                        count,
                        plural,
                        subj_name,
                        first_week.0 + 1,
                        last_week.0 + 1,
                    )
                }
            }
            ConstraintDesc::PeriodicityInterrogationCountMin {
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
                    "{} doit avoir au moins {} colle{} en {} entre la semaine {} et la semaine {}",
                    s_name,
                    min_count,
                    plural,
                    subj_name,
                    first_week.0 + 1,
                    last_week.0 + 1,
                )
            }
            ConstraintDesc::PeriodicityInterrogationCountMax {
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
                    "{} doit avoir au plus {} colle{} en {} entre la semaine {} et la semaine {}",
                    s_name,
                    max_count,
                    plural,
                    subj_name,
                    first_week.0 + 1,
                    last_week.0 + 1,
                )
            }
            ConstraintDesc::PeriodicitySeparation {
                student,
                subject,
                first_week,
                last_week,
            } => {
                let s_name = student_name(env, *student);
                let subj_name = subject_name(env, *subject);
                format!(
                    "{} ne doit pas avoir plus d'une colle en {} entre la semaine {} et la semaine {}",
                    s_name,
                    subj_name,
                    first_week.0 + 1,
                    last_week.0 + 1,
                )
            }
            ConstraintDesc::PeriodicityExactlyPeriodicInfeasible {
                student,
                subject,
                first_week,
                last_week,
                periodicity,
            } => {
                let s_name = student_name(env, *student);
                let subj_name = subject_name(env, *subject);
                format!(
                    "Pas assez de semaines actives pour une périodicité exacte de {} semaine(s) en {} pour {} (semaines {} à {})",
                    periodicity,
                    subj_name,
                    s_name,
                    first_week.0 + 1,
                    last_week.0 + 1,
                )
            }
        }
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
        .ordered_period_list
        .iter()
        .position(|(id, _)| *id == period)
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
        .and_then(|(subj_id, _)| {
            env.subjects
                .ordered_subject_list
                .iter()
                .find(|(id, _)| *id == subj_id)
                .map(|(_, s)| s.parameters.name.as_str())
        });
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
    let number = group.0 + 1;
    let name = env
        .group_lists
        .group_list_map
        .get(&group_list)
        .and_then(|gl| gl.params.group_names.get(group.0))
        .and_then(|name| name.as_ref());
    match name {
        Some(name) => format!("{} ({})", number, name),
        None => format!("{}", number),
    }
}

impl From<Option<Origin<SqliteDatabaseConnection>>> for ConstraintDesc {
    fn from(v: Option<Origin<SqliteDatabaseConnection>>) -> Self {
        ConstraintDesc::Script(v)
    }
}
