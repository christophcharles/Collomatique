use collomatique_ilp::LinExpr;

use crate::base::Identifier;

use std::collections::{BTreeMap, BTreeSet};

pub struct OneInterrogationAtATime<
    SubjectId: Identifier,
    SlotId: Identifier,
    GroupListId: Identifier,
    StudentId: Identifier,
> {
    weeks: Vec<bool>,
    students: BTreeSet<StudentId>,
    _phantom1: std::marker::PhantomData<SlotId>,
    _phantom2: std::marker::PhantomData<GroupListId>,
    _phantom3: std::marker::PhantomData<SubjectId>,
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    OneInterrogationAtATime<SubjectId, SlotId, GroupListId, StudentId>
{
    pub fn new(students: BTreeSet<StudentId>, weeks: Vec<bool>) -> Self {
        use std::marker::PhantomData;
        OneInterrogationAtATime {
            weeks,
            students,
            _phantom1: PhantomData,
            _phantom2: PhantomData,
            _phantom3: PhantomData,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OneInterrogationAtATimeDesc<StudentId: Identifier> {
    AtMostOneInterrogationAtATimeForStudent(
        BTreeSet<(StudentId, collomatique_time::SlotWithDuration)>,
        usize,
        collomatique_time::Weekday,
    ),
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    collomatique_solver::SimpleProblemConstraints
    for OneInterrogationAtATime<SubjectId, SlotId, GroupListId, StudentId>
{
    type Problem =
        crate::base::ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>;
    type GeneralConstraintDesc = OneInterrogationAtATimeDesc<StudentId>;
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
        let mut constraints = BTreeMap::new();

        let time_resolution = Self::compute_needed_time_resolution(desc);
        let slots_in_day = MINUTES_IN_DAY / time_resolution;

        for slot_num in 0..slots_in_day {
            let start_time_in_minutes = slot_num * time_resolution;
            let start_hour = start_time_in_minutes / MINUTES_IN_HOUR;
            let start_minute = start_time_in_minutes % MINUTES_IN_HOUR;

            let naive_time = chrono::NaiveTime::from_hms_milli_opt(start_hour, start_minute, 0, 0)
                .expect("time should be valid");

            for (week, week_status) in self.weeks.iter().enumerate() {
                if !*week_status {
                    continue;
                }

                for weekday in collomatique_time::Weekday::iter() {
                    let start = collomatique_time::SlotStart {
                        weekday,
                        start_time: collomatique_time::TimeOnMinutes::new(naive_time)
                            .expect("Time should be on a minute"),
                    };
                    let duration =
                        collomatique_time::NonZeroDurationInMinutes::new(time_resolution)
                            .expect("Time resolution should be non-zero");
                    let slot = collomatique_time::SlotWithDuration::new(start, duration)
                        .expect("Slot should not overlap next day");

                    for student_id in &self.students {
                        if let Some(constraint) =
                            Self::build_single_constraint(desc, &slot, week, *student_id)
                        {
                            if let Some(OneInterrogationAtATimeDesc::AtMostOneInterrogationAtATimeForStudent(prev_set, prev_week, prev_day)) = constraints.get_mut(&constraint) {
                                assert_eq!(*prev_week, week);
                                assert_eq!(*prev_day, weekday);

                                prev_set.insert((*student_id, slot.clone()));
                            } else {
                                constraints.insert(
                                    constraint,
                                    OneInterrogationAtATimeDesc::AtMostOneInterrogationAtATimeForStudent(
                                        BTreeSet::from([(*student_id, slot.clone())]),
                                        week,
                                        weekday,
                                    )
                                );
                            }
                        }
                    }
                }
            }
        }

        constraints.into_iter().collect()
    }
}

const MINUTES_IN_HOUR: u32 = 60;
const HOURS_IN_DAY: u32 = 24;
const MINUTES_IN_DAY: u32 = MINUTES_IN_HOUR * HOURS_IN_DAY;

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    OneInterrogationAtATime<SubjectId, SlotId, GroupListId, StudentId>
{
    fn compute_needed_time_resolution(
        desc: &crate::base::ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>,
    ) -> u32 {
        let mut time_resolution = MINUTES_IN_DAY;

        for (_subject_id, subject_desc) in &desc.subject_descriptions {
            time_resolution = gcd(time_resolution, subject_desc.duration.get().get());

            for (_slot_id, slot_desc) in &subject_desc.slots_descriptions {
                use chrono::Timelike;

                let start = slot_desc.slot_start.start_time.inner();
                let start_in_minutes_from_midnight =
                    start.hour() * MINUTES_IN_HOUR + start.minute();

                time_resolution = gcd(time_resolution, start_in_minutes_from_midnight);
            }
        }

        time_resolution
    }

    fn build_single_constraint(
        desc: &crate::base::ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>,
        slot: &collomatique_time::SlotWithDuration,
        week: usize,
        student_id: StudentId,
    ) -> Option<collomatique_ilp::Constraint<
            collomatique_solver::ExtraVariable<
                <crate::base::ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId> as collomatique_solver::BaseProblem>::MainVariable,
                <crate::base::ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId> as collomatique_solver::BaseProblem>::StructureVariable,
                (),
            >,
    >>{
        let mut lhs = LinExpr::constant(0.);

        for (subject_id, subject_desc) in &desc.subject_descriptions {
            let group_assignment_opt = &subject_desc.group_assignments[week];
            let Some(group_assignment) = group_assignment_opt else {
                continue;
            };
            if !group_assignment.enrolled_students.contains(&student_id) {
                continue;
            }

            for (slot_id, slot_desc) in &subject_desc.slots_descriptions {
                if !slot_desc.weeks[week] {
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

                lhs = lhs
                    + LinExpr::var(collomatique_solver::ExtraVariable::BaseStructure(
                        crate::base::variables::StructureVariable::StudentInSlot {
                            subject: *subject_id,
                            student: student_id,
                            slot: *slot_id,
                            week,
                        },
                    ));
            }
        }

        if lhs.coefficients().len() != 0 {
            let one = LinExpr::constant(1.0);
            Some(lhs.leq(&one))
        } else {
            None
        }
    }
}

fn gcd(mut n1: u32, mut n2: u32) -> u32 {
    while n2 != 0 {
        let r = n1 % n2;
        n1 = n2;
        n2 = r;
    }

    n1
}
