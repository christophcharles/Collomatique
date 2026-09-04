use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssignmentsUpdateOp {
    Assign(
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::SubjectId,
        bool,
    ),
    DuplicatePreviousPeriod(collomatique_state_colloscopes::PeriodId),
    AssignAll(
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::SubjectId,
        bool,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssignmentsUpdateError {
    #[error(transparent)]
    Assign(#[from] AssignError),
    #[error(transparent)]
    DuplicatePreviousPeriod(#[from] DuplicatePreviousPeriodError),
    #[error(transparent)]
    AssignAll(#[from] AssignAllError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssignError {
    /// period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),

    /// subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),

    /// student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),

    /// Subject does not run on given period
    #[error("invalid subject id {0:?} for period {1:?}")]
    SubjectDoesNotRunOnPeriod(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    ),

    /// Student is not present on given period
    #[error("invalid subject id {0:?} for period {1:?}")]
    StudentIsNotPresentOnPeriod(
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::PeriodId,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssignAllError {
    /// period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),

    /// subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),

    /// Subject does not run on given period
    #[error("invalid subject id {0:?} for period {1:?}")]
    SubjectDoesNotRunOnPeriod(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DuplicatePreviousPeriodError {
    /// period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),

    /// trying to override first period
    #[error("given period ({0:?}) is the first period")]
    FirstPeriodHasNoPreviousPeriod(collomatique_state_colloscopes::PeriodId),
}

impl AssignmentsUpdateOp {
    pub(crate) fn apply_to_session<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<(), AssignmentsUpdateError> {
        match self {
            Self::Assign(period_id, student_id, subject_id, status) => {
                // Build the whole target row from the current one — the row the
                // session has *now*, so two Assigns in one composite accumulate
                // instead of overwriting each other. The rest of it came from a
                // valid state, so the only id `SetRow` can reject as invalid is
                // this op's own student.
                let mut new_row = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .students(*period_id, *subject_id)
                    .cloned()
                    .unwrap_or_default();
                if *status {
                    new_row.insert(*student_id);
                } else {
                    new_row.remove(student_id);
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Assignment(
                            collomatique_state_colloscopes::AssignmentOp::SetRow(
                                *period_id,
                                *subject_id,
                                new_row,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            AssignmentPrecheckError, Convergence, Error, FixableInvariant,
                            InvalidOp, PrecheckError, Reference, StudentRefSite,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Assignment(
                                pe,
                            ))) => match pe {
                                AssignmentPrecheckError::InvalidPeriodId(id) => {
                                    AssignError::InvalidPeriodId(*id)
                                }
                                AssignmentPrecheckError::InvalidSubjectId(id) => {
                                    AssignError::InvalidSubjectId(*id)
                                }
                            },
                            // The pre-op state was valid, so any break in the
                            // set was introduced by this Assign — and the
                            // cascade could repair none of them, which is why
                            // this arm still exists. Each of the three names
                            // the row the op just wrote, and the row went back
                            // with the rolled-back op: the map then finds
                            // either no row at all (a subject that does not run
                            // on the period has none in a valid state) or the
                            // *old* row, which holds neither a dead student nor
                            // a student the period excludes. So every arm
                            // answers `None`, the engine convicts the target,
                            // and the scans below turn the break back into the
                            // bad input it came from.
                            //
                            // The dangling-student scan comes first, and that
                            // order is the whole point: the state layer used to
                            // sweep `SetRow`'s payload students in its precheck,
                            // so a dead student was reported *before* the op could
                            // land and no convergence break was ever visible
                            // alongside it. The sweep moved to the FK net (op
                            // *address* is prechecked, op *content* belongs to the
                            // checker), which means both kinds of break can now
                            // arrive in one set — e.g. a dead student on a subject
                            // that does not run on the period. Scanning FK-first
                            // keeps this public error surface exactly what it was.
                            //
                            // The two convergence scans then follow the old
                            // validator order (colloscope_params validate):
                            // subject-not-running before student-not-present.
                            Error::BrokenInvariants(set) => {
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Student {
                                        target,
                                        site: StudentRefSite::AssignmentsStudent { .. },
                                    }) = inv
                                    {
                                        return AssignError::InvalidStudentId(*target);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::AssignmentForSubjectNotRunningOnPeriod(
                                            period,
                                            subject,
                                        ),
                                    ) = inv
                                    {
                                        return AssignError::SubjectDoesNotRunOnPeriod(
                                            *subject, *period,
                                        );
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::AssignedStudentNotPresentForPeriod {
                                            period,
                                            student,
                                            ..
                                        },
                                    ) = inv
                                    {
                                        return AssignError::StudentIsNotPresentOnPeriod(
                                            *student, *period,
                                        );
                                    }
                                }
                                panic!("Unexpected invariant breaks during Assign: {set:?}");
                            }
                            _ => panic!("Unexpected error during Assign: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(())
            }
            Self::DuplicatePreviousPeriod(period_id) => {
                // Ops-level address checks: they decide whether there is
                // anything to issue at all, so they stay here.
                let Some(position) = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                else {
                    return Err(DuplicatePreviousPeriodError::InvalidPeriodId(*period_id).into());
                };

                if position == 0 {
                    return Err(
                        DuplicatePreviousPeriodError::FirstPeriodHasNoPreviousPeriod(*period_id)
                            .into(),
                    );
                }

                let previous_period_id = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .period_id_at(position - 1)
                    .expect("position > 0 checked above");
                let current_period_assignments: std::collections::BTreeMap<_, _> = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .subjects_for_period(*period_id)
                    .map(|(subject_id, students)| (subject_id, students.clone()))
                    .collect();
                let previous_period_assignments: std::collections::BTreeMap<_, _> = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .subjects_for_period(previous_period_id)
                    .map(|(subject_id, students)| (subject_id, students.clone()))
                    .collect();

                let subjects = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .ordered_subject_list
                    .clone();

                let student_map = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .student_map
                    .clone();

                // One SetRow per subject running on both periods: non-excluded
                // students copy the previous period's membership; students
                // excluded from either period keep their current status. The
                // rows themselves decide nothing — the table is sparse, so a
                // period with no enrolments yet has no rows at all; whether a
                // subject runs on a period is `excluded_periods`' business.
                //
                // The reads above are snapshots taken once, and stay right:
                // every iteration writes a different subject's row, so none of
                // them reads what an earlier one wrote.
                for (subject_id, subject) in subjects.iter() {
                    let subject_id = &subject_id;
                    if subject.excluded_periods.contains(period_id)
                        || subject.excluded_periods.contains(&previous_period_id)
                    {
                        continue;
                    }

                    let current_students = current_period_assignments
                        .get(subject_id)
                        .cloned()
                        .unwrap_or_default();
                    let previous_students = previous_period_assignments
                        .get(subject_id)
                        .cloned()
                        .unwrap_or_default();

                    let mut new_row = std::collections::BTreeSet::new();
                    for (student_id, student) in student_map.iter() {
                        let excluded = student.excluded_periods.contains(period_id)
                            || student.excluded_periods.contains(&previous_period_id);
                        let assigned = if excluded {
                            current_students.contains(&student_id)
                        } else {
                            previous_students.contains(&student_id)
                        };
                        if assigned {
                            new_row.insert(student_id);
                        }
                    }

                    // Nothing here can be rejected and nothing can cascade: the
                    // row is keyed on a subject the exclusion check above just
                    // proved runs on this period, and every student it names is
                    // live and, by the exclusion test above, present for the
                    // period.
                    session
                        .apply(
                            collomatique_state_colloscopes::Op::Assignment(
                                collomatique_state_colloscopes::AssignmentOp::SetRow(
                                    *period_id,
                                    *subject_id,
                                    new_row,
                                ),
                            ),
                            self.get_desc(),
                        )
                        .expect("a copied row names only live, present students");
                }

                Ok(())
            }
            Self::AssignAll(period_id, subject_id, status) => {
                if session
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_none()
                {
                    return Err(AssignAllError::InvalidPeriodId(*period_id).into());
                };

                let Some(subject) = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                else {
                    return Err(AssignAllError::InvalidSubjectId(*subject_id).into());
                };

                if subject.excluded_periods.contains(period_id) {
                    return Err(
                        AssignAllError::SubjectDoesNotRunOnPeriod(*subject_id, *period_id).into(),
                    );
                }

                // One SetRow for the whole row: every non-excluded student for
                // `status == true`, the empty set (row removal) for `false` —
                // every assigned student in a valid state is non-excluded, so
                // clearing them all is exactly the row's removal.
                let new_row: std::collections::BTreeSet<_> = if *status {
                    session
                        .get_data()
                        .get_inner_data()
                        .params
                        .students
                        .student_map
                        .iter()
                        .filter(|(_, student)| !student.excluded_periods.contains(period_id))
                        .map(|(student_id, _)| student_id)
                        .collect()
                } else {
                    std::collections::BTreeSet::new()
                };

                // The three prechecks above cover everything this op could
                // break: the row's key is live and running, and the students it
                // names are live and present for the period — the filter is
                // what makes the second half true.
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Assignment(
                            collomatique_state_colloscopes::AssignmentOp::SetRow(
                                *period_id,
                                *subject_id,
                                new_row,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("the row names only live students present for the period");

                assert!(result.is_none());

                Ok(())
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Assignments,
            match self {
                AssignmentsUpdateOp::Assign(_, _, _, status) => {
                    if *status {
                        "Inscrire un élève à une matière".into()
                    } else {
                        "Désinscrire un élève d'une matière".into()
                    }
                }
                AssignmentsUpdateOp::DuplicatePreviousPeriod(_) => {
                    "Dupliquer les inscriptions d'une période".into()
                }
                AssignmentsUpdateOp::AssignAll(_, _, status) => {
                    if *status {
                        "Inscrire tous les élèves à une matière".into()
                    } else {
                        "Désinscrire tous les élèves d'une matière".into()
                    }
                }
            },
        )
    }
}

#[cfg(test)]
mod tests {
    //! An assignments row is the only thing in the document that this family
    //! writes, and a row *points at* three things — a period, a subject and the
    //! students it holds — while nothing points back at a row. So no op here
    //! ever makes the cascade repair anything, and every fixture that lands
    //! something asserts an empty warning log.
    //!
    //! That is not because the row cannot break anything: it can break three
    //! things, and all three are ones the map knows a repair for in general —
    //! [collomatique_state_colloscopes::Fix::ClearAssignmentRow] when a period
    //! or a subject disappears under a settled row, and
    //! [collomatique_state_colloscopes::Fix::RemoveStudentFromAssignmentRow]
    //! when a student does. What keeps them from firing on the user's own bad
    //! payload is the presence test at the head of each arm: a rejected op
    //! changes nothing, so the map sees either no row at all or the *old* row,
    //! which holds neither a dead student nor one the period excludes, and
    //! answers `None`. The engine convicts the target and the scans turn the
    //! break back into the bad input it came from.
    //!
    //! The three composites are read-modify-write, so their fixtures also pin
    //! *which* state they read: `Assign` builds its row from the one the
    //! session has now (two Assigns in a row accumulate), `AssignAll` skips the
    //! students the period excludes, and `DuplicatePreviousPeriod` copies the
    //! previous period's membership except for students either period excludes,
    //! who keep what they have. The last two rules are what makes the two
    //! `.expect`s below true, and the mutations say so plainly: drop either one
    //! and the row it writes breaks a convergence *the cascade cannot repair* —
    //! the break is the op's own payload, so the map answers `None`, the engine
    //! convicts, and the composite dies on its expect rather than warning about
    //! anything. Which is the point: `colloscopes/ops/` may not hand the state layer a row
    //! it knows is wrong and let the repair machinery clean up after it.
    //!
    //! The base is the frozen hogwarts copy: three periods, twenty-four
    //! students, eight subjects, and a full assignments table on every period.
    //! Nothing in it is excluded from anything, so the fixtures that need an
    //! exclusion set one up with the elementary op, in plain sight.

    use super::*;
    use crate::test_utils::{fixes, hogwarts};
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::{
        AssignmentOp, Op, StudentOp, SubjectOp,
        ids::{Id, PeriodId, StudentId, SubjectId},
    };
    use std::collections::BTreeSet;

    fn subject_by_name(data: &Data, name: &str) -> SubjectId {
        data.get_inner_data()
            .params
            .subjects
            .ordered_subject_list
            .iter()
            .find(|(_id, subject)| subject.parameters.name == name)
            .map(|(id, _subject)| id)
            .unwrap_or_else(|| panic!("the fixture should have a subject named {name}"))
    }

    fn student_by_surname(data: &Data, surname: &str, firstname: &str) -> StudentId {
        data.get_inner_data()
            .params
            .students
            .student_map
            .iter()
            .find(|(_id, student)| {
                student.desc.surname == surname && student.desc.firstname == firstname
            })
            .map(|(id, _student)| id)
            .unwrap_or_else(|| {
                panic!("the fixture should have a student named {firstname} {surname}")
            })
    }

    /// The `n`-th period in display order.
    fn period_at(data: &Data, index: usize) -> PeriodId {
        data.get_inner_data()
            .params
            .periods
            .period_ids()
            .nth(index)
            .unwrap_or_else(|| panic!("the fixture should have at least {} periods", index + 1))
    }

    /// The row at `(period, subject)`, absent rows included — an absent row is
    /// « personne n'est inscrit », the canonical form of an empty one.
    fn row(data: &Data, period: PeriodId, subject: SubjectId) -> Option<BTreeSet<StudentId>> {
        data.get_inner_data()
            .params
            .assignments
            .students(period, subject)
            .cloned()
    }

    fn live_row(data: &Data, period: PeriodId, subject: SubjectId) -> BTreeSet<StudentId> {
        row(data, period, subject).expect("the fixture should have a row there")
    }

    fn all_students(data: &Data) -> BTreeSet<StudentId> {
        data.get_inner_data()
            .params
            .students
            .student_map
            .keys()
            .collect()
    }

    /// Ids no document ever issued.
    fn dangling_student() -> StudentId {
        unsafe { StudentId::new(1u64 << 40) }
    }

    fn dangling_period() -> PeriodId {
        unsafe { PeriodId::new(1u64 << 40) }
    }

    fn dangling_subject() -> SubjectId {
        unsafe { SubjectId::new(1u64 << 40) }
    }

    /// Applies a setup op through the cascade and throws away whatever it had
    /// to repair. Excluding a student from a period, for instance, takes them
    /// out of every row they were in — that is the students family's business,
    /// not this one's, and here it is only how the fixture gets the document it
    /// wants to talk about.
    fn seed(state: &mut AppState<Data, Desc>, op: Op) {
        state
            .apply_cascade(op, (OpCategory::None, "Préparation".into()))
            .expect("the fixture's setup op should land");
    }

    /// Marks `student` as absent for `period`, cascading them out of the rows
    /// they were in.
    fn exclude_student(state: &mut AppState<Data, Desc>, student: StudentId, period: PeriodId) {
        let mut updated = state
            .get_data()
            .get_inner_data()
            .params
            .students
            .student_map
            .get(&student)
            .expect("the fixture's student should be live")
            .clone();
        updated.excluded_periods.insert(period);

        seed(state, Op::Student(StudentOp::Update(student, updated)));
    }

    /// Stops `subject` from running on `period`, cascading its row away.
    fn exclude_subject(state: &mut AppState<Data, Desc>, subject: SubjectId, period: PeriodId) {
        let mut updated = state
            .get_data()
            .get_inner_data()
            .params
            .subjects
            .find_subject(subject)
            .expect("the fixture's subject should be live")
            .clone();
        updated.excluded_periods.insert(period);

        seed(state, Op::Subject(SubjectOp::Update(subject, updated)));
    }

    /// Replays `ops` on a clone of `base`: the document a fixture expects,
    /// written as the elementary ops it expects to have landed.
    fn expected_document(base: &AppState<Data, Desc>, ops: Vec<Op>) -> AppState<Data, Desc> {
        let mut expected = base.clone();
        for op in ops {
            expected
                .apply(op, (OpCategory::Assignments, "Expected".into()))
                .expect("each expected op lands in the order the cascade landed it");
        }

        expected
    }

    /// Runs one op alone on `base` and hands back what the document became and
    /// what the cascade had to repair on the way.
    fn apply_alone(
        base: &AppState<Data, Desc>,
        op: &AssignmentsUpdateOp,
    ) -> (AppState<Data, Desc>, Vec<CascadeWarning>) {
        let mut session = CascadeSession::new(base.clone());
        op.apply_to_session(&mut session)
            .unwrap_or_else(|e| panic!("{op:?} should land, got {e:?}"));

        session.commit(op.get_desc())
    }

    /// The plain case: a student who was not in the row joins it, and nothing
    /// in the document has to move for that.
    #[test]
    fn assigning_a_student_adds_them_to_the_row_and_warns_about_nothing() {
        let base = hogwarts();
        let arithmancie = subject_by_name(base.get_data(), "Arithmancie");
        let first_period = period_at(base.get_data(), 0);
        let harry = student_by_surname(base.get_data(), "Potter", "Harry");

        let before = live_row(base.get_data(), first_period, arithmancie);
        assert!(
            !before.contains(&harry),
            "the fixture's Harry should not take Arithmancie yet"
        );
        let mut after = before.clone();
        after.insert(harry);

        let op = AssignmentsUpdateOp::Assign(first_period, harry, arithmancie, true);
        let (state, warnings) = apply_alone(&base, &op);

        // Said once in the vocabulary every other family's fixtures use — the
        // repairs, read back through [crate::CascadeWarning::fix]. The fixtures
        // below say the same thing the short way.
        assert_eq!(fixes(&warnings), vec![], "nothing to repair");
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::Assignment(AssignmentOp::SetRow(
                    first_period,
                    arithmancie,
                    after
                ))],
            )
            .get_data(),
        );
    }

    /// Unassignments one student at a time, all in one session. Every one of
    /// them rebuilds the *whole* row, so each has to read the row its
    /// predecessor left — hence the assert halfway through, where all but one
    /// student are gone: an op reading the composite's pre-state instead would
    /// leave the five it did not name behind, and an op starting from an empty
    /// row would already have wiped the last one. The final op empties the row,
    /// which the state layer stores as no row at all.
    #[test]
    fn unassigning_students_one_by_one_reads_the_row_the_previous_op_left() {
        let base = hogwarts();
        let quidditch = subject_by_name(base.get_data(), "Entrainement de Quidditch");
        let first_period = period_at(base.get_data(), 0);

        let members = live_row(base.get_data(), first_period, quidditch);
        assert!(
            members.len() > 2,
            "the point of this fixture is several ops in one session"
        );
        let last_one_standing = *members.first().expect("the row is not empty");

        let mut session = CascadeSession::new(base.clone());
        for student in members.iter().filter(|s| **s != last_one_standing) {
            AssignmentsUpdateOp::Assign(first_period, *student, quidditch, false)
                .apply_to_session(&mut session)
                .expect("unassigning a student who is in the row cannot fail");
        }

        assert_eq!(
            live_row(session.get_data(), first_period, quidditch),
            BTreeSet::from([last_one_standing]),
            "each op should have shrunk the row its predecessor left"
        );

        AssignmentsUpdateOp::Assign(first_period, last_one_standing, quidditch, false)
            .apply_to_session(&mut session)
            .expect("unassigning the last student cannot fail either");
        let (state, warnings) =
            session.commit((OpCategory::Assignments, "Vider le Quidditch".into()));

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            row(state.get_data(), first_period, quidditch),
            None,
            "an emptied row is stored as no row at all"
        );
        // The six ops the composite issued shrink the row one student at a
        // time; the document they leave is the one the last of them wrote.
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::Assignment(AssignmentOp::SetRow(
                    first_period,
                    quidditch,
                    BTreeSet::new()
                ))],
            )
            .get_data(),
        );
    }

    /// The row's *address* is the state layer's precheck, and it is the one
    /// thing the dangling-FK net is structurally blind to: with an empty
    /// payload nothing lands in the document for it to see.
    #[test]
    fn a_dead_period_or_subject_is_rejected_by_assign() {
        let base = hogwarts();
        let arithmancie = subject_by_name(base.get_data(), "Arithmancie");
        let first_period = period_at(base.get_data(), 0);
        let harry = student_by_surname(base.get_data(), "Potter", "Harry");

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            AssignmentsUpdateOp::Assign(dangling_period(), harry, arithmancie, true)
                .apply_to_session(&mut session)
                .unwrap_err(),
            AssignmentsUpdateError::Assign(AssignError::InvalidPeriodId(dangling_period())),
        );
        assert_eq!(
            AssignmentsUpdateOp::Assign(first_period, harry, dangling_subject(), true)
                .apply_to_session(&mut session)
                .unwrap_err(),
            AssignmentsUpdateError::Assign(AssignError::InvalidSubjectId(dangling_subject())),
        );

        assert_eq!(session.get_data(), base.get_data());
        let (_state, warnings) = session.commit((OpCategory::Assignments, "Rien".into()));
        assert!(warnings.is_empty(), "nothing was applied: {warnings:?}");
    }

    /// A made-up student is *content*, not address: the row lands, the checker
    /// answers `DanglingFk @ AssignmentsStudent`, and the scan turns it back
    /// into the error this surface has always produced.
    #[test]
    fn a_dead_student_is_rejected_by_assign() {
        let base = hogwarts();
        let arithmancie = subject_by_name(base.get_data(), "Arithmancie");
        let first_period = period_at(base.get_data(), 0);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            AssignmentsUpdateOp::Assign(first_period, dangling_student(), arithmancie, true)
                .apply_to_session(&mut session)
                .unwrap_err(),
            AssignmentsUpdateError::Assign(AssignError::InvalidStudentId(dangling_student())),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// Assigning to a subject the period does not run. The map's arm for that
    /// break asks whether there is a row at `(period, subject)` — after the
    /// rollback there is none, since a valid document never has one for a
    /// subject that does not run there — so it answers `None` and the op is
    /// convicted rather than quietly cleared away.
    #[test]
    fn a_subject_that_does_not_run_on_the_period_is_rejected() {
        let mut base = hogwarts();
        let quidditch = subject_by_name(base.get_data(), "Entrainement de Quidditch");
        let last_period = period_at(base.get_data(), 2);
        let harry = student_by_surname(base.get_data(), "Potter", "Harry");
        exclude_subject(&mut base, quidditch, last_period);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            AssignmentsUpdateOp::Assign(last_period, harry, quidditch, true)
                .apply_to_session(&mut session)
                .unwrap_err(),
            AssignmentsUpdateError::Assign(AssignError::SubjectDoesNotRunOnPeriod(
                quidditch,
                last_period
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// Assigning a student the period excludes. Same shape, one level deeper:
    /// the map's arm looks for the student *in* the row, and the row it finds
    /// after the rollback is the old one, which does not hold them.
    #[test]
    fn a_student_absent_from_the_period_is_rejected() {
        let mut base = hogwarts();
        let arithmancie = subject_by_name(base.get_data(), "Arithmancie");
        let last_period = period_at(base.get_data(), 2);
        let ginny = student_by_surname(base.get_data(), "Weasley", "Ginny");
        exclude_student(&mut base, ginny, last_period);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            AssignmentsUpdateOp::Assign(last_period, ginny, arithmancie, true)
                .apply_to_session(&mut session)
                .unwrap_err(),
            AssignmentsUpdateError::Assign(AssignError::StudentIsNotPresentOnPeriod(
                ginny,
                last_period
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// Which break wins when a payload carries several is public API, and here
    /// it is more than a matter of taste: the dangling-student scan runs
    /// *first* because the state layer used to settle that case in its precheck,
    /// before any convergence break was visible. Since the payload sweep moved
    /// to the FK net both kinds arrive in one set, and only the scan order keeps
    /// the surface what it was. `colloscopes/ops/tests/assignments_error_surface.rs` pins
    /// the same thing on the old path; this is its twin on the new one.
    #[test]
    fn a_payload_naming_several_bad_things_reports_them_in_the_old_order() {
        let mut base = hogwarts();
        let quidditch = subject_by_name(base.get_data(), "Entrainement de Quidditch");
        let last_period = period_at(base.get_data(), 2);
        let ginny = student_by_surname(base.get_data(), "Weasley", "Ginny");
        exclude_subject(&mut base, quidditch, last_period);
        exclude_student(&mut base, ginny, last_period);

        let mut session = CascadeSession::new(base.clone());

        // A dead student on a subject that does not run: both breaks land in
        // one set, and the student wins.
        assert_eq!(
            AssignmentsUpdateOp::Assign(last_period, dangling_student(), quidditch, true)
                .apply_to_session(&mut session)
                .unwrap_err(),
            AssignmentsUpdateError::Assign(AssignError::InvalidStudentId(dangling_student())),
        );

        // A live student the period excludes, on that same subject: the
        // subject wins, which is the old validator's order
        // (subject-not-running before student-not-present).
        assert_eq!(
            AssignmentsUpdateOp::Assign(last_period, ginny, quidditch, true)
                .apply_to_session(&mut session)
                .unwrap_err(),
            AssignmentsUpdateError::Assign(AssignError::SubjectDoesNotRunOnPeriod(
                quidditch,
                last_period
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// `AssignAll(.., true)` writes the whole row at once — every student the
    /// period does not exclude. The exclusion filter is what keeps the row
    /// valid, and its mutation shows how little slack there is: without it the
    /// row names a student the period excludes,
    /// `AssignedStudentNotPresentForPeriod` breaks, and the cascade *cannot*
    /// clean up after the op — the break is the op's own payload, so the map
    /// answers `None` and the whole thing is convicted, which the composite's
    /// `.expect` turns into a crash.
    #[test]
    fn assign_all_fills_the_row_with_every_student_the_period_does_not_exclude() {
        let mut base = hogwarts();
        let arithmancie = subject_by_name(base.get_data(), "Arithmancie");
        let first_period = period_at(base.get_data(), 0);
        let ginny = student_by_surname(base.get_data(), "Weasley", "Ginny");
        exclude_student(&mut base, ginny, first_period);

        let mut expected_row = all_students(base.get_data());
        expected_row.remove(&ginny);
        assert!(
            live_row(base.get_data(), first_period, arithmancie).len() < expected_row.len(),
            "the fixture's Arithmancie row should not already hold everybody"
        );

        let op = AssignmentsUpdateOp::AssignAll(first_period, arithmancie, true);
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::Assignment(AssignmentOp::SetRow(
                    first_period,
                    arithmancie,
                    expected_row
                ))],
            )
            .get_data(),
        );
    }

    /// `AssignAll(.., false)` is the row's removal: every assigned student in a
    /// valid state is one the period does not exclude, so clearing them all
    /// leaves nothing behind.
    #[test]
    fn assign_all_false_removes_the_row() {
        let base = hogwarts();
        let arithmancie = subject_by_name(base.get_data(), "Arithmancie");
        let first_period = period_at(base.get_data(), 0);

        let op = AssignmentsUpdateOp::AssignAll(first_period, arithmancie, false);
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(row(state.get_data(), first_period, arithmancie), None);
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::Assignment(AssignmentOp::SetRow(
                    first_period,
                    arithmancie,
                    BTreeSet::new()
                ))],
            )
            .get_data(),
        );
    }

    /// `AssignAll`'s three checks are all ops-level: they decide whether there
    /// is an op to issue at all, so they are answered here and the state layer
    /// never sees the op. The third one is the interesting one — it is the
    /// same mistake the `Assign` fixture above lets the checker catch, refused
    /// one layer earlier because this composite would otherwise write a row for
    /// a subject that does not run.
    #[test]
    fn assign_all_refuses_a_dead_period_a_dead_subject_and_a_subject_that_does_not_run() {
        let mut base = hogwarts();
        let quidditch = subject_by_name(base.get_data(), "Entrainement de Quidditch");
        let arithmancie = subject_by_name(base.get_data(), "Arithmancie");
        let first_period = period_at(base.get_data(), 0);
        let last_period = period_at(base.get_data(), 2);
        exclude_subject(&mut base, quidditch, last_period);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            AssignmentsUpdateOp::AssignAll(dangling_period(), arithmancie, true)
                .apply_to_session(&mut session)
                .unwrap_err(),
            AssignmentsUpdateError::AssignAll(AssignAllError::InvalidPeriodId(dangling_period())),
        );
        assert_eq!(
            AssignmentsUpdateOp::AssignAll(first_period, dangling_subject(), true)
                .apply_to_session(&mut session)
                .unwrap_err(),
            AssignmentsUpdateError::AssignAll(AssignAllError::InvalidSubjectId(dangling_subject())),
        );
        assert_eq!(
            AssignmentsUpdateOp::AssignAll(last_period, quidditch, true)
                .apply_to_session(&mut session)
                .unwrap_err(),
            AssignmentsUpdateError::AssignAll(AssignAllError::SubjectDoesNotRunOnPeriod(
                quidditch,
                last_period
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// The composite: every subject that has a row on the target period takes
    /// the previous period's membership. Hogwarts has the same table on all
    /// three periods, so the fixture first makes the first period different —
    /// otherwise the copy would be invisible.
    #[test]
    fn duplicate_previous_period_copies_the_previous_periods_rows() {
        let mut base = hogwarts();
        let arithmancie = subject_by_name(base.get_data(), "Arithmancie");
        let first_period = period_at(base.get_data(), 0);
        let second_period = period_at(base.get_data(), 1);
        let harry = student_by_surname(base.get_data(), "Potter", "Harry");
        let hermione = student_by_surname(base.get_data(), "Granger", "Hermione");

        // Harry joins Arithmancie on the first period, Hermione leaves it.
        let mut changed = live_row(base.get_data(), first_period, arithmancie);
        changed.insert(harry);
        changed.remove(&hermione);
        seed(
            &mut base,
            Op::Assignment(AssignmentOp::SetRow(
                first_period,
                arithmancie,
                changed.clone(),
            )),
        );
        assert_ne!(
            live_row(base.get_data(), second_period, arithmancie),
            changed,
            "the two periods should differ before the copy"
        );

        let op = AssignmentsUpdateOp::DuplicatePreviousPeriod(second_period);
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        // Only Arithmancie moves: the seven other subjects already had the
        // first period's membership, so their rows are rewritten to what they
        // already were.
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::Assignment(AssignmentOp::SetRow(
                    second_period,
                    arithmancie,
                    changed
                ))],
            )
            .get_data(),
        );
    }

    /// The reported bug: duplicating into a period with no enrolments yet —
    /// the state a fresh period or a newly added subject is in — copied
    /// nothing. The table is sparse, a row with nobody in it is absent, and
    /// the composite iterated the *target* period's rows to decide what to
    /// copy; an absent row was skipped, so the one situation the button
    /// exists for was the one it did nothing in.
    #[test]
    fn duplicate_previous_period_fills_a_row_the_target_period_does_not_have_yet() {
        let mut base = hogwarts();
        let arithmancie = subject_by_name(base.get_data(), "Arithmancie");
        let first_period = period_at(base.get_data(), 0);
        let second_period = period_at(base.get_data(), 1);

        // Nobody is enrolled in Arithmancie on the second period: no row.
        seed(
            &mut base,
            Op::Assignment(AssignmentOp::SetRow(
                second_period,
                arithmancie,
                BTreeSet::new(),
            )),
        );
        assert_eq!(
            row(base.get_data(), second_period, arithmancie),
            None,
            "an emptied row is stored as no row at all"
        );

        let op = AssignmentsUpdateOp::DuplicatePreviousPeriod(second_period);
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        let expected_row = live_row(base.get_data(), first_period, arithmancie);
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::Assignment(AssignmentOp::SetRow(
                    second_period,
                    arithmancie,
                    expected_row
                ))],
            )
            .get_data(),
        );
    }

    /// The mirror image: duplication makes the target period match the
    /// previous one, so a subject with nobody enrolled on the previous period
    /// gets its target row cleared, not left as it was.
    #[test]
    fn duplicate_previous_period_clears_a_row_the_previous_period_does_not_have() {
        let mut base = hogwarts();
        let arithmancie = subject_by_name(base.get_data(), "Arithmancie");
        let first_period = period_at(base.get_data(), 0);
        let second_period = period_at(base.get_data(), 1);

        // Nobody is enrolled in Arithmancie on the first period: no row.
        seed(
            &mut base,
            Op::Assignment(AssignmentOp::SetRow(
                first_period,
                arithmancie,
                BTreeSet::new(),
            )),
        );
        assert!(
            row(base.get_data(), second_period, arithmancie).is_some(),
            "the target period should still have a row to clear"
        );

        let op = AssignmentsUpdateOp::DuplicatePreviousPeriod(second_period);
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::Assignment(AssignmentOp::SetRow(
                    second_period,
                    arithmancie,
                    BTreeSet::new()
                ))],
            )
            .get_data(),
        );
    }

    /// The exclusion rule: a student either period excludes keeps whatever
    /// status they have instead of copying the previous period's. Copying
    /// blindly would put Ginny — who is not there for the second period — back
    /// into the first row it writes, and that row would be refused outright:
    /// the break is the copied row's own doing, so the map answers `None`, the
    /// engine convicts, and the composite dies on its `.expect`. The rule is
    /// not a nicety, it is what makes that expect true.
    #[test]
    fn duplicate_previous_period_keeps_the_current_status_of_an_excluded_student() {
        let mut base = hogwarts();
        let quidditch = subject_by_name(base.get_data(), "Entrainement de Quidditch");
        let first_period = period_at(base.get_data(), 0);
        let second_period = period_at(base.get_data(), 1);
        let harry = student_by_surname(base.get_data(), "Potter", "Harry");
        let ginny = student_by_surname(base.get_data(), "Weasley", "Ginny");

        // Ginny drops out of the second period entirely; the cascade takes her
        // out of the rows she was in there.
        exclude_student(&mut base, ginny, second_period);
        // And Harry leaves the second period's Quidditch, so the copy has
        // something visible to restore.
        let mut without_harry = live_row(base.get_data(), second_period, quidditch);
        without_harry.remove(&harry);
        seed(
            &mut base,
            Op::Assignment(AssignmentOp::SetRow(
                second_period,
                quidditch,
                without_harry,
            )),
        );
        assert!(
            live_row(base.get_data(), first_period, quidditch).contains(&ginny),
            "Ginny should still be in the first period's row — the one being copied"
        );

        let op = AssignmentsUpdateOp::DuplicatePreviousPeriod(second_period);
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        let mut expected_row = live_row(base.get_data(), first_period, quidditch);
        expected_row.remove(&ginny);
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::Assignment(AssignmentOp::SetRow(
                    second_period,
                    quidditch,
                    expected_row
                ))],
            )
            .get_data(),
        );
    }

    /// The other half of the loop: a subject excluded from the target period
    /// is skipped. It has to be — writing a row there would break
    /// `AssignmentForSubjectNotRunningOnPeriod`. Here too the cascade is no
    /// safety net: the offending row is the one the op just wrote, so the map
    /// answers `None` and the composite dies on its `.expect` instead.
    #[test]
    fn duplicate_previous_period_leaves_alone_a_subject_that_does_not_run() {
        let mut base = hogwarts();
        let quidditch = subject_by_name(base.get_data(), "Entrainement de Quidditch");
        let first_period = period_at(base.get_data(), 0);
        let second_period = period_at(base.get_data(), 1);
        exclude_subject(&mut base, quidditch, second_period);

        assert!(
            row(base.get_data(), first_period, quidditch).is_some(),
            "the previous period should still run Quidditch — that is what could be copied"
        );

        let op = AssignmentsUpdateOp::DuplicatePreviousPeriod(second_period);
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(row(state.get_data(), second_period, quidditch), None);
        assert_eq!(state.get_data(), base.get_data());
    }

    /// The other skip: a subject excluded from the *previous* period has
    /// nothing to copy from, so its target row stays as it is instead of
    /// being cleared.
    #[test]
    fn duplicate_previous_period_leaves_alone_a_subject_excluded_from_the_previous_period() {
        let mut base = hogwarts();
        let quidditch = subject_by_name(base.get_data(), "Entrainement de Quidditch");
        let first_period = period_at(base.get_data(), 0);
        let second_period = period_at(base.get_data(), 1);
        exclude_subject(&mut base, quidditch, first_period);

        assert!(
            row(base.get_data(), second_period, quidditch).is_some(),
            "the target period should still run Quidditch, row intact"
        );

        let op = AssignmentsUpdateOp::DuplicatePreviousPeriod(second_period);
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(state.get_data(), base.get_data());
    }

    /// The composite's own two prechecks. The first period has no previous one
    /// to copy, which is a refusal rather than a no-op: the user asked for
    /// something that does not exist.
    #[test]
    fn duplicate_previous_period_refuses_a_dead_period_and_the_first_one() {
        let base = hogwarts();
        let first_period = period_at(base.get_data(), 0);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            AssignmentsUpdateOp::DuplicatePreviousPeriod(dangling_period())
                .apply_to_session(&mut session)
                .unwrap_err(),
            AssignmentsUpdateError::DuplicatePreviousPeriod(
                DuplicatePreviousPeriodError::InvalidPeriodId(dangling_period())
            ),
        );
        assert_eq!(
            AssignmentsUpdateOp::DuplicatePreviousPeriod(first_period)
                .apply_to_session(&mut session)
                .unwrap_err(),
            AssignmentsUpdateError::DuplicatePreviousPeriod(
                DuplicatePreviousPeriodError::FirstPeriodHasNoPreviousPeriod(first_period)
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
        let (_state, warnings) = session.commit((OpCategory::Assignments, "Rien".into()));
        assert!(warnings.is_empty(), "nothing was applied: {warnings:?}");
    }
}
