use super::*;

use std::collections::BTreeSet;

#[pyclass]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parameters {
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
    pub settings: settings::Settings,
}

#[pymethods]
impl Parameters {
    fn get_week_count(self_: PyRef<'_, Self>) -> usize {
        self_.periods.iter().map(|x| x.weeks_status.len()).sum()
    }
}

impl TryFrom<collomatique_state_colloscopes::colloscope_params::Parameters> for Parameters {
    type Error = PyErr;
    fn try_from(
        value: collomatique_state_colloscopes::colloscope_params::Parameters,
    ) -> PyResult<Self> {
        Ok(Parameters {
            periods: value
                .periods
                .ordered_period_list
                .into_iter()
                .map(|(period_id, weeks_status)| Period {
                    id: period_id.into(),
                    weeks_status: weeks_status.into_iter().map(|x| x.into()).collect(),
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
