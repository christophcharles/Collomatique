use collomatique_ilp::LinExpr;

use crate::base::Identifier;

pub struct SealedGroups<
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
    SealedGroups<SubjectId, SlotId, GroupListId, StudentId>
{
    pub fn new(group_list_id: GroupListId) -> Self {
        use std::marker::PhantomData;
        SealedGroups {
            group_list_id,
            _phantom1: PhantomData,
            _phantom2: PhantomData,
            _phantom3: PhantomData,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SealedGroupsDesc<GroupListId: Identifier, StudentId: Identifier> {
    ForbidGroupNumberForStudent(GroupListId, u32, StudentId),
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    collomatique_solver::SimpleProblemConstraints
    for SealedGroups<SubjectId, SlotId, GroupListId, StudentId>
{
    type Problem =
        crate::base::ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>;
    type GeneralConstraintDesc = SealedGroupsDesc<GroupListId, StudentId>;
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

        for (i, prefilled_group) in group_list_desc.prefilled_groups.iter().enumerate() {
            if !prefilled_group.sealed {
                continue;
            }

            let group = i as u32;

            for student in group_list_desc.remaining_students.iter().copied() {
                let lhs = LinExpr::var(collomatique_solver::ExtraVariable::BaseStructure(
                    crate::base::variables::StructureVariable::StudentInGroup {
                        group_list: self.group_list_id,
                        student,
                        group,
                    },
                ));
                let rhs = LinExpr::constant(0.);
                constraints.push((
                    lhs.eq(&rhs),
                    SealedGroupsDesc::ForbidGroupNumberForStudent(
                        self.group_list_id,
                        group,
                        student,
                    ),
                ));
            }
        }

        constraints
    }
}
