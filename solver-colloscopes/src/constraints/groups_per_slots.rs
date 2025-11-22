use collomatique_ilp::LinExpr;

use crate::base::Identifier;

pub struct GroupsPerSlots<
    SubjectId: Identifier,
    SlotId: Identifier,
    GroupListId: Identifier,
    StudentId: Identifier,
> {
    weeks: Vec<bool>,
    subject_id: SubjectId,
    _phantom1: std::marker::PhantomData<SlotId>,
    _phantom2: std::marker::PhantomData<GroupListId>,
    _phantom3: std::marker::PhantomData<StudentId>,
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    GroupsPerSlots<SubjectId, SlotId, GroupListId, StudentId>
{
    pub fn new(subject_id: SubjectId, weeks: Vec<bool>) -> Self {
        use std::marker::PhantomData;
        GroupsPerSlots {
            weeks,
            subject_id,
            _phantom1: PhantomData,
            _phantom2: PhantomData,
            _phantom3: PhantomData,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroupsPerSlotsDesc<SubjectId: Identifier, SlotId: Identifier> {
    AtMostCountGroupInSlotForWeek(SubjectId, u32, SlotId, usize),
    AtMinimumCountGroupInNonEmptySlotForWeek(SubjectId, u32, SlotId, usize),
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    collomatique_solver::SimpleProblemConstraints
    for GroupsPerSlots<SubjectId, SlotId, GroupListId, StudentId>
{
    type Problem =
        crate::base::ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>;
    type GeneralConstraintDesc = GroupsPerSlotsDesc<SubjectId, SlotId>;
    type StructureVariable = ();

    fn is_fit_for_problem(&self, desc: &Self::Problem) -> bool {
        let Some(subject_desc) = desc.subject_descriptions.get(&self.subject_id) else {
            return false;
        };

        subject_desc.group_assignments.len() == self.weeks.len()
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

        let subject_desc = desc
            .subject_descriptions
            .get(&self.subject_id)
            .expect("Subject ID should be valid if this is compatible with the base problem");

        for (week, group_assignment_opt) in subject_desc.group_assignments.iter().enumerate() {
            let Some(group_assignment) = group_assignment_opt else {
                continue;
            };
            if !self.weeks[week] {
                continue;
            }

            let group_list_desc = desc
                .group_list_descriptions
                .get(&group_assignment.group_list_id)
                .expect("Group list id should be valid");
            let max_group_count = group_list_desc.prefilled_groups.len() as u32;

            for (slot_id, slot_desc) in &subject_desc.slots_descriptions {
                if !slot_desc.weeks[week] {
                    continue;
                }

                let mut lhs = LinExpr::constant(0.);
                for group in 0..max_group_count {
                    lhs = lhs
                        + LinExpr::var(collomatique_solver::ExtraVariable::BaseMain(
                            crate::base::variables::MainVariable::GroupInSlot {
                                subject: self.subject_id,
                                slot: *slot_id,
                                week,
                                group,
                            },
                        ))
                }
                let max_count = subject_desc.groups_per_interrogation.end().get();
                let rhs = LinExpr::constant(f64::from(max_count));
                constraints.push((
                    lhs.leq(&rhs),
                    GroupsPerSlotsDesc::AtMostCountGroupInSlotForWeek(
                        self.subject_id,
                        max_count,
                        *slot_id,
                        week,
                    ),
                ));

                let min_count = subject_desc.groups_per_interrogation.start().get();
                if min_count != 0 {
                    let rhs = f64::from(min_count)
                        * LinExpr::var(collomatique_solver::ExtraVariable::BaseStructure(
                            crate::base::variables::StructureVariable::NonEmptySlot {
                                subject: self.subject_id,
                                slot: *slot_id,
                                week,
                            },
                        ));
                    constraints.push((
                        lhs.geq(&rhs),
                        GroupsPerSlotsDesc::AtMinimumCountGroupInNonEmptySlotForWeek(
                            self.subject_id,
                            min_count,
                            *slot_id,
                            week,
                        ),
                    ));
                }
            }
        }

        constraints
    }
}
