//! Group count optimization.
//!
//! This module contains the optimization function to minimize the number of groups

use crate::base::Identifier;

pub struct GroupCountMinimizer<SubjectId: Identifier + 'static, StudentId: Identifier + 'static> {
    subject_id: SubjectId,
    _phantom: std::marker::PhantomData<StudentId>,
}

impl<SubjectId: Identifier + 'static, StudentId: Identifier + 'static>
    collomatique_solver::SimpleProblemConstraints for GroupCountMinimizer<SubjectId, StudentId>
{
    type Problem = crate::base::GroupListProblem<SubjectId, StudentId>;
    type GeneralConstraintDesc = ();
    type StructureVariable = ();

    fn is_fit_for_problem(&self, desc: &Self::Problem) -> bool {
        desc.subject_descriptions.contains_key(&self.subject_id)
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
        let mut counting_group_expr = collomatique_ilp::LinExpr::constant(0.);

        if let Some(subject_desc) = desc.subject_descriptions.get(&self.subject_id) {
            let max_group = subject_desc.group_count.end().get() - 1;
            for group in 0..=max_group {
                counting_group_expr = counting_group_expr
                    + collomatique_ilp::LinExpr::var(
                        collomatique_solver::ExtraVariable::BaseStructure(
                            crate::base::variables::StructureVariable::NonEmptyGroup {
                                subject: self.subject_id.clone(),
                                group,
                            },
                        ),
                    );
            }
        }

        collomatique_ilp::Objective::new(
            counting_group_expr,
            collomatique_ilp::ObjectiveSense::Minimize,
        )
    }
}
