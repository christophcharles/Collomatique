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
        // The assignments and associations junction tables are keyed by
        // `(period, subject)`, but the Python-visible shape is a per-period
        // nested map with one (possibly empty) entry per period. Seed the
        // outer maps from the full period list to keep that shape.
        let all_period_ids: Vec<collomatique_state_colloscopes::PeriodId> =
            value.periods.period_ids().collect();
        // The sparse core only stores non-empty assignment rows, but the
        // Python-visible shape is dense (one entry per period × non-excluded
        // subject). Snapshot each subject's excluded-period set so the dense
        // skeleton can be seeded below, before `value.subjects` is consumed.
        let subject_excluded: Vec<(
            collomatique_state_colloscopes::SubjectId,
            std::collections::BTreeSet<collomatique_state_colloscopes::PeriodId>,
        )> = value
            .subjects
            .ordered_subject_list
            .iter()
            .map(|(subject_id, subject)| (subject_id, subject.excluded_periods.clone()))
            .collect();
        // The sparse slots ordering only stores subjects that have slots, but
        // the Python-visible shape is dense (one entry per subject with
        // interrogations, empty vector when it has no slots yet). Snapshot the
        // interrogation subjects before `value.subjects` is consumed.
        let interrogation_subject_ids: Vec<collomatique_state_colloscopes::SubjectId> = value
            .subjects
            .ordered_subject_list
            .iter()
            .filter(|(_, subject)| subject.parameters.interrogation_parameters.is_some())
            .map(|(subject_id, _)| subject_id)
            .collect();
        Ok(Parameters {
            periods: value
                .periods
                .period_ids()
                .map(|period_id| Period {
                    id: period_id.into(),
                    weeks_status: value
                        .periods
                        .weeks_vec_of(period_id)
                        .expect("period id from period_ids is valid")
                        .into_iter()
                        .map(|x| x.into())
                        .collect(),
                })
                .collect(),
            periods_first_week: value.periods.first_week.map(|week| week.into()),
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
            assignments: {
                let mut out: BTreeMap<PeriodId, BTreeMap<SubjectId, BTreeSet<StudentId>>> =
                    all_period_ids
                        .iter()
                        .map(|id| ((*id).into(), BTreeMap::new()))
                        .collect();
                // Seed the dense skeleton: one (possibly empty) entry per
                // non-excluded subject on each period.
                for (subject_id, excluded) in &subject_excluded {
                    for period_id in &all_period_ids {
                        if excluded.contains(period_id) {
                            continue;
                        }
                        out.entry((*period_id).into())
                            .or_default()
                            .insert((*subject_id).into(), BTreeSet::new());
                    }
                }
                // Overwrite with the actual (non-empty) rows from the core.
                for ((period_id, subject_id), students) in value.assignments.map {
                    out.entry(period_id.into()).or_default().insert(
                        subject_id.into(),
                        students.into_iter().map(|id| id.into()).collect(),
                    );
                }
                out
            },
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
            slots: interrogation_subject_ids
                .iter()
                .map(|&subject_id| {
                    // Sparse ordering: no row for a subject without slots, so
                    // fall back to an empty list to keep the dense shape.
                    let subject_slots = value
                        .slots
                        .slots_vec_for_subject(subject_id)
                        .unwrap_or_default();
                    (
                        subject_id.into(),
                        subject_slots
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
            group_lists_associations: {
                let mut out: BTreeMap<PeriodId, BTreeMap<SubjectId, group_lists::GroupListId>> =
                    all_period_ids
                        .iter()
                        .map(|id| ((*id).into(), BTreeMap::new()))
                        .collect();
                for ((period_id, subject_id), group_list_id) in
                    value.group_lists.subjects_associations
                {
                    out.entry(period_id.into())
                        .or_default()
                        .insert(subject_id.into(), group_list_id.into());
                }
                out
            },
            settings: value.settings.into(),
        })
    }
}
