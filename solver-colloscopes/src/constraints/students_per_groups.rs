use collomatique_ilp::LinExpr;

use crate::base::Identifier;

pub struct StudentsPerGroups<
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
    StudentsPerGroups<SubjectId, SlotId, GroupListId, StudentId>
{
    pub fn new(group_list_id: GroupListId) -> Self {
        use std::marker::PhantomData;
        StudentsPerGroups {
            group_list_id,
            _phantom1: PhantomData,
            _phantom2: PhantomData,
            _phantom3: PhantomData,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StudentsPerGroupsDesc<GroupListId: Identifier> {
    AtMostCountStudentsInGroup(GroupListId, u32, u32),
    AtMinimumCountStudentsInNonEmptyGroup(GroupListId, u32, u32),
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    collomatique_solver::SimpleProblemConstraints
    for StudentsPerGroups<SubjectId, SlotId, GroupListId, StudentId>
{
    type Problem =
        crate::base::ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>;
    type GeneralConstraintDesc = StudentsPerGroupsDesc<GroupListId>;
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
            if prefilled_group.sealed {
                continue;
            }

            let group = i as u32;

            let mut lhs = LinExpr::constant(0.);
            for student in group_list_desc.remaining_students.iter().copied() {
                lhs = lhs
                    + LinExpr::var(collomatique_solver::ExtraVariable::BaseStructure(
                        crate::base::variables::StructureVariable::StudentInGroup {
                            group_list: self.group_list_id,
                            student,
                            group,
                        },
                    ));
            }

            let max_count = group_list_desc.students_per_group.end().get();
            let students_already_present = prefilled_group.students.len() as u32;
            assert!(students_already_present <= max_count);

            let rhs = LinExpr::constant(f64::from(max_count - students_already_present));
            constraints.push((
                lhs.leq(&rhs),
                StudentsPerGroupsDesc::AtMostCountStudentsInGroup(
                    self.group_list_id,
                    max_count,
                    group,
                ),
            ));

            let min_count = group_list_desc.students_per_group.start().get();
            if min_count > students_already_present {
                let rhs = f64::from(min_count - students_already_present)
                    * LinExpr::var(collomatique_solver::ExtraVariable::BaseStructure(
                        crate::base::variables::StructureVariable::NonEmptyGroup {
                            group_list: self.group_list_id,
                            group,
                        },
                    ));
                constraints.push((
                    lhs.geq(&rhs),
                    StudentsPerGroupsDesc::AtMinimumCountStudentsInNonEmptyGroup(
                        self.group_list_id,
                        min_count,
                        group,
                    ),
                ));
            }
        }

        constraints
    }
}
