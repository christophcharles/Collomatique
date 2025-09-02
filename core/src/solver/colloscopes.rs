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
    #[error("A group list chosen for a subject does not contain all the students for the subject")]
    GroupListDoesNotContainAllStudents,
}

type MainVar = collomatique_solver_colloscopes::base::variables::MainVariable<
    GroupListId,
    StudentId,
    SubjectId,
    SlotId,
>;
type StructVar = collomatique_solver_colloscopes::base::variables::StructureVariable<
    GroupListId,
    StudentId,
    SubjectId,
    SlotId,
>;
type BaseProblem = collomatique_solver_colloscopes::base::ValidatedColloscopeProblem<
    SubjectId,
    SlotId,
    GroupListId,
    StudentId,
>;
type ProblemRepr = collomatique_ilp::DefaultRepr<
    collomatique_solver::ExtraVariable<MainVar, StructVar, collomatique_solver::solver::IdVariable>,
>;

pub struct ColloscopeProblemWithTranslators {
    pub problem: collomatique_solver::Problem<MainVar, StructVar, BaseProblem, ProblemRepr>,
}

impl ColloscopeProblemWithTranslators {
    pub fn from_data(data: &Data) -> Result<Self, Error> {
        let problem_desc = data_to_colloscope_problem_desc(data)?;

        use collomatique_solver_colloscopes::base::ValidationError;
        let validated_problem_desc = match problem_desc.validate() {
            Ok(v) => v,
            Err(ValidationError::EmptyGroupCountRange) => panic!("Unexpected empty group range count - this should be forbidden by data invariants"),
            Err(ValidationError::EmptyStudentPerGroupRange) => panic!("Unexpected empty students per group range count - this should be forbidden by data invariants"),
            Err(ValidationError::GroupListDoesNotContainAllStudents) => return Err(Error::GroupListDoesNotContainAllStudents),
            Err(ValidationError::InconsistentWeekCount) => panic!("Unexpected inconsistent week count - this should be satisfied by the output of data_to_colloscope_problem_desc"),
            Err(ValidationError::InconsistentWeekStatusInSlot) => panic!("Unexpected inconsistent week status in a slot - this should be satisfied by the output of data_to_colloscope_problem_desc"),
            Err(ValidationError::InvalidGroupListId) => panic!("Unexpected invalid group_list_id for subject - this should be satisfied by the output of data_to_colloscope_problem_desc"),
        };

        println!("A");
        let problem_builder =
            collomatique_solver::ProblemBuilder::<_, _, _>::new(validated_problem_desc)
                .expect("Consistent ILP description");

        println!("B");
        let problem = problem_builder.build();

        Ok(ColloscopeProblemWithTranslators { problem })
    }
}

fn data_to_colloscope_problem_desc(data: &Data) -> Result<ProblemDesc, Error> {
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

        let mut group_assignments = vec![];

        for (period_id, period) in &data.get_periods().ordered_period_list {
            if subject.excluded_periods.contains(period_id) {
                group_assignments.extend(vec![None; period.len()]);
                continue;
            }

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

            let group_assignment = Some(base::GroupAssignment {
                group_list_id,
                enrolled_students,
            });

            for week in period {
                group_assignments.push(if *week {
                    group_assignment.clone()
                } else {
                    None
                });
            }
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
