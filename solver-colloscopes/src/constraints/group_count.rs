use collomatique_ilp::LinExpr;

use crate::base::Identifier;

pub struct GroupCount<
    SubjectId: Identifier,
    SlotId: Identifier,
    GroupListId: Identifier,
    StudentId: Identifier,
> {
    group_list_id: GroupListId,
    _phantom1: std::marker::PhantomData<SlotId>,
    _phantom2: std::marker::PhantomData<SubjectId>,
    _phantom3: std::marker::PhantomData<StudentId>,
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    GroupCount<SubjectId, SlotId, GroupListId, StudentId>
{
    pub fn new(group_list_id: GroupListId) -> Self {
        use std::marker::PhantomData;
        GroupCount {
            group_list_id,
            _phantom1: PhantomData,
            _phantom2: PhantomData,
            _phantom3: PhantomData,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroupCountDesc<GroupListId: Identifier> {
    AtMinimumCountNonEmptyGroup(GroupListId, u32),
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    collomatique_solver::SimpleProblemConstraints
    for GroupCount<SubjectId, SlotId, GroupListId, StudentId>
{
    type Problem =
        crate::base::ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>;
    type GeneralConstraintDesc = GroupCountDesc<GroupListId>;
    type StructureVariable = ();

    fn is_fit_for_problem(&self, desc: &Self::Problem) -> bool {
        desc.group_list_descriptions
            .contains_key(&self.group_list_id)
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

        let group_list_desc = desc
            .group_list_descriptions
            .get(&self.group_list_id)
            .expect("Group list ID should be valid if this is compatible with the base problem");

        let mut lhs = LinExpr::constant(0.);
        let max_group_count = group_list_desc.prefilled_groups.len() as u32;
        for group in 0..max_group_count {
            lhs = lhs
                + LinExpr::var(collomatique_solver::ExtraVariable::BaseStructure(
                    crate::base::variables::StructureVariable::NonEmptyGroup {
                        group_list: self.group_list_id,
                        group,
                    },
                ));
        }

        let min_count = group_list_desc.minimum_group_count;
        if min_count > 0 {
            let rhs = LinExpr::constant(f64::from(min_count));
            constraints.push((
                lhs.geq(&rhs),
                GroupCountDesc::AtMinimumCountNonEmptyGroup(self.group_list_id, min_count),
            ));
        }

        constraints
    }
}
