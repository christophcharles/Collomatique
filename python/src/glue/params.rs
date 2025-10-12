use super::{time::SlotWithDurationError, *};

use std::collections::BTreeSet;

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneralParameters {
    #[pyo3(get)]
    pub periods: Vec<Period>,
    #[pyo3(get)]
    pub periods_first_week: Option<time::NaiveMondayDate>,
    #[pyo3(get)]
    pub subjects: Vec<subjects::Subject>,
    #[pyo3(get)]
    pub teachers: BTreeMap<TeacherId, Teacher>,
    #[pyo3(get)]
    pub students: BTreeMap<StudentId, Student>,
    #[pyo3(get)]
    pub assignments: BTreeMap<PeriodId, BTreeMap<SubjectId, BTreeSet<StudentId>>>,
    #[pyo3(get)]
    pub week_patterns: BTreeMap<WeekPatternId, WeekPattern>,
    #[pyo3(get)]
    pub slots: BTreeMap<subjects::SubjectId, Vec<slots::Slot>>,
    #[pyo3(get)]
    pub incompats: BTreeMap<incompatibilities::IncompatId, incompatibilities::Incompat>,
    #[pyo3(get)]
    pub group_lists: BTreeMap<group_lists::GroupListId, group_lists::GroupList>,
    #[pyo3(get)]
    pub group_lists_associations: BTreeMap<PeriodId, BTreeMap<SubjectId, group_lists::GroupListId>>,
    #[pyo3(get)]
    pub rules: BTreeMap<rules::RuleId, rules::Rule>,
    #[pyo3(get)]
    pub settings: settings::GeneralSettings,
}

impl TryFrom<collomatique_state_colloscopes::colloscope_params::GeneralParameters>
    for GeneralParameters
{
    type Error = PyErr;
    fn try_from(
        value: collomatique_state_colloscopes::colloscope_params::GeneralParameters,
    ) -> PyResult<Self> {
        Ok(GeneralParameters {
            periods: value
                .periods
                .ordered_period_list
                .into_iter()
                .map(|(period_id, weeks_status)| Period {
                    id: period_id.into(),
                    weeks_status,
                })
                .collect(),
            periods_first_week: match value.periods.first_week {
                Some(week) => Some(week.into()),
                None => None,
            },
            subjects: value
                .subjects
                .ordered_subject_list
                .into_iter()
                .map(|(subject_id, subject)| Subject {
                    id: subject_id.into(),
                    parameters: subject.parameters.into(),
                    excluded_periods: subject
                        .excluded_periods
                        .into_iter()
                        .map(|period_id| period_id.into())
                        .collect(),
                })
                .collect(),
            teachers: value
                .teachers
                .teacher_map
                .into_iter()
                .map(|(teacher_id, teacher)| (teacher_id.into(), teacher.into()))
                .collect(),
            students: value
                .students
                .student_map
                .into_iter()
                .map(|(student_id, student)| (student_id.into(), student.into()))
                .collect(),
            assignments: value
                .assignments
                .period_map
                .into_iter()
                .map(|(period_id, period_assignements)| {
                    (
                        period_id.into(),
                        period_assignements
                            .subject_map
                            .into_iter()
                            .map(|(subject_id, students)| {
                                (
                                    subject_id.into(),
                                    students.into_iter().map(|id| id.into()).collect(),
                                )
                            })
                            .collect(),
                    )
                })
                .collect(),
            week_patterns: value
                .week_patterns
                .week_pattern_map
                .into_iter()
                .map(|(week_pattern_id, week_pattern)| {
                    (
                        week_pattern_id.into(),
                        WeekPattern {
                            name: week_pattern.name,
                            weeks: week_pattern.weeks,
                        },
                    )
                })
                .collect(),
            slots: value
                .slots
                .subject_map
                .into_iter()
                .map(|(subject_id, subject_slots)| {
                    (
                        subject_id.into(),
                        subject_slots
                            .ordered_slots
                            .into_iter()
                            .map(|(slot_id, slot)| slots::Slot {
                                id: slot_id.into(),
                                parameters: slot.into(),
                            })
                            .collect(),
                    )
                })
                .collect(),
            incompats: value
                .incompats
                .incompat_map
                .into_iter()
                .map(|(incompat_id, incompat)| (incompat_id.into(), incompat.into()))
                .collect(),
            group_lists: value
                .group_lists
                .group_list_map
                .into_iter()
                .map(|(group_list_id, group_list)| (group_list_id.into(), group_list.into()))
                .collect(),
            group_lists_associations: value
                .group_lists
                .subjects_associations
                .into_iter()
                .map(|(period_id, subject_map)| {
                    (
                        period_id.into(),
                        subject_map
                            .into_iter()
                            .map(|(subject_id, group_list_id)| {
                                (subject_id.into(), group_list_id.into())
                            })
                            .collect(),
                    )
                })
                .collect(),
            rules: value
                .rules
                .rule_map
                .into_iter()
                .map(|(rule_id, rule)| {
                    PyResult::Ok((
                        RuleId::from(rule_id),
                        rules::Rule {
                            name: rule.name,
                            logic_rule: rule.desc.try_into()?,
                            excluded_periods: rule
                                .excluded_periods
                                .into_iter()
                                .map(|x| x.into())
                                .collect(),
                        },
                    ))
                })
                .collect::<Result<_, _>>()?,
            settings: value.settings.into(),
        })
    }
}

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeParameters {
    #[pyo3(get)]
    pub periods: Vec<ColloscopePeriod>,
    #[pyo3(get)]
    pub periods_first_week: Option<time::NaiveMondayDate>,
    #[pyo3(get)]
    pub subjects: Vec<subjects::ColloscopeSubject>,
    #[pyo3(get)]
    pub teachers: BTreeMap<ColloscopeTeacherId, ColloscopeTeacher>,
    #[pyo3(get)]
    pub students: BTreeMap<ColloscopeStudentId, ColloscopeStudent>,
    #[pyo3(get)]
    pub assignments:
        BTreeMap<ColloscopePeriodId, BTreeMap<ColloscopeSubjectId, BTreeSet<ColloscopeStudentId>>>,
    #[pyo3(get)]
    pub week_patterns: BTreeMap<ColloscopeWeekPatternId, WeekPattern>,
    #[pyo3(get)]
    pub slots: BTreeMap<subjects::ColloscopeSubjectId, Vec<slots::ColloscopeSlot>>,
    #[pyo3(get)]
    pub incompats:
        BTreeMap<incompatibilities::ColloscopeIncompatId, incompatibilities::ColloscopeIncompat>,
    #[pyo3(get)]
    pub group_lists: BTreeMap<group_lists::ColloscopeGroupListId, group_lists::ColloscopeGroupList>,
    #[pyo3(get)]
    pub group_lists_associations: BTreeMap<
        ColloscopePeriodId,
        BTreeMap<ColloscopeSubjectId, group_lists::ColloscopeGroupListId>,
    >,
    #[pyo3(get)]
    pub rules: BTreeMap<rules::ColloscopeRuleId, rules::ColloscopeRule>,
    #[pyo3(get)]
    pub settings: settings::GeneralSettings,
}

impl TryFrom<collomatique_state_colloscopes::colloscope_params::ColloscopeParameters>
    for ColloscopeParameters
{
    type Error = PyErr;
    fn try_from(
        value: collomatique_state_colloscopes::colloscope_params::ColloscopeParameters,
    ) -> PyResult<Self> {
        Ok(ColloscopeParameters {
            periods: value
                .periods
                .ordered_period_list
                .into_iter()
                .map(|(period_id, weeks_status)| ColloscopePeriod {
                    id: period_id.into(),
                    weeks_status,
                })
                .collect(),
            periods_first_week: match value.periods.first_week {
                Some(week) => Some(week.into()),
                None => None,
            },
            subjects: value
                .subjects
                .ordered_subject_list
                .into_iter()
                .map(|(subject_id, subject)| ColloscopeSubject {
                    id: subject_id.into(),
                    parameters: subject.parameters.into(),
                    excluded_periods: subject
                        .excluded_periods
                        .into_iter()
                        .map(|period_id| period_id.into())
                        .collect(),
                })
                .collect(),
            teachers: value
                .teachers
                .teacher_map
                .into_iter()
                .map(|(teacher_id, teacher)| (teacher_id.into(), teacher.into()))
                .collect(),
            students: value
                .students
                .student_map
                .into_iter()
                .map(|(student_id, student)| (student_id.into(), student.into()))
                .collect(),
            assignments: value
                .assignments
                .period_map
                .into_iter()
                .map(|(period_id, period_assignements)| {
                    (
                        period_id.into(),
                        period_assignements
                            .subject_map
                            .into_iter()
                            .map(|(subject_id, students)| {
                                (
                                    subject_id.into(),
                                    students.into_iter().map(|id| id.into()).collect(),
                                )
                            })
                            .collect(),
                    )
                })
                .collect(),
            week_patterns: value
                .week_patterns
                .week_pattern_map
                .into_iter()
                .map(|(week_pattern_id, week_pattern)| {
                    (
                        week_pattern_id.into(),
                        WeekPattern {
                            name: week_pattern.name,
                            weeks: week_pattern.weeks,
                        },
                    )
                })
                .collect(),
            slots: value
                .slots
                .subject_map
                .into_iter()
                .map(|(subject_id, subject_slots)| {
                    (
                        subject_id.into(),
                        subject_slots
                            .ordered_slots
                            .into_iter()
                            .map(|(slot_id, slot)| slots::ColloscopeSlot {
                                id: slot_id.into(),
                                parameters: slot.into(),
                            })
                            .collect(),
                    )
                })
                .collect(),
            incompats: value
                .incompats
                .incompat_map
                .into_iter()
                .map(|(incompat_id, incompat)| (incompat_id.into(), incompat.into()))
                .collect(),
            group_lists: value
                .group_lists
                .group_list_map
                .into_iter()
                .map(|(group_list_id, group_list)| (group_list_id.into(), group_list.into()))
                .collect(),
            group_lists_associations: value
                .group_lists
                .subjects_associations
                .into_iter()
                .map(|(period_id, subject_map)| {
                    (
                        period_id.into(),
                        subject_map
                            .into_iter()
                            .map(|(subject_id, group_list_id)| {
                                (subject_id.into(), group_list_id.into())
                            })
                            .collect(),
                    )
                })
                .collect(),
            rules: value
                .rules
                .rule_map
                .into_iter()
                .map(|(rule_id, rule)| {
                    PyResult::Ok((
                        ColloscopeRuleId::from(rule_id),
                        rules::ColloscopeRule {
                            name: rule.name,
                            logic_rule: rule.desc.try_into()?,
                            excluded_periods: rule
                                .excluded_periods
                                .into_iter()
                                .map(|x| x.into())
                                .collect(),
                        },
                    ))
                })
                .collect::<Result<_, _>>()?,
            settings: value.settings.into(),
        })
    }
}

impl TryFrom<ColloscopeParameters>
    for collomatique_state_colloscopes::colloscope_params::ColloscopeParameters
{
    type Error = PyErr;
    fn try_from(
        value: ColloscopeParameters,
    ) -> PyResult<collomatique_state_colloscopes::colloscope_params::ColloscopeParameters> {
        Ok(
            collomatique_state_colloscopes::colloscope_params::ColloscopeParameters {
                periods: collomatique_state_colloscopes::periods::Periods {
                    first_week: match value.periods_first_week {
                        Some(week) => Some(week.into()),
                        None => None,
                    },
                    ordered_period_list: value
                        .periods
                        .into_iter()
                        .map(|period| (period.id.into(), period.weeks_status))
                        .collect(),
                },
                subjects: collomatique_state_colloscopes::subjects::Subjects {
                    ordered_subject_list: value
                        .subjects
                        .into_iter()
                        .map(|subject| {
                            (
                                subject.id.into(),
                                collomatique_state_colloscopes::subjects::Subject {
                                    parameters: subject.parameters.into(),
                                    excluded_periods: subject
                                        .excluded_periods
                                        .into_iter()
                                        .map(|period_id| period_id.into())
                                        .collect(),
                                },
                            )
                        })
                        .collect(),
                },
                teachers: collomatique_state_colloscopes::teachers::Teachers {
                    teacher_map: value
                        .teachers
                        .into_iter()
                        .map(|(teacher_id, teacher)| (teacher_id.into(), teacher.into()))
                        .collect(),
                },
                students: collomatique_state_colloscopes::students::Students {
                    student_map: value
                        .students
                        .into_iter()
                        .map(|(student_id, student)| (student_id.into(), student.into()))
                        .collect(),
                },
                assignments: collomatique_state_colloscopes::assignments::Assignments {
                    period_map: value
                        .assignments
                        .into_iter()
                        .map(|(period_id, period_assignements)| {
                            (
                                period_id.into(),
                                collomatique_state_colloscopes::assignments::PeriodAssignments {
                                    subject_map: period_assignements
                                        .into_iter()
                                        .map(|(subject_id, students)| {
                                            (
                                                subject_id.into(),
                                                students.into_iter().map(|id| id.into()).collect(),
                                            )
                                        })
                                        .collect(),
                                },
                            )
                        })
                        .collect(),
                },
                week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns {
                    week_pattern_map: value
                        .week_patterns
                        .into_iter()
                        .map(|(week_pattern_id, week_pattern)| {
                            (
                                week_pattern_id.into(),
                                collomatique_state_colloscopes::week_patterns::WeekPattern {
                                    name: week_pattern.name,
                                    weeks: week_pattern.weeks,
                                },
                            )
                        })
                        .collect(),
                },
                slots: collomatique_state_colloscopes::slots::Slots {
                    subject_map: value
                        .slots
                        .into_iter()
                        .map(|(subject_id, subject_slots)| {
                            (
                                subject_id.into(),
                                collomatique_state_colloscopes::slots::SubjectSlots {
                                    ordered_slots: subject_slots
                                        .into_iter()
                                        .map(|slot| (slot.id.into(), slot.parameters.into()))
                                        .collect(),
                                },
                            )
                        })
                        .collect(),
                },
                incompats: collomatique_state_colloscopes::incompats::Incompats {
                    incompat_map: value
                        .incompats
                        .into_iter()
                        .map(|(incompat_id, incompat)| {
                            PyResult::Ok((
                                incompat_id.into(),
                                incompat.try_into().map_err(|e| match e {
                                    SlotWithDurationError::SlotOverlapsWithNextDay => {
                                        PyValueError::new_err("Slot overlaps with next day")
                                    }
                                })?,
                            ))
                        })
                        .collect::<PyResult<_>>()?,
                },
                group_lists: collomatique_state_colloscopes::group_lists::GroupLists {
                    group_list_map: value
                        .group_lists
                        .into_iter()
                        .map(|(group_list_id, group_list)| {
                            (group_list_id.into(), group_list.into())
                        })
                        .collect(),
                    subjects_associations: value
                        .group_lists_associations
                        .into_iter()
                        .map(|(period_id, subject_map)| {
                            (
                                period_id.into(),
                                subject_map
                                    .into_iter()
                                    .map(|(subject_id, group_list_id)| {
                                        (subject_id.into(), group_list_id.into())
                                    })
                                    .collect(),
                            )
                        })
                        .collect(),
                },
                rules: collomatique_state_colloscopes::rules::Rules {
                    rule_map: value
                        .rules
                        .into_iter()
                        .map(|(rule_id, rule)| {
                            (
                                rule_id.into(),
                                collomatique_state_colloscopes::rules::Rule {
                                    name: rule.name,
                                    desc: rule.logic_rule.into(),
                                    excluded_periods: rule
                                        .excluded_periods
                                        .into_iter()
                                        .map(|x| x.into())
                                        .collect(),
                                },
                            )
                        })
                        .collect(),
                },
                settings: collomatique_state_colloscopes::settings::GeneralSettings {
                    strict_limits: value.settings.strict_limits.into(),
                },
            },
        )
    }
}
