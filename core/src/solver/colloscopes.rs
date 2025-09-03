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
    #[error("Some students enrolled in subjects {0:?} do not appear in group list {1:?}")]
    GroupListDoesNotContainAllStudents(SubjectId, GroupListId),
    #[error(
        "Prefilled group {1} exceeds the maximum number of students per group (group list {0:?})"
    )]
    TooManyStudentsInPrefilledGroup(GroupListId, u32),
    #[error("Sealed group {1} does not have enough students (group list {0:?})")]
    TooFewStudentsInSealedGroup(GroupListId, u32),
    #[error("Prefilled group {2} exceeds the maximum number of students per group when specialized for subject {0:?} on week {1}")]
    TooManyStudentsInPrefilledGroupForSubject(SubjectId, usize, u32),
    #[error("Sealed group {2} does not have enough students when specialized for subject {0:?} on week {1}")]
    TooFewStudentsInSealedGroupForSubject(SubjectId, usize, u32),
    #[error("Group list {0:?} has a maximum number of groups of {1} but has {2} prefilled groups")]
    TooManyPrefilledGroups(GroupListId, u32, usize),
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

pub enum ColloscopeTranslator {
    GroupsPerSlot(
        collomatique_solver::Translator<
            collomatique_solver_colloscopes::constraints::groups_per_slots::GroupsPerSlots<
                SubjectId,
                SlotId,
                GroupListId,
                StudentId,
            >,
        >,
    ),
    StudentsPerGroups(
        collomatique_solver::Translator<
            collomatique_solver_colloscopes::constraints::students_per_groups::StudentsPerGroups<
                SubjectId,
                SlotId,
                GroupListId,
                StudentId,
            >,
        >,
    ),
    GroupCount(
        collomatique_solver::Translator<
            collomatique_solver_colloscopes::constraints::group_count::GroupCount<
                SubjectId,
                SlotId,
                GroupListId,
                StudentId,
            >,
        >,
    ),
    SealedGroups(
        collomatique_solver::Translator<
            collomatique_solver_colloscopes::constraints::sealed_groups::SealedGroups<
                SubjectId,
                SlotId,
                GroupListId,
                StudentId,
            >,
        >,
    ),
    StudentsPerGroupsForSubject(
        collomatique_solver::Translator<
            collomatique_solver_colloscopes::constraints::students_per_groups_for_subject::StudentsPerGroupsForSubject<
                SubjectId,
                SlotId,
                GroupListId,
                StudentId,
            >,
        >,
    ),
    StrictLimits(
        collomatique_solver::Translator<
            collomatique_solver_colloscopes::constraints::strict_limits::StrictLimits<
                SubjectId,
                SlotId,
                GroupListId,
                StudentId,
            >,
        >,
    ),
    OneInterrogationAtATime(
        collomatique_solver::Translator<
            collomatique_solver_colloscopes::constraints::one_interrogation_at_a_time::OneInterrogationAtATime<
                SubjectId,
                SlotId,
                GroupListId,
                StudentId,
            >,
        >,
    ),
}

pub struct ColloscopeProblemWithTranslators {
    pub problem: collomatique_solver::Problem<MainVar, StructVar, BaseProblem, ProblemRepr>,
    pub translators: Vec<ColloscopeTranslator>,
}

impl ColloscopeProblemWithTranslators {
    pub fn from_data(data: &Data) -> Result<Self, Error> {
        let problem_desc = data_to_colloscope_problem_desc(data)?;

        use collomatique_solver_colloscopes::base::ValidationError;
        let validated_problem_desc = match problem_desc.validate() {
            Ok(v) => v,
            Err(ValidationError::EmptyGroupCountRange(id)) => panic!("Unexpected empty group range count (group list {:?}) - this should be forbidden by data invariants", id),
            Err(ValidationError::EmptyStudentPerGroupRange(id)) => panic!("Unexpected empty students per group range count (group list {:?}) - this should be forbidden by data invariants", id),
            Err(ValidationError::GroupListDoesNotContainAllStudents(subject_id, group_list_id)) => return Err(Error::GroupListDoesNotContainAllStudents(subject_id, group_list_id)),
            Err(ValidationError::InconsistentWeekCount) => panic!("Unexpected inconsistent week count - this should be satisfied by the output of data_to_colloscope_problem_desc"),
            Err(ValidationError::InconsistentWeekStatusInSlot(week, id)) => panic!("Unexpected inconsistent week status for week {} in a slot {:?} - this should be satisfied by the output of data_to_colloscope_problem_desc", week, id),
            Err(ValidationError::InvalidGroupListId(group_list_id,week,subject_id)) => panic!("Unexpected invalid group_list_id {:?} for subject {:?} on week {} - this should be satisfied by the output of data_to_colloscope_problem_desc", group_list_id, subject_id, week),
            Err(ValidationError::DuplicateStudentInGroupList(id)) => panic!("Unexpected duplicated students in group list {:?} - this should be satisfied by the output of data_to_colloscope_problem_desc", id),
            Err(ValidationError::GroupCountTooBigForU32(id)) => panic!("Group count exceeds u32 capacity in group list {:?}. If this is intentional, the panic is earned...", id),
            Err(ValidationError::TooManyStudentsInPrefilledGroup(id, group)) => return Err(Error::TooManyStudentsInPrefilledGroup(id, group)),
            Err(ValidationError::TooManyStudentsInPrefilledGroupForSubject(id, week, group)) => return Err(Error::TooManyStudentsInPrefilledGroupForSubject(id, week, group)),
            Err(ValidationError::TooFewStudentsInSealedGroup(id, group)) => return Err(Error::TooFewStudentsInSealedGroup(id, group)),
            Err(ValidationError::TooFewStudentsInSealedGroupForSubject(id, week, group)) => return Err(Error::TooFewStudentsInSealedGroupForSubject(id, week, group)),
            Err(ValidationError::EmptyStudentPerGroupRangeForSubject(id)) => panic!("Unexpected empty students per group range count (subject {:?}) - this should be forbidden by data invariants", id),
            Err(ValidationError::SlotIncompatibleWithItsDuration(subject_id, slot_id)) => panic!("Unexpected incompatibility between slot and its duration (slot {:?} of subject {:?}) - this should be forbidden by data invariants", slot_id, subject_id),
        };

        let mut problem_builder =
            collomatique_solver::ProblemBuilder::<_, _, _>::new(validated_problem_desc)
                .expect("Consistent ILP description");

        let mut translators = vec![];

        let weeks = generate_active_weeks_list(data);

        add_groups_per_slots_constraints(&mut problem_builder, &mut translators, data, &weeks);
        add_students_per_groups_constraints(&mut problem_builder, &mut translators, data);
        add_group_count_constraints(&mut problem_builder, &mut translators, data);
        add_sealed_groups_constraints(&mut problem_builder, &mut translators, data);
        add_students_per_groups_for_subject_constraints(
            &mut problem_builder,
            &mut translators,
            data,
        );
        add_strict_limits_constraints(&mut problem_builder, &mut translators, data, &weeks);
        add_one_interrogation_at_a_time_constraints(
            &mut problem_builder,
            &mut translators,
            data,
            &weeks,
        );

        let problem = problem_builder.build();

        Ok(ColloscopeProblemWithTranslators {
            problem,
            translators,
        })
    }
}

fn generate_active_weeks_list(data: &Data) -> Vec<bool> {
    let mut weeks = vec![];
    for (_period_id, period) in &data.get_periods().ordered_period_list {
        weeks.extend(period.into_iter().copied());
    }
    weeks
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
            take_duration_into_account: params.take_duration_into_account,
            students_per_group: params.students_per_group.clone(),
            groups_per_interrogation: params.groups_per_interrogation.clone(),
            slots_descriptions,
            group_assignments,
        };

        subject_descriptions.insert(subject_id.clone(), subject_desc);
    }

    let mut group_list_descriptions = BTreeMap::new();

    for (group_list_id, group_list) in &data.get_group_lists().group_list_map {
        let mut prefilled_groups = vec![
            base::PrefilledGroup {
                students: BTreeSet::new(),
                sealed: false,
            };
            *group_list.params.group_count.end() as usize
        ];

        if group_list.prefilled_groups.groups.len() > prefilled_groups.len() {
            return Err(Error::TooManyPrefilledGroups(
                *group_list_id,
                *group_list.params.group_count.end(),
                group_list.prefilled_groups.groups.len(),
            ));
        }

        for (i, prefilled_group) in group_list.prefilled_groups.groups.iter().enumerate() {
            prefilled_groups[i].sealed = prefilled_group.sealed;
            prefilled_groups[i].students = prefilled_group.students.clone();
        }

        let remaining_students = group_list.remaining_students_to_dispatch(&students);

        group_list_descriptions.insert(
            group_list_id.clone(),
            base::GroupListDescription {
                students_per_group: group_list.params.students_per_group.clone(),
                minimum_group_count: *group_list.params.group_count.start(),
                prefilled_groups,
                remaining_students,
            },
        );
    }

    Ok(ProblemDesc {
        group_list_descriptions,
        subject_descriptions,
    })
}

fn add_groups_per_slots_constraints(
    problem_builder: &mut collomatique_solver::ProblemBuilder<MainVar, StructVar, BaseProblem>,
    translators: &mut Vec<ColloscopeTranslator>,
    data: &Data,
    weeks: &Vec<bool>,
) {
    for (subject_id, subject) in &data.get_subjects().ordered_subject_list {
        if subject.parameters.interrogation_parameters.is_none() {
            continue;
        }
        let groups_per_slots_constraints =
            collomatique_solver_colloscopes::constraints::groups_per_slots::GroupsPerSlots::new(
                *subject_id,
                weeks.clone(),
            );
        translators.push(ColloscopeTranslator::GroupsPerSlot(
            problem_builder
                .add_constraints(groups_per_slots_constraints, 0.)
                .expect("Translator should be compatible with problem"),
        ));
    }
}

fn add_students_per_groups_constraints(
    problem_builder: &mut collomatique_solver::ProblemBuilder<MainVar, StructVar, BaseProblem>,
    translators: &mut Vec<ColloscopeTranslator>,
    data: &Data,
) {
    for (group_list_id, _group_list) in &data.get_group_lists().group_list_map {
        let students_per_groups_constraints =
            collomatique_solver_colloscopes::constraints::students_per_groups::StudentsPerGroups::new(
                *group_list_id,
            );
        translators.push(ColloscopeTranslator::StudentsPerGroups(
            problem_builder
                .add_constraints(students_per_groups_constraints, 0.)
                .expect("Translator should be compatible with problem"),
        ));
    }
}

fn add_group_count_constraints(
    problem_builder: &mut collomatique_solver::ProblemBuilder<MainVar, StructVar, BaseProblem>,
    translators: &mut Vec<ColloscopeTranslator>,
    data: &Data,
) {
    for (group_list_id, _group_list) in &data.get_group_lists().group_list_map {
        let group_count_constraints =
            collomatique_solver_colloscopes::constraints::group_count::GroupCount::new(
                *group_list_id,
            );
        translators.push(ColloscopeTranslator::GroupCount(
            problem_builder
                .add_constraints(group_count_constraints, 0.)
                .expect("Translator should be compatible with problem"),
        ));
    }
}

fn add_sealed_groups_constraints(
    problem_builder: &mut collomatique_solver::ProblemBuilder<MainVar, StructVar, BaseProblem>,
    translators: &mut Vec<ColloscopeTranslator>,
    data: &Data,
) {
    for (group_list_id, _group_list) in &data.get_group_lists().group_list_map {
        let sealed_groups_constraints =
            collomatique_solver_colloscopes::constraints::sealed_groups::SealedGroups::new(
                *group_list_id,
            );
        translators.push(ColloscopeTranslator::SealedGroups(
            problem_builder
                .add_constraints(sealed_groups_constraints, 0.)
                .expect("Translator should be compatible with problem"),
        ));
    }
}

fn add_students_per_groups_for_subject_constraints(
    problem_builder: &mut collomatique_solver::ProblemBuilder<MainVar, StructVar, BaseProblem>,
    translators: &mut Vec<ColloscopeTranslator>,
    data: &Data,
) {
    let mut params = BTreeSet::new();
    for (_period_id, subject_map) in &data.get_group_lists().subjects_associations {
        for (subject_id, group_list_id) in subject_map {
            params.insert((*subject_id, *group_list_id));
        }
    }

    for (subject_id, group_list_id) in params {
        let students_per_groups_for_subject_constraints =
            collomatique_solver_colloscopes::constraints::students_per_groups_for_subject::StudentsPerGroupsForSubject::new(
                group_list_id,
                subject_id,
            );
        translators.push(ColloscopeTranslator::StudentsPerGroupsForSubject(
            problem_builder
                .add_constraints(students_per_groups_for_subject_constraints, 0.)
                .expect("Translator should be compatible with problem"),
        ));
    }
}

fn add_strict_limits_constraints(
    problem_builder: &mut collomatique_solver::ProblemBuilder<MainVar, StructVar, BaseProblem>,
    translators: &mut Vec<ColloscopeTranslator>,
    data: &Data,
    weeks: &Vec<bool>,
) {
    let students = data.get_students().student_map.keys().copied().collect();
    let settings = data.get_settings();

    let strict_limits_constraints =
        collomatique_solver_colloscopes::constraints::strict_limits::StrictLimits::new(
            students,
            weeks.clone(),
            settings.strict_limits.interrogations_per_week.clone(),
            settings.strict_limits.max_interrogations_per_day.clone(),
        );
    translators.push(ColloscopeTranslator::StrictLimits(
        problem_builder
            .add_constraints(strict_limits_constraints, 0.)
            .expect("Translator should be compatible with problem"),
    ));
}

fn add_one_interrogation_at_a_time_constraints(
    problem_builder: &mut collomatique_solver::ProblemBuilder<MainVar, StructVar, BaseProblem>,
    translators: &mut Vec<ColloscopeTranslator>,
    data: &Data,
    weeks: &Vec<bool>,
) {
    let students = data.get_students().student_map.keys().copied().collect();

    let one_interrogation_at_a_time_constraints =
        collomatique_solver_colloscopes::constraints::one_interrogation_at_a_time::OneInterrogationAtATime::new(
            students,
            weeks.clone(),
        );
    translators.push(ColloscopeTranslator::OneInterrogationAtATime(
        problem_builder
            .add_constraints(one_interrogation_at_a_time_constraints, 0.)
            .expect("Translator should be compatible with problem"),
    ));
}
