use crate::ids::{
    GlobalWeek, GroupListId, GroupNum, IncompatId, PairingRuleId, PeriodId, SlotId,
    SlotPairingRuleId, StudentId, SubjectId, TeacherId,
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ExtraVarName {
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
    PairingsPenalty {
        rule: PairingRuleId,
    },
    SlotPairingsPenalty {
        rule: SlotPairingRuleId,
    },
    LimitsMaxPerDayPenalty,
    LimitsMaxPerWeekPenalty,
    LimitsMinPerWeekPenalty,
    IsLastTeacherSeen {
        subject: SubjectId,
        student: StudentId,
        teacher: TeacherId,
        week: GlobalWeek,
    },
    BalancingAvoidTwiceInARowPenalty {
        subject: SubjectId,
    },
    BalancingYearRotationPenalty {
        subject: SubjectId,
    },
    BalancingRotationPenalty {
        subject: SubjectId,
    },
    BalancingSlotRotationPenalty {
        subject: SubjectId,
    },
    BalancingPeriodRotationPenalty {
        subject: SubjectId,
    },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum InfeasibleConstraint {
    PeriodicityExactlyPeriodicInfeasible {
        student: StudentId,
        subject: SubjectId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        periodicity: u32,
    },
    PeriodicityOncePerBlockInfeasible {
        student: StudentId,
        subject: SubjectId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        weeks_per_block: u32,
    },
    BalancingAvoidTwiceUnsupported {
        subject: SubjectId,
    },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum StructuralConstraint {
    OneInterrogationAtOnce {
        student: StudentId,
        slot_a: SlotId,
        slot_b: SlotId,
        week: GlobalWeek,
    },
    IncompatSaturated {
        student: StudentId,
        incompat: IncompatId,
        subject: SubjectId,
        week: GlobalWeek,
    },
    IncompatNonSaturated {
        student: StudentId,
        incompat: IncompatId,
        subject: SubjectId,
        week: GlobalWeek,
        minimum_free_slots: u32,
    },
    StudentHasGroup {
        student: StudentId,
        group_list: GroupListId,
    },
    ForbiddenGroup {
        group_list: GroupListId,
        group: GroupNum,
        slot: SlotId,
        week: GlobalWeek,
        subject: SubjectId,
    },
    SlotPairingUsedImpliesNotUsed {
        rule: SlotPairingRuleId,
        week: GlobalWeek,
    },
    SlotPairingNotUsedImpliesNotUsed {
        rule: SlotPairingRuleId,
        week: GlobalWeek,
    },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum QualityConstraint {
    StudentsPerGroupMax {
        group_list: GroupListId,
        group: GroupNum,
        max_students: u32,
    },
    StudentsPerGroupForSubjectMax {
        group_list: GroupListId,
        group: GroupNum,
        subject: SubjectId,
        period: PeriodId,
        max_students: u32,
    },
    GroupCountPerInterrogationMax {
        slot: SlotId,
        week: GlobalWeek,
        max_groups: u32,
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
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ProgressiveConstraint {
    StudentsPerGroupMin {
        group_list: GroupListId,
        group: GroupNum,
        min_students: u32,
    },
    StudentsPerGroupForSubjectMin {
        group_list: GroupListId,
        group: GroupNum,
        subject: SubjectId,
        period: PeriodId,
        min_students: u32,
    },
    GroupCountPerInterrogationMin {
        slot: SlotId,
        week: GlobalWeek,
        min_groups: u32,
    },
    PeriodicityInterrogationCountMin {
        student: StudentId,
        subject: SubjectId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        min_count: u32,
    },
    PeriodicityInterrogationCountExact {
        student: StudentId,
        subject: SubjectId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        count: u32,
    },
    SlotPairingUsedImpliesUsed {
        rule: SlotPairingRuleId,
        week: GlobalWeek,
    },
    SlotPairingNotUsedImpliesUsed {
        rule: SlotPairingRuleId,
        week: GlobalWeek,
    },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum PreferenceConstraint {
    GroupFilledByAscendingOrder {
        group_list: GroupListId,
        group: GroupNum,
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
    BalancingAvoidTwiceInARow {
        student: StudentId,
        subject: SubjectId,
        teacher: TeacherId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
    },
    BalancingAvoidTwiceInARowRecursive {
        student: StudentId,
        subject: SubjectId,
        teacher: TeacherId,
        week: GlobalWeek,
    },
    BalancingYearRotation {
        student: StudentId,
        subject: SubjectId,
        teacher: TeacherId,
        max_count: u32,
    },
    BalancingRotation {
        student: StudentId,
        subject: SubjectId,
        teacher: TeacherId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        max_count: u32,
    },
    BalancingSlotRotation {
        student: StudentId,
        subject: SubjectId,
        slot: SlotId,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        max_count: u32,
    },
    BalancingPeriodRotation {
        student: StudentId,
        subject: SubjectId,
        teacher: TeacherId,
        period: u32,
        first_week: GlobalWeek,
        last_week: GlobalWeek,
        max_count: u32,
    },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ConstraintDesc {
    Level0(InfeasibleConstraint),
    Level1(StructuralConstraint),
    Level2(QualityConstraint),
    Level3(ProgressiveConstraint),
    Level4(PreferenceConstraint),
}

impl From<InfeasibleConstraint> for ConstraintDesc {
    fn from(c: InfeasibleConstraint) -> Self {
        ConstraintDesc::Level0(c)
    }
}

impl From<StructuralConstraint> for ConstraintDesc {
    fn from(c: StructuralConstraint) -> Self {
        ConstraintDesc::Level1(c)
    }
}

impl From<QualityConstraint> for ConstraintDesc {
    fn from(c: QualityConstraint) -> Self {
        ConstraintDesc::Level2(c)
    }
}

impl From<ProgressiveConstraint> for ConstraintDesc {
    fn from(c: ProgressiveConstraint) -> Self {
        ConstraintDesc::Level3(c)
    }
}

impl From<PreferenceConstraint> for ConstraintDesc {
    fn from(c: PreferenceConstraint) -> Self {
        ConstraintDesc::Level4(c)
    }
}

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
                let next_g_name = group_name(env, *group_list, group.next());
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

#[derive(Debug, Clone, Copy)]
struct WeekRange {
    first_week: Option<GlobalWeek>,
    last_week: Option<GlobalWeek>,
}

impl WeekRange {
    fn bounded(first_week: GlobalWeek, last_week: GlobalWeek) -> Self {
        Self {
            first_week: Some(first_week),
            last_week: Some(last_week),
        }
    }

    fn unbounded() -> Self {
        Self {
            first_week: None,
            last_week: None,
        }
    }

    fn is_subset_of(&self, other: &Self) -> bool {
        let start_ok = match (self.first_week, other.first_week) {
            (_, None) => true,
            (None, Some(_)) => false,
            (Some(a), Some(b)) => a >= b,
        };
        let end_ok = match (self.last_week, other.last_week) {
            (_, None) => true,
            (None, Some(_)) => false,
            (Some(a), Some(b)) => a <= b,
        };
        start_ok && end_ok
    }
}

#[derive(Debug, Clone, Copy)]
enum CountBoundKind {
    Upper(u32),
    Lower(u32),
    Exact(u32),
}

#[derive(Debug)]
enum ViolationFamily {
    PeriodicityCount {
        student: StudentId,
        subject: SubjectId,
        range: WeekRange,
        kind: CountBoundKind,
    },
    TeacherRotation {
        student: StudentId,
        subject: SubjectId,
        teacher: TeacherId,
        range: WeekRange,
        max_count: u32,
    },
    SlotRotation {
        student: StudentId,
        subject: SubjectId,
        slot: SlotId,
        range: WeekRange,
        max_count: u32,
    },
    StudentsInGroup {
        group_list: GroupListId,
        group: GroupNum,
        subject_scope: Option<(SubjectId, PeriodId)>,
        kind: CountBoundKind,
    },
    GroupCount {
        slot: SlotId,
        week: GlobalWeek,
        kind: CountBoundKind,
    },
    InterrogationsPerTimePeriod {
        student: StudentId,
        week: GlobalWeek,
        day: Option<collomatique_time::Weekday>,
        kind: CountBoundKind,
    },
}

impl ConstraintDesc {
    fn violation_family(&self) -> Option<ViolationFamily> {
        match self {
            ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
                student,
                subject,
                first_week,
                last_week,
                max_count,
            }) => Some(ViolationFamily::PeriodicityCount {
                student: *student,
                subject: *subject,
                range: WeekRange::bounded(*first_week, *last_week),
                kind: CountBoundKind::Upper(*max_count),
            }),
            ConstraintDesc::Level2(QualityConstraint::PeriodicitySeparation {
                student,
                subject,
                first_week,
                last_week,
            }) => Some(ViolationFamily::PeriodicityCount {
                student: *student,
                subject: *subject,
                range: WeekRange::bounded(*first_week, *last_week),
                kind: CountBoundKind::Upper(1),
            }),
            ConstraintDesc::Level3(ProgressiveConstraint::PeriodicityInterrogationCountMin {
                student,
                subject,
                first_week,
                last_week,
                min_count,
            }) => Some(ViolationFamily::PeriodicityCount {
                student: *student,
                subject: *subject,
                range: WeekRange::bounded(*first_week, *last_week),
                kind: CountBoundKind::Lower(*min_count),
            }),
            ConstraintDesc::Level3(ProgressiveConstraint::PeriodicityInterrogationCountExact {
                student,
                subject,
                first_week,
                last_week,
                count,
            }) => Some(ViolationFamily::PeriodicityCount {
                student: *student,
                subject: *subject,
                range: WeekRange::bounded(*first_week, *last_week),
                kind: CountBoundKind::Exact(*count),
            }),

            ConstraintDesc::Level4(PreferenceConstraint::BalancingRotation {
                student,
                subject,
                teacher,
                first_week,
                last_week,
                max_count,
            }) => Some(ViolationFamily::TeacherRotation {
                student: *student,
                subject: *subject,
                teacher: *teacher,
                range: WeekRange::bounded(*first_week, *last_week),
                max_count: *max_count,
            }),
            ConstraintDesc::Level4(PreferenceConstraint::BalancingPeriodRotation {
                student,
                subject,
                teacher,
                first_week,
                last_week,
                max_count,
                ..
            }) => Some(ViolationFamily::TeacherRotation {
                student: *student,
                subject: *subject,
                teacher: *teacher,
                range: WeekRange::bounded(*first_week, *last_week),
                max_count: *max_count,
            }),
            ConstraintDesc::Level4(PreferenceConstraint::BalancingYearRotation {
                student,
                subject,
                teacher,
                max_count,
            }) => Some(ViolationFamily::TeacherRotation {
                student: *student,
                subject: *subject,
                teacher: *teacher,
                range: WeekRange::unbounded(),
                max_count: *max_count,
            }),

            ConstraintDesc::Level4(PreferenceConstraint::BalancingSlotRotation {
                student,
                subject,
                slot,
                first_week,
                last_week,
                max_count,
            }) => Some(ViolationFamily::SlotRotation {
                student: *student,
                subject: *subject,
                slot: *slot,
                range: WeekRange::bounded(*first_week, *last_week),
                max_count: *max_count,
            }),

            ConstraintDesc::Level2(QualityConstraint::StudentsPerGroupMax {
                group_list,
                group,
                max_students,
            }) => Some(ViolationFamily::StudentsInGroup {
                group_list: *group_list,
                group: *group,
                subject_scope: None,
                kind: CountBoundKind::Upper(*max_students),
            }),
            ConstraintDesc::Level2(QualityConstraint::StudentsPerGroupForSubjectMax {
                group_list,
                group,
                subject,
                period,
                max_students,
            }) => Some(ViolationFamily::StudentsInGroup {
                group_list: *group_list,
                group: *group,
                subject_scope: Some((*subject, *period)),
                kind: CountBoundKind::Upper(*max_students),
            }),
            ConstraintDesc::Level3(ProgressiveConstraint::StudentsPerGroupMin {
                group_list,
                group,
                min_students,
            }) => Some(ViolationFamily::StudentsInGroup {
                group_list: *group_list,
                group: *group,
                subject_scope: None,
                kind: CountBoundKind::Lower(*min_students),
            }),
            ConstraintDesc::Level3(ProgressiveConstraint::StudentsPerGroupForSubjectMin {
                group_list,
                group,
                subject,
                period,
                min_students,
            }) => Some(ViolationFamily::StudentsInGroup {
                group_list: *group_list,
                group: *group,
                subject_scope: Some((*subject, *period)),
                kind: CountBoundKind::Lower(*min_students),
            }),

            ConstraintDesc::Level2(QualityConstraint::GroupCountPerInterrogationMax {
                slot,
                week,
                max_groups,
            }) => Some(ViolationFamily::GroupCount {
                slot: *slot,
                week: *week,
                kind: CountBoundKind::Upper(*max_groups),
            }),
            ConstraintDesc::Level3(ProgressiveConstraint::GroupCountPerInterrogationMin {
                slot,
                week,
                min_groups,
            }) => Some(ViolationFamily::GroupCount {
                slot: *slot,
                week: *week,
                kind: CountBoundKind::Lower(*min_groups),
            }),

            ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerDay {
                student,
                week,
                day,
                max,
            }) => Some(ViolationFamily::InterrogationsPerTimePeriod {
                student: *student,
                week: *week,
                day: Some(*day),
                kind: CountBoundKind::Upper(*max),
            }),
            ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerWeek {
                student,
                week,
                max,
            }) => Some(ViolationFamily::InterrogationsPerTimePeriod {
                student: *student,
                week: *week,
                day: None,
                kind: CountBoundKind::Upper(*max),
            }),
            ConstraintDesc::Level4(PreferenceConstraint::MinInterrogationsPerWeek {
                student,
                week,
                min,
            }) => Some(ViolationFamily::InterrogationsPerTimePeriod {
                student: *student,
                week: *week,
                day: None,
                kind: CountBoundKind::Lower(*min),
            }),

            _ => None,
        }
    }

    /// Returns true if violating `self` necessarily implies violating `other`.
    pub fn violation_implies(&self, other: &Self) -> bool {
        if self == other {
            return true;
        }

        let (Some(self_fam), Some(other_fam)) = (self.violation_family(), other.violation_family())
        else {
            return false;
        };

        match (&self_fam, &other_fam) {
            (
                ViolationFamily::PeriodicityCount {
                    student: s1,
                    subject: sub1,
                    range: r1,
                    kind: k1,
                },
                ViolationFamily::PeriodicityCount {
                    student: s2,
                    subject: sub2,
                    range: r2,
                    kind: k2,
                },
            ) => s1 == s2 && sub1 == sub2 && ranged_bound_implies(r1, k1, r2, k2),

            (
                ViolationFamily::TeacherRotation {
                    student: s1,
                    subject: sub1,
                    teacher: t1,
                    range: r1,
                    max_count: n1,
                },
                ViolationFamily::TeacherRotation {
                    student: s2,
                    subject: sub2,
                    teacher: t2,
                    range: r2,
                    max_count: n2,
                },
            ) => {
                s1 == s2
                    && sub1 == sub2
                    && t1 == t2
                    && ranged_bound_implies(
                        r1,
                        &CountBoundKind::Upper(*n1),
                        r2,
                        &CountBoundKind::Upper(*n2),
                    )
            }

            (
                ViolationFamily::SlotRotation {
                    student: s1,
                    subject: sub1,
                    slot: sl1,
                    range: r1,
                    max_count: n1,
                },
                ViolationFamily::SlotRotation {
                    student: s2,
                    subject: sub2,
                    slot: sl2,
                    range: r2,
                    max_count: n2,
                },
            ) => {
                s1 == s2
                    && sub1 == sub2
                    && sl1 == sl2
                    && ranged_bound_implies(
                        r1,
                        &CountBoundKind::Upper(*n1),
                        r2,
                        &CountBoundKind::Upper(*n2),
                    )
            }

            (
                ViolationFamily::StudentsInGroup {
                    group_list: gl1,
                    group: g1,
                    subject_scope: sc1,
                    kind: k1,
                },
                ViolationFamily::StudentsInGroup {
                    group_list: gl2,
                    group: g2,
                    subject_scope: sc2,
                    kind: k2,
                },
            ) => gl1 == gl2 && g1 == g2 && students_in_group_implies(sc1, k1, sc2, k2),

            (
                ViolationFamily::GroupCount {
                    slot: sl1,
                    week: w1,
                    kind: k1,
                },
                ViolationFamily::GroupCount {
                    slot: sl2,
                    week: w2,
                    kind: k2,
                },
            ) => sl1 == sl2 && w1 == w2 && bound_implies(k1, k2),

            (
                ViolationFamily::InterrogationsPerTimePeriod {
                    student: s1,
                    week: w1,
                    day: d1,
                    kind: k1,
                },
                ViolationFamily::InterrogationsPerTimePeriod {
                    student: s2,
                    week: w2,
                    day: d2,
                    kind: k2,
                },
            ) => s1 == s2 && w1 == w2 && time_period_implies(d1, k1, d2, k2),

            _ => false,
        }
    }
}

fn bound_implies(a: &CountBoundKind, b: &CountBoundKind) -> bool {
    match (a, b) {
        (CountBoundKind::Upper(n1), CountBoundKind::Upper(n2)) => n1 >= n2,
        (CountBoundKind::Upper(n), CountBoundKind::Exact(m)) => n >= m,
        (CountBoundKind::Lower(m1), CountBoundKind::Lower(m2)) => m1 <= m2,
        (CountBoundKind::Lower(m), CountBoundKind::Exact(n)) => m <= n,
        _ => false,
    }
}

fn ranged_bound_implies(
    range_a: &WeekRange,
    kind_a: &CountBoundKind,
    range_b: &WeekRange,
    kind_b: &CountBoundKind,
) -> bool {
    match (kind_a, kind_b) {
        (CountBoundKind::Upper(n1), CountBoundKind::Upper(n2)) => {
            range_a.is_subset_of(range_b) && n1 >= n2
        }
        (CountBoundKind::Upper(n), CountBoundKind::Exact(m)) => {
            range_a.is_subset_of(range_b) && n >= m
        }

        (CountBoundKind::Lower(m1), CountBoundKind::Lower(m2)) => {
            range_b.is_subset_of(range_a) && m1 <= m2
        }
        (CountBoundKind::Lower(m), CountBoundKind::Exact(n)) => {
            range_b.is_subset_of(range_a) && m <= n
        }

        _ => false,
    }
}

fn students_in_group_implies(
    scope_a: &Option<(SubjectId, PeriodId)>,
    kind_a: &CountBoundKind,
    scope_b: &Option<(SubjectId, PeriodId)>,
    kind_b: &CountBoundKind,
) -> bool {
    match (scope_a, scope_b) {
        (None, None) => bound_implies(kind_a, kind_b),
        (Some((s1, p1)), Some((s2, p2))) if s1 == s2 && p1 == p2 => bound_implies(kind_a, kind_b),

        (Some(_), None) => match (kind_a, kind_b) {
            (CountBoundKind::Upper(n1), CountBoundKind::Upper(n2)) => n1 >= n2,
            (CountBoundKind::Upper(n), CountBoundKind::Exact(m)) => n >= m,
            _ => false,
        },

        (None, Some(_)) => match (kind_a, kind_b) {
            (CountBoundKind::Lower(m1), CountBoundKind::Lower(m2)) => m1 <= m2,
            (CountBoundKind::Lower(m), CountBoundKind::Exact(n)) => m <= n,
            _ => false,
        },

        _ => false,
    }
}

fn time_period_implies(
    day_a: &Option<collomatique_time::Weekday>,
    kind_a: &CountBoundKind,
    day_b: &Option<collomatique_time::Weekday>,
    kind_b: &CountBoundKind,
) -> bool {
    match (day_a, day_b) {
        (None, None) => bound_implies(kind_a, kind_b),
        (Some(d1), Some(d2)) if d1 == d2 => bound_implies(kind_a, kind_b),

        (Some(_), None) => match (kind_a, kind_b) {
            (CountBoundKind::Upper(n1), CountBoundKind::Upper(n2)) => n1 >= n2,
            (CountBoundKind::Upper(n), CountBoundKind::Exact(m)) => n >= m,
            _ => false,
        },

        (None, Some(_)) => match (kind_a, kind_b) {
            (CountBoundKind::Lower(m1), CountBoundKind::Lower(m2)) => m1 <= m2,
            (CountBoundKind::Lower(m), CountBoundKind::Exact(n)) => m <= n,
            _ => false,
        },

        _ => false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use collomatique_state_colloscopes::ids::Id;

    fn student(n: u64) -> StudentId {
        unsafe { StudentId::new(n) }
    }
    fn subject(n: u64) -> SubjectId {
        unsafe { SubjectId::new(n) }
    }
    fn teacher(n: u64) -> TeacherId {
        unsafe { TeacherId::new(n) }
    }
    fn slot(n: u64) -> SlotId {
        unsafe { SlotId::new(n) }
    }
    fn group_list(n: u64) -> GroupListId {
        unsafe { GroupListId::new(n) }
    }
    fn period(n: u64) -> PeriodId {
        unsafe { PeriodId::new(n) }
    }
    fn week(n: usize) -> GlobalWeek {
        GlobalWeek(n)
    }
    fn group(n: usize) -> GroupNum {
        GroupNum(n)
    }

    #[test]
    fn reflexivity() {
        let c = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        assert!(c.violation_implies(&c));
    }

    #[test]
    fn no_family_reflexivity() {
        let c = ConstraintDesc::Level1(StructuralConstraint::StudentHasGroup {
            student: student(1),
            group_list: group_list(1),
        });
        assert!(c.violation_implies(&c));
        let c2 = ConstraintDesc::Level1(StructuralConstraint::StudentHasGroup {
            student: student(2),
            group_list: group_list(1),
        });
        assert!(!c.violation_implies(&c2));
    }

    // === Family A: Periodicity ===

    #[test]
    fn max_implies_exact_same_range_same_bound() {
        let max = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        let exact =
            ConstraintDesc::Level3(ProgressiveConstraint::PeriodicityInterrogationCountExact {
                student: student(1),
                subject: subject(1),
                first_week: week(0),
                last_week: week(5),
                count: 3,
            });
        assert!(max.violation_implies(&exact));
        assert!(!exact.violation_implies(&max));
    }

    #[test]
    fn max_higher_bound_implies_max_lower_bound_same_range() {
        let max5 = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(10),
            max_count: 5,
        });
        let max3 = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(10),
            max_count: 3,
        });
        assert!(max5.violation_implies(&max3));
        assert!(!max3.violation_implies(&max5));
    }

    #[test]
    fn max_inner_range_implies_max_outer_range_same_bound() {
        let inner = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(2),
            last_week: week(5),
            max_count: 3,
        });
        let outer = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(10),
            max_count: 3,
        });
        assert!(inner.violation_implies(&outer));
        assert!(!outer.violation_implies(&inner));
    }

    #[test]
    fn separation_implies_max_1_when_nested() {
        let sep = ConstraintDesc::Level2(QualityConstraint::PeriodicitySeparation {
            student: student(1),
            subject: subject(1),
            first_week: week(2),
            last_week: week(3),
        });
        let max = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 1,
        });
        assert!(sep.violation_implies(&max));
        assert!(!max.violation_implies(&sep));
    }

    #[test]
    fn separation_does_not_imply_max_2() {
        let sep = ConstraintDesc::Level2(QualityConstraint::PeriodicitySeparation {
            student: student(1),
            subject: subject(1),
            first_week: week(2),
            last_week: week(3),
        });
        let max = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 2,
        });
        assert!(!sep.violation_implies(&max));
    }

    #[test]
    fn min_implies_exact_same_range() {
        let min = ConstraintDesc::Level3(ProgressiveConstraint::PeriodicityInterrogationCountMin {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            min_count: 3,
        });
        let exact =
            ConstraintDesc::Level3(ProgressiveConstraint::PeriodicityInterrogationCountExact {
                student: student(1),
                subject: subject(1),
                first_week: week(0),
                last_week: week(5),
                count: 3,
            });
        assert!(min.violation_implies(&exact));
        assert!(!exact.violation_implies(&min));
    }

    #[test]
    fn min_outer_range_implies_min_inner_range() {
        let outer =
            ConstraintDesc::Level3(ProgressiveConstraint::PeriodicityInterrogationCountMin {
                student: student(1),
                subject: subject(1),
                first_week: week(0),
                last_week: week(10),
                min_count: 3,
            });
        let inner =
            ConstraintDesc::Level3(ProgressiveConstraint::PeriodicityInterrogationCountMin {
                student: student(1),
                subject: subject(1),
                first_week: week(2),
                last_week: week(5),
                min_count: 4,
            });
        assert!(outer.violation_implies(&inner));
        assert!(!inner.violation_implies(&outer));
    }

    #[test]
    fn max_does_not_imply_min() {
        let max = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        let min = ConstraintDesc::Level3(ProgressiveConstraint::PeriodicityInterrogationCountMin {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            min_count: 2,
        });
        assert!(!max.violation_implies(&min));
        assert!(!min.violation_implies(&max));
    }

    #[test]
    fn different_students_incomparable() {
        let a = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        let b = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(2),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        assert!(!a.violation_implies(&b));
        assert!(!b.violation_implies(&a));
    }

    // === Family B: Teacher rotation ===

    #[test]
    fn rotation_implies_year_rotation() {
        let rot = ConstraintDesc::Level4(PreferenceConstraint::BalancingRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        let year = ConstraintDesc::Level4(PreferenceConstraint::BalancingYearRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            max_count: 2,
        });
        assert!(rot.violation_implies(&year));
        assert!(!year.violation_implies(&rot));
    }

    #[test]
    fn rotation_does_not_imply_year_rotation_when_bound_too_low() {
        let rot = ConstraintDesc::Level4(PreferenceConstraint::BalancingRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 2,
        });
        let year = ConstraintDesc::Level4(PreferenceConstraint::BalancingYearRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            max_count: 4,
        });
        assert!(!rot.violation_implies(&year));
    }

    #[test]
    fn rotation_implies_period_rotation_when_nested() {
        let rot = ConstraintDesc::Level4(PreferenceConstraint::BalancingRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            first_week: week(2),
            last_week: week(5),
            max_count: 3,
        });
        let period_rot = ConstraintDesc::Level4(PreferenceConstraint::BalancingPeriodRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            period: 1,
            first_week: week(0),
            last_week: week(10),
            max_count: 2,
        });
        assert!(rot.violation_implies(&period_rot));
        assert!(!period_rot.violation_implies(&rot));
    }

    #[test]
    fn year_rotation_higher_bound_implies_lower() {
        let year5 = ConstraintDesc::Level4(PreferenceConstraint::BalancingYearRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            max_count: 5,
        });
        let year3 = ConstraintDesc::Level4(PreferenceConstraint::BalancingYearRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            max_count: 3,
        });
        assert!(year5.violation_implies(&year3));
        assert!(!year3.violation_implies(&year5));
    }

    #[test]
    fn different_teachers_incomparable() {
        let a = ConstraintDesc::Level4(PreferenceConstraint::BalancingRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        let b = ConstraintDesc::Level4(PreferenceConstraint::BalancingRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(2),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        assert!(!a.violation_implies(&b));
    }

    // === Family C: Slot rotation ===

    #[test]
    fn slot_rotation_nested_ranges() {
        let inner = ConstraintDesc::Level4(PreferenceConstraint::BalancingSlotRotation {
            student: student(1),
            subject: subject(1),
            slot: slot(1),
            first_week: week(0),
            last_week: week(3),
            max_count: 2,
        });
        let outer = ConstraintDesc::Level4(PreferenceConstraint::BalancingSlotRotation {
            student: student(1),
            subject: subject(1),
            slot: slot(1),
            first_week: week(0),
            last_week: week(10),
            max_count: 2,
        });
        assert!(inner.violation_implies(&outer));
        assert!(!outer.violation_implies(&inner));
    }

    // === Family D: Students in group ===

    #[test]
    fn for_subject_max_implies_group_max() {
        let for_subj = ConstraintDesc::Level2(QualityConstraint::StudentsPerGroupForSubjectMax {
            group_list: group_list(1),
            group: group(0),
            subject: subject(1),
            period: period(1),
            max_students: 5,
        });
        let total = ConstraintDesc::Level2(QualityConstraint::StudentsPerGroupMax {
            group_list: group_list(1),
            group: group(0),
            max_students: 4,
        });
        assert!(for_subj.violation_implies(&total));
        assert!(!total.violation_implies(&for_subj));
    }

    #[test]
    fn group_min_implies_for_subject_min() {
        let total = ConstraintDesc::Level3(ProgressiveConstraint::StudentsPerGroupMin {
            group_list: group_list(1),
            group: group(0),
            min_students: 2,
        });
        let for_subj =
            ConstraintDesc::Level3(ProgressiveConstraint::StudentsPerGroupForSubjectMin {
                group_list: group_list(1),
                group: group(0),
                subject: subject(1),
                period: period(1),
                min_students: 3,
            });
        assert!(total.violation_implies(&for_subj));
        assert!(!for_subj.violation_implies(&total));
    }

    #[test]
    fn group_max_does_not_imply_for_subject_max() {
        let total = ConstraintDesc::Level2(QualityConstraint::StudentsPerGroupMax {
            group_list: group_list(1),
            group: group(0),
            max_students: 10,
        });
        let for_subj = ConstraintDesc::Level2(QualityConstraint::StudentsPerGroupForSubjectMax {
            group_list: group_list(1),
            group: group(0),
            subject: subject(1),
            period: period(1),
            max_students: 5,
        });
        assert!(!total.violation_implies(&for_subj));
    }

    // === Family E: Group count ===

    #[test]
    fn group_count_max_higher_implies_lower() {
        let max5 = ConstraintDesc::Level2(QualityConstraint::GroupCountPerInterrogationMax {
            slot: slot(1),
            week: week(0),
            max_groups: 5,
        });
        let max3 = ConstraintDesc::Level2(QualityConstraint::GroupCountPerInterrogationMax {
            slot: slot(1),
            week: week(0),
            max_groups: 3,
        });
        assert!(max5.violation_implies(&max3));
        assert!(!max3.violation_implies(&max5));
    }

    #[test]
    fn group_count_max_does_not_imply_min() {
        let max = ConstraintDesc::Level2(QualityConstraint::GroupCountPerInterrogationMax {
            slot: slot(1),
            week: week(0),
            max_groups: 5,
        });
        let min = ConstraintDesc::Level3(ProgressiveConstraint::GroupCountPerInterrogationMin {
            slot: slot(1),
            week: week(0),
            min_groups: 2,
        });
        assert!(!max.violation_implies(&min));
        assert!(!min.violation_implies(&max));
    }

    // === Family F: Interrogations per time period ===

    #[test]
    fn max_per_day_implies_max_per_week() {
        let day = ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerDay {
            student: student(1),
            week: week(0),
            day: collomatique_time::Weekday::iter().next().unwrap(),
            max: 3,
        });
        let wk = ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerWeek {
            student: student(1),
            week: week(0),
            max: 2,
        });
        assert!(day.violation_implies(&wk));
        assert!(!wk.violation_implies(&day));
    }

    #[test]
    fn max_per_day_does_not_imply_max_per_week_when_bound_too_low() {
        let day = ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerDay {
            student: student(1),
            week: week(0),
            day: collomatique_time::Weekday::iter().next().unwrap(),
            max: 2,
        });
        let wk = ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerWeek {
            student: student(1),
            week: week(0),
            max: 5,
        });
        assert!(!day.violation_implies(&wk));
    }

    #[test]
    fn max_per_week_does_not_imply_max_per_day() {
        let wk = ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerWeek {
            student: student(1),
            week: week(0),
            max: 3,
        });
        let day = ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerDay {
            student: student(1),
            week: week(0),
            day: collomatique_time::Weekday::iter().next().unwrap(),
            max: 2,
        });
        assert!(!wk.violation_implies(&day));
    }

    #[test]
    fn different_days_incomparable() {
        let mon = ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerDay {
            student: student(1),
            week: week(0),
            day: collomatique_time::Weekday::iter().next().unwrap(),
            max: 3,
        });
        let tue = ConstraintDesc::Level4(PreferenceConstraint::MaxInterrogationsPerDay {
            student: student(1),
            week: week(0),
            day: collomatique_time::Weekday::iter().nth(1).unwrap(),
            max: 3,
        });
        assert!(!mon.violation_implies(&tue));
    }

    // === Cross-family ===

    #[test]
    fn periodicity_vs_rotation_incomparable() {
        let periodicity =
            ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
                student: student(1),
                subject: subject(1),
                first_week: week(0),
                last_week: week(5),
                max_count: 3,
            });
        let rotation = ConstraintDesc::Level4(PreferenceConstraint::BalancingRotation {
            student: student(1),
            subject: subject(1),
            teacher: teacher(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        assert!(!periodicity.violation_implies(&rotation));
        assert!(!rotation.violation_implies(&periodicity));
    }

    #[test]
    fn no_family_vs_family_incomparable() {
        let structural = ConstraintDesc::Level1(StructuralConstraint::StudentHasGroup {
            student: student(1),
            group_list: group_list(1),
        });
        let max = ConstraintDesc::Level2(QualityConstraint::PeriodicityInterrogationCountMax {
            student: student(1),
            subject: subject(1),
            first_week: week(0),
            last_week: week(5),
            max_count: 3,
        });
        assert!(!structural.violation_implies(&max));
        assert!(!max.violation_implies(&structural));
    }
}
