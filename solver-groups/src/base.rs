//! Definition of relevant structures to describe group lists

pub mod solution;
pub mod variables;

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use std::ops::RangeInclusive;

pub trait Identifier:
    Clone + Copy + std::fmt::Debug + Ord + PartialOrd + Eq + PartialEq + Send + Sync
{
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectDescription<StudentId: Identifier> {
    pub students: BTreeSet<StudentId>,
    pub group_count: RangeInclusive<NonZeroU32>,
    pub students_per_group: RangeInclusive<NonZeroU32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodDescription<SubjectId: Identifier, StudentId: Identifier> {
    pub subject_descriptions: BTreeMap<SubjectId, SubjectDescription<StudentId>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupListProblem<
    PeriodId: Identifier + 'static,
    SubjectId: Identifier + 'static,
    StudentId: Identifier + 'static,
> {
    pub period_descriptions: BTreeMap<PeriodId, PeriodDescription<SubjectId, StudentId>>,
}

impl<
        PeriodId: Identifier + 'static,
        SubjectId: Identifier + 'static,
        StudentId: Identifier + 'static,
    > collomatique_solver::SimpleBaseProblem for GroupListProblem<PeriodId, SubjectId, StudentId>
{
    type MainVariable = variables::MainVariable<PeriodId, SubjectId, StudentId>;
    type PartialSolution = solution::GroupListSolution<PeriodId, SubjectId, StudentId>;
    type StructureVariable = variables::StructureVariable<PeriodId, SubjectId, StudentId>;

    fn main_variables(
        &self,
    ) -> std::collections::BTreeMap<Self::MainVariable, collomatique_ilp::Variable> {
        self.period_descriptions
            .iter()
            .flat_map(|(period_id, period_desc)| {
                let period = period_id.clone();
                period_desc.subject_descriptions.iter().flat_map(
                    move |(subject_id, subject_desc)| {
                        let max_group = subject_desc.group_count.end().get() - 1;
                        let subject = subject_id.clone();
                        subject_desc.students.iter().map(move |student_id| {
                            (
                                variables::MainVariable::GroupForStudent {
                                    period,
                                    subject,
                                    student: student_id.clone(),
                                },
                                collomatique_ilp::Variable::integer()
                                    .min(0.)
                                    .max(max_group.into()),
                            )
                        })
                    },
                )
            })
            .collect()
    }

    fn aggregated_variables(
        &self,
    ) -> Vec<
        Box<
            dyn collomatique_solver::tools::AggregatedVariables<
                collomatique_solver::generics::BaseVariable<
                    Self::MainVariable,
                    Self::StructureVariable,
                >,
            >,
        >,
    > {
        let mut output = vec![];

        for (period_id, period_desc) in &self.period_descriptions {
            for (subject_id, subject_desc) in &period_desc.subject_descriptions {
                let max_group = subject_desc.group_count.end().get() - 1;
                for student_id in &subject_desc.students {
                    let subject = subject_id.clone();
                    let student = student_id.clone();
                    let period = period_id.clone();
                    output.push(Box::new(collomatique_solver::tools::UIntToBinVariables {
                        variable_name_builder: move |i| {
                            collomatique_solver::generics::BaseVariable::Structure(
                                variables::StructureVariable::StudentInGroup {
                                    period,
                                    subject,
                                    student,
                                    group: i,
                                },
                            )
                        },
                        original_variable: collomatique_solver::generics::BaseVariable::Main(
                            variables::MainVariable::GroupForStudent {
                                period: period_id.clone(),
                                subject: subject_id.clone(),
                                student: student_id.clone(),
                            },
                        ),
                        original_range: 0..=max_group,
                    })
                        as Box<dyn collomatique_solver::tools::AggregatedVariables<_>>);
                }

                for group in 0..=max_group {
                    output.push(Box::new(collomatique_solver::tools::OrVariable {
                        variable_name: collomatique_solver::generics::BaseVariable::Structure(
                            variables::StructureVariable::NonEmptyGroup {
                                period: period_id.clone(),
                                subject: subject_id.clone(),
                                group,
                            },
                        ),
                        original_variables: subject_desc
                            .students
                            .iter()
                            .map(|student_id| {
                                collomatique_solver::generics::BaseVariable::Structure(
                                    variables::StructureVariable::StudentInGroup {
                                        period: period_id.clone(),
                                        subject: subject_id.clone(),
                                        student: student_id.clone(),
                                        group,
                                    },
                                )
                            })
                            .collect(),
                    })
                        as Box<dyn collomatique_solver::tools::AggregatedVariables<_>>);
                }
            }
        }

        output
    }

    fn configuration_to_partial_solution(
        &self,
        config: &collomatique_ilp::ConfigData<Self::MainVariable>,
    ) -> Self::PartialSolution {
        solution::GroupListSolution {
            period_map: self
                .period_descriptions
                .iter()
                .map(|(period_id, period_desc)| {
                    (
                        period_id.clone(),
                        solution::GroupListsForPeriod {
                            group_lists: period_desc
                                .subject_descriptions
                                .iter()
                                .map(|(subject_id, subject_desc)| {
                                    (
                                        subject_id.clone(),
                                        solution::GroupList {
                                            student_map: subject_desc
                                                .students
                                                .iter()
                                                .filter_map(|student_id| {
                                                    let var =
                                                        variables::MainVariable::GroupForStudent {
                                                            period: period_id.clone(),
                                                            subject: subject_id.clone(),
                                                            student: student_id.clone(),
                                                        };
                                                    match config.get(var) {
                                                        Some(v) => {
                                                            Some((student_id.clone(), v as u32))
                                                        }
                                                        None => None,
                                                    }
                                                })
                                                .collect(),
                                        },
                                    )
                                })
                                .collect(),
                        },
                    )
                })
                .collect(),
        }
    }

    fn partial_solution_to_configuration(
        &self,
        sol: &Self::PartialSolution,
    ) -> Option<collomatique_ilp::ConfigData<Self::MainVariable>> {
        let mut config = collomatique_ilp::ConfigData::new();

        for (period_id, lists_for_period) in &sol.period_map {
            let Some(period_desc) = self.period_descriptions.get(period_id) else {
                return None;
            };

            for (subject_id, group_list) in &lists_for_period.group_lists {
                let Some(subject_desc) = period_desc.subject_descriptions.get(subject_id) else {
                    return None;
                };

                let max_group = subject_desc.students_per_group.end().get() - 1;
                for (student_id, group_num) in &group_list.student_map {
                    if !subject_desc.students.contains(student_id) {
                        return None;
                    }
                    if *group_num > max_group {
                        return None;
                    }
                    config = config.set(
                        variables::MainVariable::GroupForStudent {
                            period: period_id.clone(),
                            subject: subject_id.clone(),
                            student: student_id.clone(),
                        },
                        f64::from(*group_num),
                    );
                }
            }
        }

        Some(config)
    }
}
