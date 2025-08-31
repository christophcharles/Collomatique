//! Colloscope Solver module
//!
//! This module contains the translation code
//! from [collomatique_state_colloscopes] to [collomatique_solver_colloscopes].

use std::collections::{BTreeMap, BTreeSet};

use collomatique_solver_colloscopes::base::{self, ColloscopeProblem};
use collomatique_state_colloscopes::{Data, GroupListId, PeriodId, SlotId, StudentId, SubjectId};

type ProblemDesc = ColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>;

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Error {
    /// A group list is needed for every period for every subject
    #[error("subject {0:?} does not have an associated group list for period {1:?}")]
    MissingGroupList(SubjectId, PeriodId),
}

pub fn data_to_colloscope_problem_desc(data: &Data) -> Result<ProblemDesc, Error> {
    let students: BTreeSet<_> = data.get_students().student_map.keys().copied().collect();

    let mut subject_descriptions = BTreeMap::new();

    for (subject_id, subject) in &data.get_subjects().ordered_subject_list {
        let Some(params) = &subject.parameters.interrogation_parameters else {
            continue;
        };

        let mut slots_descriptions = BTreeMap::new();

        let slot_list = data
            .get_slots()
            .subject_map
            .get(subject_id)
            .expect("Subject should have slots as it has interrogations");

        for (slot_id, slot) in &slot_list.ordered_slots {
            let mut weeks = vec![];

            let week_pattern_opt = slot.week_pattern.map(|id| {
                data.get_week_patterns()
                    .week_pattern_map
                    .get(&id)
                    .expect("Week pattern id should be valid")
            });

            for (period_id, period) in &data.get_periods().ordered_period_list {
                if subject.excluded_periods.contains(period_id) {
                    weeks.extend(vec![false; period.len()]);
                    continue;
                }

                for week in period {
                    if !*week {
                        weeks.push(false);
                        continue;
                    }

                    let Some(week_pattern) = week_pattern_opt else {
                        weeks.push(true);
                        continue;
                    };

                    if week_pattern.weeks.len() <= weeks.len() {
                        weeks.push(true);
                        continue;
                    }

                    weeks.push(week_pattern.weeks[weeks.len()]);
                }
            }

            let slot_desc = base::SlotDescription {
                slot_start: slot.start_time.clone(),
                weeks,
            };

            slots_descriptions.insert(slot_id.clone(), slot_desc);
        }

        let mut group_assignments = base::GroupAssignments {
            starting_group_assignment: None,
            other_group_assignments: vec![],
        };

        let mut start_week = 0usize;
        for (i, (period_id, period)) in data.get_periods().ordered_period_list.iter().enumerate() {
            let group_assignment = if subject.excluded_periods.contains(period_id) {
                None
            } else {
                let group_list_id = data
                    .get_group_lists()
                    .subjects_associations
                    .get(period_id)
                    .expect("Period id should be valid")
                    .get(subject_id)
                    .ok_or(Error::MissingGroupList(*subject_id, *period_id))?
                    .clone();

                let enrolled_students = data
                    .get_assignments()
                    .period_map
                    .get(period_id)
                    .expect("Period ID should be valid")
                    .subject_map
                    .get(subject_id)
                    .expect("Subject ID should have assignments for this period")
                    .clone();

                Some(base::GroupAssignment {
                    group_list_id,
                    enrolled_students,
                })
            };

            if i == 0 {
                group_assignments.starting_group_assignment = group_assignment;
            } else {
                group_assignments
                    .other_group_assignments
                    .push(base::DatedGroupAssignment {
                        start_week,
                        group_assignment,
                    })
            }

            start_week += period.len();
        }

        let subject_desc = base::SubjectDescription {
            duration: params.duration.clone(),
            students_per_group: params.students_per_group.clone(),
            groups_per_interrogation: params.groups_per_interrogation.clone(),
            slots_descriptions,
            group_assignments,
        };

        subject_descriptions.insert(subject_id.clone(), subject_desc);
    }

    Ok(ProblemDesc {
        group_list_descriptions: data
            .get_group_lists()
            .group_list_map
            .iter()
            .map(|(group_list_id, group_list)| {
                (
                    group_list_id.clone(),
                    base::GroupListDescription {
                        students_per_group: group_list.params.students_per_group.clone(),
                        group_count: group_list.params.group_count.clone(),
                        students: students
                            .difference(&group_list.params.excluded_students)
                            .copied()
                            .collect(),
                    },
                )
            })
            .collect(),
        subject_descriptions,
    })
}
