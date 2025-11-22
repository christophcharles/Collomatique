use std::collections::BTreeSet;

use collomatique_ilp::LinExpr;

use crate::base::Identifier;

pub struct StudentsPerGroupsForSubject<
    SubjectId: Identifier,
    SlotId: Identifier,
    GroupListId: Identifier,
    StudentId: Identifier,
> {
    group_list_id: GroupListId,
    subject_id: SubjectId,
    _phantom1: std::marker::PhantomData<SlotId>,
    _phantom2: std::marker::PhantomData<SubjectId>,
    _phantom3: std::marker::PhantomData<StudentId>,
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    StudentsPerGroupsForSubject<SubjectId, SlotId, GroupListId, StudentId>
{
    pub fn new(group_list_id: GroupListId, subject_id: SubjectId) -> Self {
        use std::marker::PhantomData;
        StudentsPerGroupsForSubject {
            group_list_id,
            subject_id,
            _phantom1: PhantomData,
            _phantom2: PhantomData,
            _phantom3: PhantomData,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StudentsPerGroupsForSubjectDesc<
    SubjectId: Identifier,
    GroupListId: Identifier,
    StudentId: Identifier,
> {
    AtMostCountStudentsInGroupForSubjectWithSubclass(
        GroupListId,
        u32,
        u32,
        SubjectId,
        BTreeSet<StudentId>,
    ),
    AtMinimumCountStudentsInNonEmptyGroupForSubjectWithSubclass(
        GroupListId,
        u32,
        u32,
        SubjectId,
        BTreeSet<StudentId>,
    ),
}

impl<SubjectId: Identifier, SlotId: Identifier, GroupListId: Identifier, StudentId: Identifier>
    collomatique_solver::SimpleProblemConstraints
    for StudentsPerGroupsForSubject<SubjectId, SlotId, GroupListId, StudentId>
{
    type Problem =
        crate::base::ValidatedColloscopeProblem<SubjectId, SlotId, GroupListId, StudentId>;
    type GeneralConstraintDesc = StudentsPerGroupsForSubjectDesc<SubjectId, GroupListId, StudentId>;
    type StructureVariable = ();

    fn is_fit_for_problem(&self, desc: &Self::Problem) -> bool {
        desc.group_list_descriptions
            .contains_key(&self.group_list_id)
            && desc.subject_descriptions.contains_key(&self.subject_id)
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

        let subject_desc = desc
            .subject_descriptions
            .get(&self.subject_id)
            .expect("Subject ID should be valid if this is compatible with the base problem");

        let mut subclasses = BTreeSet::new();
        for group_assignment_opt in &subject_desc.group_assignments {
            let Some(group_assignment) = group_assignment_opt else {
                continue;
            };
            if group_assignment.group_list_id != self.group_list_id {
                continue;
            }

            let subclass = group_assignment.enrolled_students.clone();
            subclasses.insert(subclass);
        }

        for subclass in subclasses {
            for (i, prefilled_group) in group_list_desc.prefilled_groups.iter().enumerate() {
                if prefilled_group.sealed {
                    continue;
                }

                let group = i as u32;

                let mut lhs = LinExpr::constant(0.);
                for student in subclass.iter().copied() {
                    lhs = lhs
                        + LinExpr::var(collomatique_solver::ExtraVariable::BaseStructure(
                            crate::base::variables::StructureVariable::StudentInGroup {
                                group_list: self.group_list_id,
                                student,
                                group,
                            },
                        ));
                }

                let max_count = subject_desc.students_per_group.end().get();
                let students_already_present =
                    prefilled_group.students.intersection(&subclass).count() as u32;
                assert!(students_already_present <= max_count);

                let rhs = LinExpr::constant(f64::from(max_count - students_already_present));
                constraints.push((
                    lhs.leq(&rhs),
                    StudentsPerGroupsForSubjectDesc::AtMostCountStudentsInGroupForSubjectWithSubclass(
                        self.group_list_id,
                        max_count,
                        group,
                        self.subject_id,
                        subclass.clone(),
                    ),
                ));

                let min_count = group_list_desc.students_per_group.start().get();
                if min_count > students_already_present {
                    let rhs = f64::from(min_count - students_already_present)
                        * LinExpr::var(collomatique_solver::ExtraVariable::BaseStructure(
                            crate::base::variables::StructureVariable::NonEmptyGroupForSubClass {
                                subclass: subclass.clone(),
                                group_list: self.group_list_id,
                                group,
                            },
                        ));
                    constraints.push((
                        lhs.geq(&rhs),
                        StudentsPerGroupsForSubjectDesc::AtMinimumCountStudentsInNonEmptyGroupForSubjectWithSubclass(
                            self.group_list_id,
                            min_count,
                            group,
                            self.subject_id,
                            subclass.clone(),
                        ),
                    ));
                }
            }
        }

        constraints
    }
}
