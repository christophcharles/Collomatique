//! Group diversity optimization.
//!
//! This module contains the optimization function to minimize the number of different groups
//! so that students see mostly the same people across subjects and during the year.

use std::collections::BTreeSet;

use crate::base::Identifier;

pub struct GroupDiversityMinimizer<SubjectId: Identifier + 'static, StudentId: Identifier + 'static>
{
    _phantom1: std::marker::PhantomData<SubjectId>,
    _phantom2: std::marker::PhantomData<StudentId>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum StructureVariable<SubjectId: Identifier + 'static, StudentId: Identifier + 'static> {
    BothStudentInGroup {
        subject: SubjectId,
        group: u32,
        student1: StudentId,
        student2: StudentId,
    },
    StudentsShareAGroup {
        student1: StudentId,
        student2: StudentId,
    },
}

impl<SubjectId: Identifier + 'static, StudentId: Identifier + 'static>
    collomatique_solver::SimpleProblemConstraints
    for GroupDiversityMinimizer<SubjectId, StudentId>
{
    type Problem = crate::base::GroupListProblem<SubjectId, StudentId>;
    type GeneralConstraintDesc = ();
    type StructureVariable = StructureVariable<SubjectId, StudentId>;

    fn is_fit_for_problem(&self, _desc: &Self::Problem) -> bool {
        true
    }

    fn extra_aggregated_variables(
        &self,
        desc: &Self::Problem,
    ) -> Vec<
        Box<
            dyn collomatique_solver::tools::AggregatedVariables<
                collomatique_solver::generics::ExtraVariable<
                    <Self::Problem as collomatique_solver::BaseProblem>::MainVariable,
                    <Self::Problem as collomatique_solver::BaseProblem>::StructureVariable,
                    Self::StructureVariable,
                >,
            >,
        >,
    > {
        let mut output = vec![];

        let mut students = BTreeSet::new();

        for (subject_id, subject_desc) in &desc.subject_descriptions {
            let max_group = subject_desc.group_count.end().get() - 1;
            for student1 in &subject_desc.students {
                students.insert(student1.clone());
                for student2 in &subject_desc.students {
                    if student2 <= student1 {
                        continue;
                    }
                    for group in 0..=max_group {
                        output.push(Box::new(collomatique_solver::tools::AndVariable {
                            variable_name: collomatique_solver::generics::ExtraVariable::Extra(
                                StructureVariable::BothStudentInGroup {
                                    subject: subject_id.clone(),
                                    group,
                                    student1: student1.clone(),
                                    student2: student2.clone(),
                                },
                            ),
                            original_variables: BTreeSet::from([
                                collomatique_solver::generics::ExtraVariable::BaseStructure(
                                    crate::base::variables::StructureVariable::StudentInGroup {
                                        subject: subject_id.clone(),
                                        student: student1.clone(),
                                        group,
                                    },
                                ),
                                collomatique_solver::generics::ExtraVariable::BaseStructure(
                                    crate::base::variables::StructureVariable::StudentInGroup {
                                        subject: subject_id.clone(),
                                        student: student2.clone(),
                                        group,
                                    },
                                ),
                            ]),
                        })
                            as Box<dyn collomatique_solver::tools::AggregatedVariables<_>>);
                    }
                }
            }
        }

        for student1 in &students {
            for student2 in &students {
                if student2 <= student1 {
                    continue;
                }

                let mut original_variables = BTreeSet::new();

                for (subject_id, subject_desc) in &desc.subject_descriptions {
                    if !subject_desc.students.contains(student1)
                        || !subject_desc.students.contains(student2)
                    {
                        continue;
                    }

                    let max_group = subject_desc.group_count.end().get() - 1;
                    for group in 0..=max_group {
                        original_variables.insert(
                            collomatique_solver::generics::ExtraVariable::Extra(
                                StructureVariable::BothStudentInGroup {
                                    subject: subject_id.clone(),
                                    group,
                                    student1: student1.clone(),
                                    student2: student2.clone(),
                                },
                            ),
                        );
                    }
                }

                output.push(Box::new(collomatique_solver::tools::OrVariable {
                    variable_name: collomatique_solver::generics::ExtraVariable::Extra(
                        StructureVariable::StudentsShareAGroup {
                            student1: student1.clone(),
                            student2: student2.clone(),
                        },
                    ),
                    original_variables,
                })
                    as Box<dyn collomatique_solver::tools::AggregatedVariables<_>>);
            }
        }

        output
    }

    fn general_constraints(
        &self,
        _desc: &Self::Problem,
    ) -> Vec<(
        collomatique_ilp::Constraint<
            collomatique_solver::ExtraVariable<
                <Self::Problem as collomatique_solver::BaseProblem>::MainVariable,
                <Self::Problem as collomatique_solver::BaseProblem>::StructureVariable,
                Self::StructureVariable,
            >,
        >,
        Self::GeneralConstraintDesc,
    )> {
        vec![]
    }

    fn objective(
        &self,
        desc: &Self::Problem,
    ) -> collomatique_ilp::Objective<
        collomatique_solver::ExtraVariable<
            <Self::Problem as collomatique_solver::BaseProblem>::MainVariable,
            <Self::Problem as collomatique_solver::BaseProblem>::StructureVariable,
            Self::StructureVariable,
        >,
    > {
        let mut student_pair_counting_expr = collomatique_ilp::LinExpr::constant(0.);

        let mut students = BTreeSet::new();

        for (_subject_id, subject_desc) in &desc.subject_descriptions {
            students.extend(subject_desc.students.iter().cloned());
        }

        for student1 in &students {
            for student2 in &students {
                if student2 <= student1 {
                    continue;
                }

                student_pair_counting_expr = student_pair_counting_expr
                    + collomatique_ilp::LinExpr::var(
                        collomatique_solver::generics::ExtraVariable::Extra(
                            StructureVariable::StudentsShareAGroup {
                                student1: student1.clone(),
                                student2: student2.clone(),
                            },
                        ),
                    );
            }
        }

        collomatique_ilp::Objective::new(
            student_pair_counting_expr,
            collomatique_ilp::ObjectiveSense::Minimize,
        )
    }
}
