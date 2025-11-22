use collomatique_ilp::LinExpr;

use crate::base::Identifier;

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

pub struct StrictLimits<
    SubjectId: Identifier,
    SlotId: Identifier,
    GroupListId: Identifier,
    StudentId: Identifier,
> {
    weeks: Vec<bool>,
    students: BTreeSet<StudentId>,
    interrogations_per_week_min: Option<u32>,
    interrogations_per_week_max: Option<u32>,
    max_interrogations_per_day: Option<NonZeroU32>,
    _phantom1: std::marker::PhantomData<SlotId>,
    _phantom2: std::marker::PhantomData<GroupListId>,
    _phantom3: std::marker::PhantomData<SubjectId>,
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    StrictLimits<SubjectId, SlotId, GroupListId, StudentId>
{
    pub fn new(
        students: BTreeSet<StudentId>,
        weeks: Vec<bool>,
        interrogations_per_week_min: Option<u32>,
        interrogations_per_week_max: Option<u32>,
        max_interrogations_per_day: Option<NonZeroU32>,
    ) -> Self {
        use std::marker::PhantomData;
        StrictLimits {
            weeks,
            students,
            interrogations_per_week_min,
            interrogations_per_week_max,
            max_interrogations_per_day,
            _phantom1: PhantomData,
            _phantom2: PhantomData,
            _phantom3: PhantomData,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StrictLimitsDesc<StudentId: Identifier> {
    AtMostCountInterrogationPerDayForStudentOnWeekAndDay(
        u32,
        StudentId,
        usize,
        collomatique_time::Weekday,
    ),
    AtMostCountInterrogationPerWeekForStudentOnWeek(u32, StudentId, usize),
    AtMinimumCountInterrogationPerWeekForStudentOnWeek(u32, StudentId, usize),
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    collomatique_solver::SimpleProblemConstraints
    for StrictLimits<SubjectId, SlotId, GroupListId, StudentId>
{
    type Problem =
        crate::base::ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>;
    type GeneralConstraintDesc = StrictLimitsDesc<StudentId>;
    type StructureVariable = ();

    fn is_fit_for_problem(&self, desc: &Self::Problem) -> bool {
        let Some((_subject_id, subject_desc)) = desc.subject_descriptions.iter().next() else {
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

        for student_id in &self.students {
            for (week, week_status) in self.weeks.iter().enumerate() {
                if !*week_status {
                    continue;
                }

                let mut counting_interrogations_in_week_expr = LinExpr::constant(0.);
                let mut counting_interrogations_per_day_exprs: BTreeMap<_, _> =
                    collomatique_time::Weekday::iter()
                        .map(|weekday| (weekday, LinExpr::constant(0.)))
                        .collect();

                for (subject_id, subject_desc) in &desc.subject_descriptions {
                    if !subject_desc.take_duration_into_account {
                        continue;
                    }

                    let group_assignment_opt = &subject_desc.group_assignments[week];
                    let Some(group_assignment) = group_assignment_opt else {
                        continue;
                    };

                    if !group_assignment.enrolled_students.contains(student_id) {
                        continue;
                    }

                    for (slot_id, slot_desc) in &subject_desc.slots_descriptions {
                        if !slot_desc.weeks[week] {
                            continue;
                        }

                        counting_interrogations_in_week_expr = counting_interrogations_in_week_expr
                            + LinExpr::var(collomatique_solver::ExtraVariable::BaseStructure(
                                crate::base::variables::StructureVariable::StudentInSlot {
                                    subject: *subject_id,
                                    student: *student_id,
                                    slot: *slot_id,
                                    week,
                                },
                            ));

                        let new_expr = &counting_interrogations_per_day_exprs
                            [&slot_desc.slot_start.weekday]
                            + &LinExpr::var(collomatique_solver::ExtraVariable::BaseStructure(
                                crate::base::variables::StructureVariable::StudentInSlot {
                                    subject: *subject_id,
                                    student: *student_id,
                                    slot: *slot_id,
                                    week,
                                },
                            ));

                        counting_interrogations_per_day_exprs
                            .insert(slot_desc.slot_start.weekday.clone(), new_expr);
                    }
                }

                if let Some(max_count) = &self.interrogations_per_week_max {
                    let rhs = LinExpr::constant(f64::from(*max_count));
                    constraints.push((
                        counting_interrogations_in_week_expr.leq(&rhs),
                        StrictLimitsDesc::AtMostCountInterrogationPerWeekForStudentOnWeek(
                            *max_count,
                            *student_id,
                            week,
                        ),
                    ));
                }

                if let Some(min_count) = &self.interrogations_per_week_min {
                    let rhs = LinExpr::constant(f64::from(*min_count));
                    constraints.push((
                        counting_interrogations_in_week_expr.geq(&rhs),
                        StrictLimitsDesc::AtMinimumCountInterrogationPerWeekForStudentOnWeek(
                            *min_count,
                            *student_id,
                            week,
                        ),
                    ));
                }

                if let Some(count_per_day) = &self.max_interrogations_per_day {
                    let max_count = count_per_day.get();
                    let rhs = LinExpr::constant(f64::from(max_count));
                    for (weekday, lhs) in counting_interrogations_per_day_exprs {
                        if lhs.coefficients().len() == 0 {
                            continue;
                        }
                        constraints.push((
                            lhs.leq(&rhs),
                            StrictLimitsDesc::AtMostCountInterrogationPerDayForStudentOnWeekAndDay(
                                max_count,
                                *student_id,
                                week,
                                weekday,
                            ),
                        ));
                    }
                }
            }
        }

        constraints
    }
}
