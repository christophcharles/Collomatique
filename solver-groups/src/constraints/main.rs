//! Main group lists constraints.
//!
//! This module contains the basic constraints that define what a
//! group lists are.

use crate::base::Identifier;
use std::num::NonZeroU32;

pub struct GroupListConstraints<SubjectId: Identifier + 'static, StudentId: Identifier + 'static> {
    _phantom1: std::marker::PhantomData<SubjectId>,
    _phantom2: std::marker::PhantomData<StudentId>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstraintDesc<SubjectId: Identifier + 'static> {
    GroupCountLowerBoundForSubject(SubjectId, NonZeroU32),
    GroupCountUpperBoundForSubject(SubjectId, NonZeroU32),
    StudentCountLowerBoundInSpecificGroup(SubjectId, u32, NonZeroU32),
    StudentCountUpperBoundInSpecificGroup(SubjectId, u32, NonZeroU32),
}

impl<SubjectId: Identifier + 'static, StudentId: Identifier + 'static>
    collomatique_solver::SimpleProblemConstraints for GroupListConstraints<SubjectId, StudentId>
{
    type Problem = crate::base::GroupListProblem<SubjectId, StudentId>;
    type GeneralConstraintDesc = ConstraintDesc<SubjectId>;
    type StructureVariable = ();

    fn is_fit_for_problem(&self, _desc: &Self::Problem) -> bool {
        true
    }

    fn extra_aggregated_variables(
        &self,
        _desc: &Self::Problem,
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
        vec![]
    }

    fn general_constraints(
        &self,
        desc: &Self::Problem,
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
        let mut constraints = vec![];

        for (subject_id, subject_desc) in &desc.subject_descriptions {
            let mut counting_group_expr = collomatique_ilp::LinExpr::<
                collomatique_solver::ExtraVariable<_, _, _>,
            >::constant(0.);
            let max_group = subject_desc.group_count.end().get() - 1;
            for group in 0..=max_group {
                counting_group_expr = counting_group_expr
                    + collomatique_ilp::LinExpr::var(
                        collomatique_solver::ExtraVariable::BaseStructure(
                            crate::base::variables::StructureVariable::NonEmptyGroup {
                                subject: subject_id.clone(),
                                group,
                            },
                        ),
                    );
            }

            constraints.push((
                counting_group_expr.geq(&collomatique_ilp::LinExpr::<
                    collomatique_solver::ExtraVariable<_, _, _>,
                >::constant(
                    subject_desc.group_count.start().get().into()
                )),
                ConstraintDesc::GroupCountLowerBoundForSubject(
                    subject_id.clone(),
                    subject_desc.group_count.start().clone(),
                ),
            ));
            constraints.push((
                counting_group_expr.leq(&collomatique_ilp::LinExpr::<
                    collomatique_solver::ExtraVariable<_, _, _>,
                >::constant(
                    subject_desc.group_count.end().get().into()
                )),
                ConstraintDesc::GroupCountUpperBoundForSubject(
                    subject_id.clone(),
                    subject_desc.group_count.end().clone(),
                ),
            ));

            for group in 0..=max_group {
                let mut counting_student_expr = collomatique_ilp::LinExpr::<
                    collomatique_solver::ExtraVariable<_, _, _>,
                >::constant(0.);
                for student_id in &subject_desc.students {
                    counting_student_expr = counting_student_expr
                        + collomatique_ilp::LinExpr::var(
                            collomatique_solver::ExtraVariable::BaseStructure(
                                crate::base::variables::StructureVariable::StudentInGroup {
                                    subject: subject_id.clone(),
                                    student: student_id.clone(),
                                    group,
                                },
                            ),
                        );
                }

                let lower_bound = f64::from(subject_desc.group_count.start().get());
                let conditional_lower_bound = lower_bound
                    * collomatique_ilp::LinExpr::var(
                        collomatique_solver::ExtraVariable::BaseStructure(
                            crate::base::variables::StructureVariable::NonEmptyGroup {
                                subject: subject_id.clone(),
                                group,
                            },
                        ),
                    );
                constraints.push((
                    counting_student_expr.geq(&conditional_lower_bound),
                    ConstraintDesc::StudentCountLowerBoundInSpecificGroup(
                        subject_id.clone(),
                        group,
                        subject_desc.group_count.start().clone(),
                    ),
                ));
                constraints.push((
                    counting_student_expr.leq(&collomatique_ilp::LinExpr::<
                        collomatique_solver::ExtraVariable<_, _, _>,
                    >::constant(
                        subject_desc.students_per_group.end().get().into(),
                    )),
                    ConstraintDesc::StudentCountUpperBoundInSpecificGroup(
                        subject_id.clone(),
                        group,
                        subject_desc.group_count.end().clone(),
                    ),
                ));
            }
        }

        constraints
    }
}
