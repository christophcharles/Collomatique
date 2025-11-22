//! Main group lists constraints.
//!
//! This module contains the basic constraints that define what a
//! group lists are.

use crate::base::Identifier;
use std::num::NonZeroU32;

pub struct GroupListConstraints<
    PeriodId: Identifier + 'static,
    SubjectId: Identifier + 'static,
    StudentId: Identifier + 'static,
> {
    period_id: PeriodId,
    _phantom1: std::marker::PhantomData<SubjectId>,
    _phantom2: std::marker::PhantomData<StudentId>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstraintDesc<PeriodId: Identifier + 'static, SubjectId: Identifier + 'static> {
    GroupCountLowerBoundForSubject(PeriodId, SubjectId, NonZeroU32),
    GroupCountUpperBoundForSubject(PeriodId, SubjectId, NonZeroU32),
    StudentCountLowerBoundInSpecificGroup(PeriodId, SubjectId, u32, NonZeroU32),
    StudentCountUpperBoundInSpecificGroup(PeriodId, SubjectId, u32, NonZeroU32),
}

impl<
        PeriodId: Identifier + 'static,
        SubjectId: Identifier + 'static,
        StudentId: Identifier + 'static,
    > collomatique_solver::SimpleProblemConstraints
    for GroupListConstraints<PeriodId, SubjectId, StudentId>
{
    type Problem = crate::base::GroupListProblem<PeriodId, SubjectId, StudentId>;
    type GeneralConstraintDesc = ConstraintDesc<PeriodId, SubjectId>;
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
            let Some(period_desc) = subject_desc.period_descriptions.get(&self.period_id) else {
                return vec![];
            };

            let mut counting_group_expr = collomatique_ilp::LinExpr::<
                collomatique_solver::ExtraVariable<_, _, _>,
            >::constant(0.);
            let max_group = period_desc.group_count.end().get() - 1;
            for group in 0..=max_group {
                counting_group_expr = counting_group_expr
                    + collomatique_ilp::LinExpr::var(
                        collomatique_solver::ExtraVariable::BaseStructure(
                            crate::base::variables::StructureVariable::NonEmptyGroup {
                                period: self.period_id.clone(),
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
                    period_desc.group_count.start().get().into()
                )),
                ConstraintDesc::GroupCountLowerBoundForSubject(
                    self.period_id.clone(),
                    subject_id.clone(),
                    period_desc.group_count.start().clone(),
                ),
            ));
            constraints.push((
                counting_group_expr.leq(&collomatique_ilp::LinExpr::<
                    collomatique_solver::ExtraVariable<_, _, _>,
                >::constant(
                    period_desc.group_count.end().get().into()
                )),
                ConstraintDesc::GroupCountUpperBoundForSubject(
                    self.period_id.clone(),
                    subject_id.clone(),
                    period_desc.group_count.end().clone(),
                ),
            ));

            for group in 0..=max_group {
                let mut counting_student_expr = collomatique_ilp::LinExpr::<
                    collomatique_solver::ExtraVariable<_, _, _>,
                >::constant(0.);
                for student_id in &period_desc.students {
                    counting_student_expr = counting_student_expr
                        + collomatique_ilp::LinExpr::var(
                            collomatique_solver::ExtraVariable::BaseStructure(
                                crate::base::variables::StructureVariable::StudentInGroup {
                                    period: self.period_id.clone(),
                                    subject: subject_id.clone(),
                                    student: student_id.clone(),
                                    group,
                                },
                            ),
                        );
                }

                let lower_bound = f64::from(subject_desc.students_per_group.start().get());
                let conditional_lower_bound = lower_bound
                    * collomatique_ilp::LinExpr::var(
                        collomatique_solver::ExtraVariable::BaseStructure(
                            crate::base::variables::StructureVariable::NonEmptyGroup {
                                period: self.period_id.clone(),
                                subject: subject_id.clone(),
                                group,
                            },
                        ),
                    );
                constraints.push((
                    counting_student_expr.geq(&conditional_lower_bound),
                    ConstraintDesc::StudentCountLowerBoundInSpecificGroup(
                        self.period_id.clone(),
                        subject_id.clone(),
                        group,
                        subject_desc.students_per_group.start().clone(),
                    ),
                ));
                constraints.push((
                    counting_student_expr.leq(&collomatique_ilp::LinExpr::<
                        collomatique_solver::ExtraVariable<_, _, _>,
                    >::constant(
                        subject_desc.students_per_group.end().get().into(),
                    )),
                    ConstraintDesc::StudentCountUpperBoundInSpecificGroup(
                        self.period_id.clone(),
                        subject_id.clone(),
                        group,
                        subject_desc.students_per_group.end().clone(),
                    ),
                ));
            }
        }

        constraints
    }
}
