use collomatique_ilp::LinExpr;
use collomatique_solver::ExtraVariable;

use crate::base::Identifier;

use std::collections::BTreeSet;
use std::num::NonZeroU32;

pub struct IncompatForSingleWeek<
    SubjectId: Identifier,
    SlotId: Identifier,
    GroupListId: Identifier,
    StudentId: Identifier,
    IncompatId: Identifier,
> {
    incompat_id: IncompatId,
    week: usize,
    students: BTreeSet<StudentId>,
    slots: Vec<collomatique_time::SlotWithDuration>,
    minimum_free_slots: NonZeroU32,
    _phantom1: std::marker::PhantomData<SlotId>,
    _phantom2: std::marker::PhantomData<GroupListId>,
    _phantom3: std::marker::PhantomData<SubjectId>,
}

impl<
        SubjectId: Identifier,
        SlotId: Identifier,
        GroupListId: Identifier,
        StudentId: Identifier,
        IncompatId: Identifier,
    > IncompatForSingleWeek<SubjectId, SlotId, GroupListId, StudentId, IncompatId>
{
    pub fn new(
        incompat_id: IncompatId,
        students: BTreeSet<StudentId>,
        week: usize,
        slots: Vec<collomatique_time::SlotWithDuration>,
        minimum_free_slots: NonZeroU32,
    ) -> Self {
        use std::marker::PhantomData;
        IncompatForSingleWeek {
            incompat_id,
            week,
            students,
            slots,
            minimum_free_slots,
            _phantom1: PhantomData,
            _phantom2: PhantomData,
            _phantom3: PhantomData,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum IncompatForSingleWeekVariable<IncompatId: Identifier, StudentId: Identifier> {
    IncompatSlotUsedByStudent {
        incompat_id: IncompatId,
        student: StudentId,
        week: usize,
        incompat_slot: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum IncompatForSingleWeekDesc<IncompatId: Identifier, StudentId: Identifier> {
    AtMinimumCountSlotAvailableForStudentAndWeek(IncompatId, StudentId, usize),
}

impl<
        SubjectId: Identifier,
        SlotId: Identifier,
        GroupListId: Identifier,
        StudentId: Identifier,
        IncompatId: Identifier,
    > collomatique_solver::SimpleProblemConstraints
    for IncompatForSingleWeek<SubjectId, SlotId, GroupListId, StudentId, IncompatId>
{
    type Problem =
        crate::base::ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>;
    type GeneralConstraintDesc = IncompatForSingleWeekDesc<IncompatId, StudentId>;
    type StructureVariable = IncompatForSingleWeekVariable<IncompatId, StudentId>;

    fn is_fit_for_problem(&self, desc: &Self::Problem) -> bool {
        let Some((_subject_id, subject_desc)) = desc.subject_descriptions.iter().next() else {
            return false;
        };

        self.week < subject_desc.group_assignments.len()
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
        let mut variables = vec![];

        for student_id in &self.students {
            for (incompat_slot, slot) in self.slots.iter().enumerate() {
                variables.push(self.build_single_variable(desc, *student_id, slot, incompat_slot))
            }
        }

        variables
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
        let mut constraints = vec![];

        for student_id in &self.students {
            let mut lhs = LinExpr::constant(0.);

            for incompat_slot in 0..self.slots.len() {
                lhs = lhs
                    + LinExpr::var(collomatique_solver::ExtraVariable::Extra(
                        IncompatForSingleWeekVariable::IncompatSlotUsedByStudent {
                            incompat_id: self.incompat_id,
                            student: *student_id,
                            week: self.week,
                            incompat_slot,
                        },
                    ));
            }

            let max_count = (self.slots.len() as u32) - self.minimum_free_slots.get();
            let rhs = LinExpr::constant(f64::from(max_count));
            constraints.push((
                lhs.leq(&rhs),
                IncompatForSingleWeekDesc::AtMinimumCountSlotAvailableForStudentAndWeek(
                    self.incompat_id,
                    *student_id,
                    self.week,
                ),
            ))
        }

        constraints
    }
}

impl<
        SubjectId: Identifier,
        SlotId: Identifier,
        GroupListId: Identifier,
        StudentId: Identifier,
        IncompatId: Identifier,
    > IncompatForSingleWeek<SubjectId, SlotId, GroupListId, StudentId, IncompatId>
{
    fn build_single_variable(&self, desc: &crate::base::ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>, student_id: StudentId, slot: &collomatique_time::SlotWithDuration, incompat_slot: usize) -> Box<
        dyn collomatique_solver::tools::AggregatedVariables<
            collomatique_solver::generics::ExtraVariable<
                <crate::base::ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId> as collomatique_solver::BaseProblem>::MainVariable,
                <crate::base::ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId> as collomatique_solver::BaseProblem>::StructureVariable,
                IncompatForSingleWeekVariable<IncompatId, StudentId>,
            >,
        >,
    >{
        let mut original_variables = BTreeSet::new();

        for (subject_id, subject_desc) in &desc.subject_descriptions {
            let group_assignment_opt = &subject_desc.group_assignments[self.week];
            let Some(group_assignment) = group_assignment_opt else {
                continue;
            };
            if !group_assignment.enrolled_students.contains(&student_id) {
                continue;
            }

            for (slot_id, slot_desc) in &subject_desc.slots_descriptions {
                if !slot_desc.weeks[self.week] {
                    continue;
                }

                let slot_with_duration = collomatique_time::SlotWithDuration::new(
                    slot_desc.slot_start.clone(),
                    subject_desc.duration,
                )
                .expect("Slots should be compatible with their duration at this point");

                if !slot_with_duration.overlaps_with(slot) {
                    continue;
                }

                original_variables.insert(collomatique_solver::ExtraVariable::BaseStructure(
                    crate::base::variables::StructureVariable::StudentInSlot {
                        subject: *subject_id,
                        student: student_id,
                        slot: *slot_id,
                        week: self.week,
                    },
                ));
            }
        }

        use collomatique_solver::tools::{FixedVariable, OrVariable};
        if original_variables.is_empty() {
            Box::new(FixedVariable {
                variable_name: ExtraVariable::Extra(
                    IncompatForSingleWeekVariable::IncompatSlotUsedByStudent {
                        incompat_id: self.incompat_id,
                        student: student_id,
                        week: self.week,
                        incompat_slot,
                    },
                ),
                value: false,
            }) as Box<dyn collomatique_solver::tools::AggregatedVariables<_>>
        } else {
            Box::new(OrVariable {
                variable_name: ExtraVariable::Extra(
                    IncompatForSingleWeekVariable::IncompatSlotUsedByStudent {
                        incompat_id: self.incompat_id,
                        student: student_id,
                        week: self.week,
                        incompat_slot,
                    },
                ),
                original_variables,
            }) as Box<dyn collomatique_solver::tools::AggregatedVariables<_>>
        }
    }
}
