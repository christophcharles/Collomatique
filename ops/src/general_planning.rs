use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GeneralPlanningUpdateOp {
    DeleteFirstWeek,
    UpdateFirstWeek(collomatique_time::WeekStart),
    AddNewPeriod(usize),
    UpdatePeriodWeekCount(collomatique_state_colloscopes::PeriodId, usize),
    /// Delete a period *and all its weeks*
    ///
    /// The weeks are removed first (authored — the user asked for them to go,
    /// so they generate no warnings), then the period itself.
    DeletePeriodAndWeeks(collomatique_state_colloscopes::PeriodId),
    CutPeriod(collomatique_state_colloscopes::PeriodId, usize),
    MergeWithPreviousPeriod(collomatique_state_colloscopes::PeriodId),
    UpdateWeekStatus(collomatique_state_colloscopes::PeriodId, usize, bool),
    UpdateWeekAnnotation(
        collomatique_state_colloscopes::PeriodId,
        usize,
        Option<non_empty_string::NonEmptyString>,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum GeneralPlanningUpdateError {
    #[error(transparent)]
    UpdatePeriodWeekCount(#[from] UpdatePeriodWeekCountError),
    #[error(transparent)]
    DeletePeriodAndWeeks(#[from] DeletePeriodAndWeeksError),
    #[error(transparent)]
    CutPeriod(#[from] CutPeriodError),
    #[error(transparent)]
    MergeWithPreviousPeriod(#[from] MergeWithPreviousPeriodError),
    #[error(transparent)]
    UpdateWeekStatus(#[from] UpdateWeekStatusError),
    #[error(transparent)]
    UpdateWeekAnnotation(#[from] UpdateWeekAnnotationError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdatePeriodWeekCountError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeletePeriodAndWeeksError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum CutPeriodError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("Remaining week count ({0}) is larger than available week count ({1})")]
    RemainingWeekCountTooBig(usize, usize),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum MergeWithPreviousPeriodError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("This is the first period and cannot be merged with the non-existent previous one")]
    NoPreviousPeriodToMergeWith,
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateWeekStatusError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("Week number {0} is larger that the number of available weeks ({1})")]
    InvalidWeekNumber(usize, usize),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateWeekAnnotationError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("Week number {0} is larger that the number of available weeks ({1})")]
    InvalidWeekNumber(usize, usize),
}

impl GeneralPlanningUpdateOp {
    pub(crate) fn apply_to_session<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<Option<collomatique_state_colloscopes::PeriodId>, GeneralPlanningUpdateError> {
        match self {
            GeneralPlanningUpdateOp::DeleteFirstWeek => {
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::ChangeStartDate(None),
                        ),
                        self.get_desc(),
                    )
                    // The calendar's start date is a field nothing else in the
                    // document reads: no reference site holds it and no
                    // convergence predicate mentions it. There is no precheck
                    // either — the op carries no id.
                    .expect("the start date contradicts nothing");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
            GeneralPlanningUpdateOp::UpdateFirstWeek(date) => {
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::ChangeStartDate(Some(
                                date.clone(),
                            )),
                        ),
                        self.get_desc(),
                    )
                    // Same as above: setting the date contradicts as little as
                    // clearing it.
                    .expect("the start date contradicts nothing");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
            GeneralPlanningUpdateOp::AddNewPeriod(week_count) => {
                // Create the period empty, then grow it one week at a time so
                // the week ops are the sole authority on week data.
                let last_period = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .period_ids()
                    .last();

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Period(match last_period {
                            Some(id) => collomatique_state_colloscopes::PeriodOp::AddAfter(id),
                            None => collomatique_state_colloscopes::PeriodOp::AddFront,
                        }),
                        self.get_desc(),
                    )
                    // A brand new period is empty and nobody names it yet: no
                    // week belongs to it, no exclusion set mentions it, no
                    // assignment row and no association is keyed on it. The
                    // anchor is the period list's own last period, read one
                    // line above.
                    .expect("a period nothing names yet contradicts nothing");
                let new_id = match result {
                    Some(collomatique_state_colloscopes::NewId::PeriodId(id)) => id,
                    _ => panic!("Unexpected result! {:?}", result),
                };

                let mut prev_week_id: Option<collomatique_state_colloscopes::WeekId> = None;
                for _ in 0..*week_count {
                    let week_desc = collomatique_state_colloscopes::weeks::WeekDesc::new(true);
                    let week_op = match prev_week_id {
                        None => collomatique_state_colloscopes::WeekOp::AddFront(new_id, week_desc),
                        Some(prev) => {
                            collomatique_state_colloscopes::WeekOp::AddAfter(prev, week_desc)
                        }
                    };
                    let result = session
                        .apply(
                            collomatique_state_colloscopes::Op::Week(week_op),
                            self.get_desc(),
                        )
                        // The period was created one moment ago and the anchor
                        // week by the previous turn of this loop, so both ids
                        // are live; and a fresh week carries no colle, so no
                        // interrogation predicate has anything to look at.
                        .expect("a week nothing names yet contradicts nothing");
                    match result {
                        Some(collomatique_state_colloscopes::NewId::WeekId(id)) => {
                            prev_week_id = Some(id)
                        }
                        _ => panic!("Unexpected result! {:?}", result),
                    }
                }

                Ok(Some(new_id))
            }
            GeneralPlanningUpdateOp::UpdatePeriodWeekCount(period_id, week_count) => {
                session
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .ok_or(UpdatePeriodWeekCountError::InvalidPeriodId(*period_id))?;
                let old_week_count = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .weeks
                    .week_count_for_period(*period_id)
                    .unwrap_or(0);

                if *week_count > old_week_count {
                    // Grow: append weeks off the last one (front if the period
                    // is empty), copying the last week's description — the shape
                    // the old whole-period update produced via `Vec::resize`.
                    let fill_desc = session
                        .get_data()
                        .get_inner_data()
                        .params
                        .weeks
                        .weeks_desc_vec_for_period(*period_id)
                        .unwrap_or_default()
                        .last()
                        .cloned()
                        .unwrap_or(collomatique_state_colloscopes::weeks::WeekDesc::new(true));

                    let mut prev_week_id = if old_week_count == 0 {
                        None
                    } else {
                        session
                            .get_data()
                            .get_inner_data()
                            .params
                            .weeks
                            .week_id_at(*period_id, old_week_count - 1)
                    };

                    for _ in old_week_count..*week_count {
                        let week_op = match prev_week_id {
                            None => collomatique_state_colloscopes::WeekOp::AddFront(
                                *period_id,
                                fill_desc.clone(),
                            ),
                            Some(prev) => collomatique_state_colloscopes::WeekOp::AddAfter(
                                prev,
                                fill_desc.clone(),
                            ),
                        };
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Week(week_op),
                                self.get_desc(),
                            )
                            // As in `AddNewPeriod`: the period is checked just
                            // above, the anchor week comes out of the document
                            // (or out of the previous turn), and a week nobody
                            // has written a colle on yet contradicts nothing.
                            .expect("a week nothing names yet contradicts nothing");
                        match result {
                            Some(collomatique_state_colloscopes::NewId::WeekId(id)) => {
                                prev_week_id = Some(id)
                            }
                            _ => panic!("Unexpected result! {:?}", result),
                        }
                    }
                } else if *week_count < old_week_count {
                    // Shrink: drop the tail weeks last-to-first. Each removal
                    // leaves the colloscope cells written on that week and the
                    // week-pattern bits excluding it dangling, and the cascade
                    // clears both — the two cleaning scans the old body ran here
                    // are exactly those two repairs.
                    for pos in (*week_count..old_week_count).rev() {
                        let week_id = session
                            .get_data()
                            .get_inner_data()
                            .params
                            .weeks
                            .week_id_at(*period_id, pos)
                            .expect("position in range");
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Week(
                                    collomatique_state_colloscopes::WeekOp::Remove(week_id),
                                ),
                                self.get_desc(),
                            )
                            .expect("the cascade resolves everything a week removal breaks");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                Ok(None)
            }
            GeneralPlanningUpdateOp::DeletePeriodAndWeeks(period_id) => {
                // Empty the period one week at a time, then remove it. The week
                // removals are *authored*: the user asked for the weeks to go,
                // so no « la semaine X sera supprimée » fix may show up in the
                // warning log — only what each removal cascades on its own
                // (colloscope cells, week-pattern bits), which is the genuinely
                // surprising part. A bare `PeriodOp::Remove` would take the
                // weeks with it too (the state layer has no week-empty guard,
                // and a dangling `Week::period_id` is a fixable reference), but
                // it would say so one warning per week.
                if let Some(week_count) = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .weeks
                    .week_count_for_period(*period_id)
                {
                    for pos in (0..week_count).rev() {
                        let week_id = session
                            .get_data()
                            .get_inner_data()
                            .params
                            .weeks
                            .week_id_at(*period_id, pos)
                            .expect("position in range");
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Week(
                                    collomatique_state_colloscopes::WeekOp::Remove(week_id),
                                ),
                                self.get_desc(),
                            )
                            .expect("the cascade resolves everything a week removal breaks");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::Remove(*period_id),
                        ),
                        self.get_desc(),
                    )
                    // ★ D13: a dead period id used to die on an `.expect` here,
                    // reachable straight from the Python API — the error
                    // variant existed and was never constructed. Everything
                    // else a period removal leaves behind (the exclusion sets
                    // naming it, its assignment rows, its group-list
                    // associations) is pre-existing material the cascade
                    // repairs, so the eight-phase cleaning the old body ran
                    // first has no successor here.
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, InvalidOp, PeriodPrecheckError, PrecheckError,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Period(pe))) => {
                                match pe {
                                    PeriodPrecheckError::InvalidPeriodId(id) => {
                                        DeletePeriodAndWeeksError::InvalidPeriodId(*id)
                                    }
                                    // Only the `Add` branches of
                                    // `force_apply_period` answer this, and only
                                    // on an annotated id: a removal cannot
                                    // clobber anything.
                                    PeriodPrecheckError::PeriodIdAlreadyExists(_) => panic!(
                                        "Unexpected PeriodPrecheckError during \
                                         DeletePeriodAndWeeks: {e:?}"
                                    ),
                                }
                            }
                            _ => panic!("Unexpected error during DeletePeriodAndWeeks: {e:?}"),
                        }
                    })?;

                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }

                Ok(None)
            }
            GeneralPlanningUpdateOp::CutPeriod(period_id, new_week_count) => {
                session
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .ok_or(CutPeriodError::InvalidPeriodId(*period_id))?;
                let old_week_count = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .weeks
                    .week_count_for_period(*period_id)
                    .unwrap_or(0);

                if *new_week_count > old_week_count {
                    Err(CutPeriodError::RemainingWeekCountTooBig(
                        *new_week_count,
                        old_week_count,
                    ))?;
                }

                // Create the tail period empty; the tail weeks are moved into it
                // below. Content (colloscope cells + week-pattern bits) travels
                // with each week, so no save/clean/restore dance is needed.
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::AddAfter(*period_id),
                        ),
                        self.get_desc(),
                    )
                    // The anchor is checked just above, and an empty period
                    // nobody names yet contradicts nothing.
                    .expect("a period nothing names yet contradicts nothing");
                let new_id = match result {
                    Some(collomatique_state_colloscopes::NewId::PeriodId(id)) => id,
                    _ => panic!("Unexpected result! {:?}", result),
                };

                // Propagate period-level references to the new period *before*
                // moving weeks: a cell that travels is read against its new
                // period's context — which subjects run there, which group list
                // bounds its groups — and the cascade would *clear* it if that
                // context were not in place yet. This is the frame rule read
                // forwards: the loops below plan against the pre-state, and
                // nothing a cascade can answer here creates or removes a
                // subject, a student or a group list.
                let ordered_subject_list = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .ordered_subject_list
                    .clone();
                for (subject_id, subject) in ordered_subject_list.iter() {
                    if subject.excluded_periods.contains(period_id) {
                        let mut new_subject = subject.clone();
                        new_subject.excluded_periods.insert(new_id);
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Subject(
                                    collomatique_state_colloscopes::SubjectOp::Update(
                                        subject_id,
                                        new_subject,
                                    ),
                                ),
                                self.get_desc(),
                            )
                            // The only thing the payload changes is one more
                            // period in the excluded set, and that period is
                            // one nothing points at yet: it holds no week, no
                            // assignment row and no association, so none of the
                            // three predicates that read an exclusion has
                            // anything at that coordinate to complain about.
                            .expect("excluding a subject from an empty period contradicts nothing");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let student_map = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .student_map
                    .clone();
                for (student_id, student) in student_map.iter() {
                    if student.excluded_periods.contains(period_id) {
                        let mut new_student = student.clone();
                        new_student.excluded_periods.insert(new_id);
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Student(
                                    collomatique_state_colloscopes::StudentOp::Update(
                                        student_id,
                                        new_student,
                                    ),
                                ),
                                self.get_desc(),
                            )
                            // Same argument: the new period holds no assignment
                            // row yet, so the one predicate reading a student
                            // exclusion has no row to find them in.
                            .expect("excluding a student from an empty period contradicts nothing");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let period_assignments: Vec<(_, std::collections::BTreeSet<_>)> = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .subjects_for_period(*period_id)
                    .map(|(subject_id, students)| (subject_id, students.clone()))
                    .collect();

                for (subject_id, assigned_students) in period_assignments {
                    let result = session
                        .apply(
                            collomatique_state_colloscopes::Op::Assignment(
                                collomatique_state_colloscopes::AssignmentOp::SetRow(
                                    new_id,
                                    subject_id,
                                    assigned_students,
                                ),
                            ),
                            self.get_desc(),
                        )
                        // The row is a copy of a live one, and the two
                        // predicates reading a row — the subject must run on the
                        // period, every listed student must be present for it —
                        // are satisfied at the new coordinate exactly because
                        // they are satisfied at the old one: the two loops above
                        // gave the new period the very same exclusions.
                        .expect("a copied row is as valid at the new period as at the old one");

                    if result.is_some() {
                        panic!("Unexpected result! {:?}", result);
                    }
                }

                let period_associations: Vec<(_, _)> = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .subjects_associations
                    .iter()
                    .filter_map(|((period, subject), group_list)| {
                        (period == *period_id).then_some((subject, *group_list))
                    })
                    .collect();
                for (subject_id, group_list_id) in period_associations {
                    let result = session
                        .apply(
                            collomatique_state_colloscopes::Op::GroupList(
                                collomatique_state_colloscopes::GroupListOp::AssignToSubject(
                                    new_id,
                                    subject_id,
                                    Some(group_list_id),
                                ),
                            ),
                            self.get_desc(),
                        )
                        // Likewise a copy of a live entry: the subject holds
                        // interrogations and runs on the new period for the same
                        // reason it does on the old one. And the colles the new
                        // bound could leave out of range are the ones on the new
                        // period's weeks — of which there are none yet.
                        .expect(
                            "a copied association is as valid at the new period as at the old one",
                        );
                    if result.is_some() {
                        panic!("Unexpected result! {:?}", result);
                    }
                }

                // Move the tail weeks into the new period, preserving order.
                // Detaching each week from the source automatically shortens it;
                // week ids are stable, so capture them before the first move.
                let tail_week_ids: Vec<collomatique_state_colloscopes::WeekId> = (*new_week_count
                    ..old_week_count)
                    .map(|pos| {
                        session
                            .get_data()
                            .get_inner_data()
                            .params
                            .weeks
                            .week_id_at(*period_id, pos)
                            .expect("tail week exists")
                    })
                    .collect();
                for (dest_pos, week_id) in tail_week_ids.into_iter().enumerate() {
                    let result = session
                        .apply(
                            collomatique_state_colloscopes::Op::Week(
                                collomatique_state_colloscopes::WeekOp::Move(
                                    week_id, new_id, dest_pos,
                                ),
                            ),
                            self.get_desc(),
                        )
                        // The destination is the period created above and the
                        // position is the loop's own counter, so the two
                        // prechecks hold. What the move rewrites is the week's
                        // period, which its colles are read against — and the
                        // new period was given the old one's exclusions and
                        // associations precisely so that nothing about them
                        // changes. A cut loses no colle.
                        .expect("the new period was prepared so the moved weeks keep their colles");
                    if result.is_some() {
                        panic!("Unexpected result! {:?}", result);
                    }
                }

                Ok(Some(new_id))
            }
            GeneralPlanningUpdateOp::MergeWithPreviousPeriod(period_id) => {
                let pos = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .ok_or(MergeWithPreviousPeriodError::InvalidPeriodId(*period_id))?;
                if pos == 0 {
                    Err(MergeWithPreviousPeriodError::NoPreviousPeriodToMergeWith)?;
                }

                let previous_id = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .period_id_at(pos - 1)
                    .expect("pos > 0 checked above");

                // Append every week of this period to the end of the previous
                // one, preserving order (the two periods are neighbours, so the
                // global week order does not move and no week pattern changes
                // meaning). Content travels with each week — this is where the
                // step's first divergence lands: the old body reconciled the two
                // periods *before* the move and erased every cell it could not
                // carry, and its own comment admitted it. Now a cell survives
                // unless the surviving period's context genuinely invalidates
                // it, and the cascade clears exactly those. Detaching from the
                // source shortens it; week ids are stable, so capture them
                // before the first move.
                let append_start = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .weeks
                    .week_count_for_period(previous_id)
                    .unwrap_or(0);
                let week_count = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .weeks
                    .week_count_for_period(*period_id)
                    .unwrap_or(0);
                let week_ids: Vec<collomatique_state_colloscopes::WeekId> = (0..week_count)
                    .map(|pos| {
                        session
                            .get_data()
                            .get_inner_data()
                            .params
                            .weeks
                            .week_id_at(*period_id, pos)
                            .expect("week exists")
                    })
                    .collect();
                for (offset, week_id) in week_ids.into_iter().enumerate() {
                    let result = session
                        .apply(
                            collomatique_state_colloscopes::Op::Week(
                                collomatique_state_colloscopes::WeekOp::Move(
                                    week_id,
                                    previous_id,
                                    append_start + offset,
                                ),
                            ),
                            self.get_desc(),
                        )
                        // The destination is read off the period list and the
                        // position is the append point plus the loop's counter,
                        // so the two prechecks hold. Everything else a move can
                        // contradict is a colle already written on the week —
                        // its slot's subject may not run on the surviving
                        // period, or the group list there may have fewer groups
                        // — and each of those the cascade repairs on the cell,
                        // which the rolled-back move leaves right where it is.
                        .expect(
                            "the cascade repairs whatever colles the surviving period invalidates",
                        );
                    if result.is_some() {
                        panic!("Unexpected result! {:?}", result);
                    }
                }

                // The weeks are out; what is left on the dying period is the
                // material keyed on it, which the removal below drops with one
                // warning each. Those warnings are only worth reading when the
                // drop changes what the merged document says — so the entries
                // the surviving period repeats are dropped here instead,
                // silently. See the helper for why this cannot run any earlier.
                self.drop_material_the_previous_period_repeats(session, previous_id, *period_id);

                // The emptied period goes, and with it everything that was keyed
                // on it *and not repeated by the period it merged into*: the six
                // phases of the old reconcile-with-previous cleaning are
                // replaced by that removal's own cascade.
                let result = GeneralPlanningUpdateOp::DeletePeriodAndWeeks(*period_id)
                    .apply_to_session(session)
                    // The period was found at the top of this arm, and nothing
                    // between here and there can have removed it: no fix
                    // removes a period.
                    .expect("the period this arm merged away is still there to delete");

                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }

                Ok(None)
            }
            GeneralPlanningUpdateOp::UpdateWeekStatus(period_id, week_num, state) => {
                session
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .ok_or(UpdateWeekStatusError::InvalidPeriodId(*period_id))?;
                let desc = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .weeks
                    .weeks_desc_vec_for_period(*period_id)
                    .unwrap_or_default();

                if *week_num >= desc.len() {
                    Err(UpdateWeekStatusError::InvalidWeekNumber(
                        *week_num,
                        desc.len(),
                    ))?;
                }

                let week_id = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .weeks
                    .week_id_at(*period_id, *week_num)
                    .expect("week number checked in range above");
                let mut new_desc = desc[*week_num].clone();
                new_desc.interrogations = *state;

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Week(
                            collomatique_state_colloscopes::WeekOp::Update(week_id, new_desc),
                        ),
                        self.get_desc(),
                    )
                    // Turning the colles off on a week contradicts the colles
                    // already written there — the checker's
                    // `InterrogationOnInactiveWeek` — and those cells were
                    // already in the document, so the cascade clears them. That
                    // is the scan the old body ran here.
                    .expect("the cascade clears the colles an inactive week can no longer hold");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
            GeneralPlanningUpdateOp::UpdateWeekAnnotation(period_id, week_num, annotation) => {
                session
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .ok_or(UpdateWeekAnnotationError::InvalidPeriodId(*period_id))?;
                let desc = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .weeks
                    .weeks_desc_vec_for_period(*period_id)
                    .unwrap_or_default();

                if *week_num >= desc.len() {
                    Err(UpdateWeekAnnotationError::InvalidWeekNumber(
                        *week_num,
                        desc.len(),
                    ))?;
                }

                let week_id = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .weeks
                    .week_id_at(*period_id, *week_num)
                    .expect("week number checked in range above");
                let mut new_desc = desc[*week_num].clone();
                new_desc.annotation = annotation.clone();

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Week(
                            collomatique_state_colloscopes::WeekOp::Update(week_id, new_desc),
                        ),
                        self.get_desc(),
                    )
                    // A week's annotation is free text nothing else reads.
                    .expect("an annotation contradicts nothing");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
        }
    }

    /// Drops from `merged` every period-keyed entry the surviving `previous`
    /// period already repeats, *authored* — so the period removal that follows
    /// finds nothing left to warn about there.
    ///
    /// A period is named from exactly seven sites — the
    /// [collomatique_state_colloscopes::refs::PeriodRefSite] variants. One is
    /// its weeks, which the merge has already moved by the time this runs; the
    /// six others are dropped by the removal below, one warning each. Those
    /// warnings are worth reading when the drop *changes what the document says
    /// about the moved weeks*, and noise when it does not: an association the
    /// surviving period repeats bounds the moved colles exactly as it did
    /// before, and an exclusion the surviving period repeats keeps them just as
    /// excluded. So the repeated ones are dropped here, silently, and only the
    /// rest reach the user.
    ///
    /// Every op below is the very op the matching fix would have applied
    /// (`Fix::to_annotated_op`), so the merged document is exactly what it was
    /// before this existed. Only the warning list gets shorter.
    ///
    /// **This must run after the week move.** With the weeks still on the dying
    /// period, clearing an association takes the group bound of every colle
    /// written on them to zero and the cascade empties the cells one by one.
    /// Once the period is empty, nothing reads its material and none of these
    /// ops cascades anything.
    fn drop_material_the_previous_period_repeats<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
        previous: collomatique_state_colloscopes::PeriodId,
        merged: collomatique_state_colloscopes::PeriodId,
    ) {
        use collomatique_state_colloscopes::{
            AssignmentOp, GroupListOp, Op, PairingOp, SlotPairingOp, StudentOp, SubjectOp,
        };

        // ---- Subjects excluded from both periods.
        let subjects: Vec<_> = session
            .get_data()
            .get_inner_data()
            .params
            .subjects
            .ordered_subject_list
            .iter()
            .filter(|(_id, subject)| {
                subject.excluded_periods.contains(&merged)
                    && subject.excluded_periods.contains(&previous)
            })
            .map(|(id, subject)| (id, subject.clone()))
            .collect();
        for (subject_id, subject) in subjects {
            let mut rebuilt = subject;
            rebuilt.excluded_periods.remove(&merged);
            let result = session
                .apply(
                    Op::Subject(SubjectOp::Update(subject_id, rebuilt)),
                    self.get_desc(),
                )
                // Lifting an exclusion only ever permits more: each of the three
                // predicates that read a subject's excluded periods — its
                // assignment rows, its associations, the colles on its slots —
                // fires on the exclusion being *there*.
                .expect("lifting a subject's period exclusion contradicts nothing");
            if result.is_some() {
                panic!("Unexpected result! {:?}", result);
            }
        }

        // ---- Students absent from both periods.
        let students: Vec<_> = session
            .get_data()
            .get_inner_data()
            .params
            .students
            .student_map
            .iter()
            .filter(|(_id, student)| {
                student.excluded_periods.contains(&merged)
                    && student.excluded_periods.contains(&previous)
            })
            .map(|(id, student)| (id, student.clone()))
            .collect();
        for (student_id, student) in students {
            let mut rebuilt = student;
            rebuilt.excluded_periods.remove(&merged);
            let result = session
                .apply(
                    Op::Student(StudentOp::Update(student_id, rebuilt)),
                    self.get_desc(),
                )
                // Same argument: the one predicate reading a student's excluded
                // periods asks whether an assignment row lists them on a period
                // they skip, which lifting the exclusion can only settle.
                .expect("lifting a student's period exclusion contradicts nothing");
            if result.is_some() {
                panic!("Unexpected result! {:?}", result);
            }
        }

        // ---- Pairing rules disabled on both periods.
        let pairing_rules: Vec<_> = session
            .get_data()
            .get_inner_data()
            .params
            .pairings
            .pairing_rule_map
            .iter()
            .filter(|(_id, rule)| {
                rule.excluded_periods().contains(&merged)
                    && rule.excluded_periods().contains(&previous)
            })
            .map(|(id, rule)| (id, rule.clone()))
            .collect();
        for (rule_id, rule) in pairing_rules {
            // Sealed value: `into_parts` is the door for callers that rebuild,
            // and `PairingRule::new`'s only failure is the two parts naming one
            // subject — which dropping a period cannot cause.
            let (antecedent, consequent, mut excluded_periods, soft) = rule.into_parts();
            excluded_periods.remove(&merged);
            let rebuilt = collomatique_state_colloscopes::pairings::PairingRule::new(
                antecedent,
                consequent,
                excluded_periods,
                soft,
            )
            .expect("removing an excluded period cannot make the parts share a subject");
            let result = session
                .apply(
                    Op::Pairing(PairingOp::Update(rule_id, rebuilt)),
                    self.get_desc(),
                )
                // A pairing rule's excluded periods are named by no convergence
                // predicate at all — only by the reference sweep, and the period
                // they name is still there.
                .expect("a pairing rule's excluded periods contradict nothing");
            if result.is_some() {
                panic!("Unexpected result! {:?}", result);
            }
        }

        // ---- Slot pairing rules disabled on both periods.
        let slot_pairing_rules: Vec<_> = session
            .get_data()
            .get_inner_data()
            .params
            .slot_pairings
            .slot_pairing_rule_map
            .iter()
            .filter(|(_id, rule)| {
                rule.excluded_periods().contains(&merged)
                    && rule.excluded_periods().contains(&previous)
            })
            .map(|(id, rule)| (id, rule.clone()))
            .collect();
        for (rule_id, rule) in slot_pairing_rules {
            let (antecedent, consequent, mut excluded_periods, soft) = rule.into_parts();
            excluded_periods.remove(&merged);
            let rebuilt = collomatique_state_colloscopes::slot_pairings::SlotPairingRule::new(
                antecedent,
                consequent,
                excluded_periods,
                soft,
            )
            .expect("removing an excluded period cannot make the parts share a slot");
            let result = session
                .apply(
                    Op::SlotPairing(SlotPairingOp::Update(rule_id, rebuilt)),
                    self.get_desc(),
                )
                .expect("a slot pairing rule's excluded periods contradict nothing");
            if result.is_some() {
                panic!("Unexpected result! {:?}", result);
            }
        }

        // ---- Assignment rows the surviving period's own row already covers.
        // Subset, not equality: what makes the warning noise is that no student
        // loses their inscription, and a surviving row holding *more* students
        // loses none either.
        let rows: Vec<_> = {
            let assignments = &session.get_data().get_inner_data().params.assignments;
            assignments
                .subjects_for_period(merged)
                .filter(|(subject_id, students)| {
                    assignments
                        .students(previous, *subject_id)
                        .is_some_and(|kept| students.is_subset(kept))
                })
                .map(|(subject_id, _students)| subject_id)
                .collect()
        };
        for subject_id in rows {
            let result = session
                .apply(
                    Op::Assignment(AssignmentOp::SetRow(
                        merged,
                        subject_id,
                        std::collections::BTreeSet::new(),
                    )),
                    self.get_desc(),
                )
                // Rows are canonical-absent, so an empty set removes this one.
                // Both predicates that read a row need the row to be there.
                .expect("clearing an assignment row contradicts nothing");
            if result.is_some() {
                panic!("Unexpected result! {:?}", result);
            }
        }

        // ---- Associations the surviving period repeats, same group list.
        let associations: Vec<_> = {
            let subjects_associations = &session
                .get_data()
                .get_inner_data()
                .params
                .group_lists
                .subjects_associations;
            subjects_associations
                .iter()
                .filter_map(|((period, subject), group_list)| {
                    (period == merged
                        && subjects_associations.get(&(previous, subject)) == Some(group_list))
                    .then_some(subject)
                })
                .collect()
        };
        for subject_id in associations {
            let result = session
                .apply(
                    Op::GroupList(GroupListOp::AssignToSubject(merged, subject_id, None)),
                    self.get_desc(),
                )
                // The one thing an association bounds is the colles on its
                // period's weeks — and this period has none left.
                .expect("unassigning on a period with no week left contradicts nothing");
            if result.is_some() {
                panic!("Unexpected result! {:?}", result);
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::GeneralPlanning,
            match self {
                GeneralPlanningUpdateOp::DeleteFirstWeek => "Effacer le début des colles".into(),
                GeneralPlanningUpdateOp::UpdateFirstWeek(_date) => {
                    "Changer le début des colles".into()
                }
                GeneralPlanningUpdateOp::AddNewPeriod(_week_count) => "Ajouter une période".into(),
                GeneralPlanningUpdateOp::UpdatePeriodWeekCount(_period_id, _week_count) => {
                    "Modifier une période".into()
                }
                GeneralPlanningUpdateOp::DeletePeriodAndWeeks(_period_id) => {
                    "Supprimer une période".into()
                }
                GeneralPlanningUpdateOp::CutPeriod(_period_id, _new_week_count) => {
                    "Découper une période".into()
                }
                GeneralPlanningUpdateOp::MergeWithPreviousPeriod(_period_id) => {
                    "Fusionner deux périodes".into()
                }
                GeneralPlanningUpdateOp::UpdateWeekStatus(_period_id, _week_num, state) => {
                    if *state {
                        "Ajouter une semaine de colle".into()
                    } else {
                        "Supprimer une semaine de colle".into()
                    }
                }
                GeneralPlanningUpdateOp::UpdateWeekAnnotation(
                    _period_id,
                    _week_num,
                    annotation,
                ) => {
                    if annotation.is_some() {
                        "Annoter une semaine de colle".into()
                    } else {
                        "Effacer l'annotation d'une semaine de colle".into()
                    }
                }
            },
        )
    }
}

#[cfg(test)]
mod tests {
    //! The calendar is the floor every other family stands on. A period holds
    //! weeks; a week holds the colles written on it and is named by the
    //! patterns that skip it; and the period itself is a key — the assignment
    //! rows, the group-list associations and the exclusion sets of subjects,
    //! students and pairing rules all say « on this period ». So the ops here
    //! are the ones with the widest blast radius, and they are also the ones
    //! whose old bodies cleaned the most: eight phases before a period could be
    //! deleted, six more before two of them could be merged.
    //!
    //! What replaces all of it is the ordinary cascade, plus one deliberate
    //! choice: **the composites author their own week removals**. A bare
    //! `PeriodOp::Remove` would take the weeks with it anyway — the state layer
    //! has no week-empty guard and a dangling `Week::period_id` is a fixable
    //! reference — but it would say so, one « la semaine X sera supprimée » per
    //! week. The user who asked to delete a period asked for its weeks; the
    //! surprising part is only what each week takes down with *it*, and that is
    //! what the fixtures below read in the warning log.
    //!
    //! Two divergences from the old world land here, and each has its fixture.
    //! Merging two periods now **keeps the colloscope**: the weeks travel with
    //! their colles and only the genuinely invalidated ones are repaired, where
    //! the old body reconciled the two periods first and erased what it could
    //! not carry (`docs/todos/fixme_ops.md`). And deleting a period on a dead
    //! id **returns an error** instead of killing the process — the variant
    //! existed all along and was never constructed (★ D13).
    //!
    //! The frozen hogwarts base carries three periods of 4, 14 and 22 weeks,
    //! two week patterns that split every week between them — « Semaines
    //! paires » skips the odd ones, « Semaines impaires » the even ones, so
    //! removing *any* week frees exactly one exclusion — eight assignment rows
    //! and six group-list associations per period. It carries no colloscope at
    //! all, so every fixture about a colle writes it first, in plain sight at
    //! its head.

    use super::*;
    use crate::test_utils::{fixes, hogwarts};
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::{
        AssignmentOp, ColloscopeOp, Fix, GroupListOp, NewId, Op, PairingOp, PeriodOp,
        SlotPairingOp, StudentOp, Subject, SubjectOp, WeekOp, WeekPatternOp,
        ids::{
            Id, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId, SubjectId, WeekId,
            WeekPatternId,
        },
        pairings::{PairingRule, RulePart},
        slot_pairings::SlotPairingRule,
        students::Student,
        week_patterns::WeekPattern,
        weeks::WeekDesc,
    };
    use std::collections::{BTreeMap, BTreeSet};

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

    fn student_by_name(data: &Data, surname: &str, firstname: &str) -> StudentId {
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

    fn subject_of(data: &Data, subject: SubjectId) -> Subject {
        data.get_inner_data()
            .params
            .subjects
            .find_subject(subject)
            .expect("the fixture's subject should be live")
            .clone()
    }

    fn student_of(data: &Data, student: StudentId) -> Student {
        data.get_inner_data()
            .params
            .students
            .student_map
            .get(&student)
            .expect("the fixture's student should be live")
            .clone()
    }

    fn period_at(data: &Data, index: usize) -> PeriodId {
        data.get_inner_data()
            .params
            .periods
            .period_ids()
            .nth(index)
            .unwrap_or_else(|| panic!("the fixture should have at least {} periods", index + 1))
    }

    fn week_at(data: &Data, period: PeriodId, position: usize) -> WeekId {
        data.get_inner_data()
            .params
            .weeks
            .week_id_at(period, position)
            .unwrap_or_else(|| panic!("the period should have a week at position {position}"))
    }

    fn week_count(data: &Data, period: PeriodId) -> usize {
        data.get_inner_data()
            .params
            .weeks
            .week_count_for_period(period)
            .unwrap_or(0)
    }

    fn week_descs(data: &Data, period: PeriodId) -> Vec<WeekDesc> {
        data.get_inner_data()
            .params
            .weeks
            .weeks_desc_vec_for_period(period)
            .unwrap_or_default()
    }

    /// The subject's slots, in display order.
    fn slots_of_subject(data: &Data, subject: SubjectId) -> Vec<SlotId> {
        data.get_inner_data()
            .params
            .slots
            .slots_for_subject(subject)
            .into_iter()
            .flatten()
            .map(|(id, _slot)| *id)
            .collect()
    }

    /// The weeks of `period` a colle may be written on for `slot`, in global
    /// week order.
    fn writable_weeks(data: &Data, slot: SlotId, period: PeriodId) -> Vec<WeekId> {
        let params = &data.get_inner_data().params;
        params
            .week_ids()
            .filter(|week| {
                params.weeks.week_position(*week).map(|(p, _pos)| p) == Some(period)
                    && params.is_interrogation_possible(slot, *week)
            })
            .collect()
    }

    /// The eight subjects of the base, in **id** order — which is the order the
    /// cascade meets a period's rows, both `AssignmentsKey` and
    /// `AssociationEntry` carrying the subject inside the reference site. The
    /// first six are the ones a group list is associated with.
    fn subjects_in_id_order(data: &Data) -> [SubjectId; 8] {
        let subjects = [
            subject_by_name(data, "Potions"),
            subject_by_name(data, "Défense contre les forces du Mal"),
            subject_by_name(data, "Métamorphose"),
            subject_by_name(data, "Arithmancie"),
            subject_by_name(data, "Divination"),
            subject_by_name(data, "Potions - TP"),
            subject_by_name(data, "Déjeuner à la Grande Salle"),
            subject_by_name(data, "Entrainement de Quidditch"),
        ];
        assert!(
            subjects.windows(2).all(|pair| pair[0] < pair[1]),
            "the fixture's subjects should be listed in id order, got {subjects:?}"
        );

        subjects
    }

    /// What a period removal leaves behind on the base once its weeks are gone:
    /// its eight assignment rows, then its six group-list associations — the
    /// canonical order, `AssignmentsKey` being declared before
    /// `AssociationEntry` and each site carrying the subject id it sorts on.
    fn period_scoped_fixes(data: &Data, period: PeriodId) -> Vec<Fix> {
        let subjects = subjects_in_id_order(data);
        let assignments = &data.get_inner_data().params.assignments;
        let associations = &data
            .get_inner_data()
            .params
            .group_lists
            .subjects_associations;
        for (index, subject) in subjects.iter().enumerate() {
            assert!(
                assignments.students(period, *subject).is_some(),
                "every subject of the base should hold an assignment row on the period"
            );
            assert_eq!(
                associations.contains(&(period, *subject)),
                index < 6,
                "the base should associate a group list with its first six subjects only"
            );
        }

        let mut fixes: Vec<Fix> = subjects
            .iter()
            .map(|&subject| Fix::ClearAssignmentRow { period, subject })
            .collect();
        fixes.extend(
            subjects[..6]
                .iter()
                .map(|&subject| Fix::UnassignGroupList { period, subject }),
        );

        fixes
    }

    /// The elementary ops those fixes translate to, followed by the removal
    /// they clear the way for.
    fn period_removal_ops(data: &Data, period: PeriodId) -> Vec<Op> {
        let subjects = subjects_in_id_order(data);
        let mut ops: Vec<Op> = subjects
            .iter()
            .map(|&subject| Op::Assignment(AssignmentOp::SetRow(period, subject, BTreeSet::new())))
            .collect();
        ops.extend(
            subjects[..6]
                .iter()
                .map(|&subject| Op::GroupList(GroupListOp::AssignToSubject(period, subject, None))),
        );
        ops.push(Op::Period(PeriodOp::Remove(period)));

        ops
    }

    /// The pattern that skips `week` — the base's two patterns share every week
    /// between them, so there is exactly one.
    fn pattern_skipping(data: &Data, week: WeekId) -> WeekPatternId {
        let mut found: Vec<WeekPatternId> = data
            .get_inner_data()
            .params
            .week_patterns
            .week_pattern_map
            .iter()
            .filter(|(_id, pattern)| pattern.excluded_weeks.contains(&week))
            .map(|(id, _pattern)| id)
            .collect();
        assert_eq!(
            found.len(),
            1,
            "exactly one of the base's two patterns should skip each week"
        );

        found.pop().expect("checked non-empty just above")
    }

    fn pattern_of(data: &Data, pattern: WeekPatternId) -> WeekPattern {
        data.get_inner_data()
            .params
            .week_patterns
            .week_pattern_map
            .get(&pattern)
            .expect("the fixture's pattern should be live")
            .clone()
    }

    /// Removing the weeks at `positions` of `period`, in the order given: the
    /// repairs the cascade answers (one freed pattern exclusion per week) and
    /// the elementary ops they translate to, interleaved with the removals
    /// themselves.
    ///
    /// The rebuilt payloads are threaded: a pattern that loses two weeks is
    /// rebuilt twice, the second time off the first rebuild.
    fn week_removal_effects(
        data: &Data,
        period: PeriodId,
        positions: impl IntoIterator<Item = usize>,
    ) -> (Vec<Fix>, Vec<Op>) {
        let mut patterns: BTreeMap<WeekPatternId, WeekPattern> = BTreeMap::new();
        let mut expected_fixes = Vec::new();
        let mut expected_ops = Vec::new();

        for position in positions {
            let week = week_at(data, period, position);
            let pattern = pattern_skipping(data, week);
            let rebuilt = patterns
                .entry(pattern)
                .or_insert_with(|| pattern_of(data, pattern));
            rebuilt.excluded_weeks.remove(&week);

            expected_fixes.push(Fix::RemoveWeekPatternExclusion {
                pattern,
                week,
                rebuilt: rebuilt.clone(),
            });
            expected_ops.push(Op::WeekPattern(WeekPatternOp::Update(
                pattern,
                rebuilt.clone(),
            )));
            expected_ops.push(Op::Week(WeekOp::Remove(week)));
        }

        (expected_fixes, expected_ops)
    }

    /// An id no document ever issued.
    fn dangling_period() -> PeriodId {
        unsafe { PeriodId::new(1u64 << 40) }
    }

    /// Replays `ops` on a clone of `base`: the document a fixture expects,
    /// written as the elementary ops it expects the composite to have landed —
    /// each of them valid in that order, exactly as the cascade lands them.
    fn expected_document(base: &AppState<Data, Desc>, ops: Vec<Op>) -> AppState<Data, Desc> {
        let mut expected = base.clone();
        for op in ops {
            expected
                .apply(op, (OpCategory::GeneralPlanning, "Expected".into()))
                .expect("each expected op lands in the order the cascade landed it");
        }

        expected
    }

    /// Applies one preparation op to the base a fixture builds on.
    fn prepare(base: &mut AppState<Data, Desc>, op: Op) {
        base.apply(
            op.clone(),
            (OpCategory::GeneralPlanning, "Préparation".into()),
        )
        .unwrap_or_else(|e| panic!("the preparation op {op:?} should land, got {e:?}"));
    }

    /// Applies one preparation op that creates something, and hands back the id
    /// it issued.
    fn prepare_new(base: &mut AppState<Data, Desc>, op: Op) -> NewId {
        base.apply(
            op.clone(),
            (OpCategory::GeneralPlanning, "Préparation".into()),
        )
        .unwrap_or_else(|e| panic!("the preparation op {op:?} should land, got {e:?}"))
        .unwrap_or_else(|| panic!("the preparation op {op:?} should have issued an id"))
    }

    /// Runs one op alone on `base` and hands back what the document became,
    /// what the cascade had to repair on the way, and the id the op issued.
    fn apply_alone(
        base: &AppState<Data, Desc>,
        op: &GeneralPlanningUpdateOp,
    ) -> (AppState<Data, Desc>, Vec<CascadeWarning>, Option<PeriodId>) {
        let mut session = CascadeSession::new(base.clone());
        let new_id = op
            .apply_to_session(&mut session)
            .unwrap_or_else(|e| panic!("{op:?} should land, got {e:?}"));
        let (state, warnings) = session.commit(op.get_desc());

        (state, warnings, new_id)
    }

    /// The start date is a field nothing else in the document reads: setting it
    /// and clearing it both land alone.
    #[test]
    fn the_first_week_moves_and_disappears_without_a_word() {
        let base = hogwarts();

        let date = collomatique_time::WeekStart::new(
            chrono::NaiveDate::from_ymd_opt(1995, 9, 11).expect("a real Monday"),
        )
        .expect("a Monday is a valid week start");
        let op = GeneralPlanningUpdateOp::UpdateFirstWeek(date.clone());
        let (moved, warnings, new_id) = apply_alone(&base, &op);

        assert!(new_id.is_none());
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            moved.get_data(),
            expected_document(
                &base,
                vec![Op::Period(PeriodOp::ChangeStartDate(Some(date)))],
            )
            .get_data(),
        );

        let op = GeneralPlanningUpdateOp::DeleteFirstWeek;
        let (cleared, warnings, _new_id) = apply_alone(&moved, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            cleared.get_data(),
            expected_document(&moved, vec![Op::Period(PeriodOp::ChangeStartDate(None))]).get_data(),
        );
    }

    /// A period is created empty and grown one week at a time. Nothing names it
    /// yet and no colle is written on its weeks, so the whole composite lands
    /// in silence — and the id it issued comes back for the caller.
    #[test]
    fn adding_a_period_hands_back_its_id_and_warns_about_nothing() {
        let base = hogwarts();
        let last = period_at(base.get_data(), 2);

        let op = GeneralPlanningUpdateOp::AddNewPeriod(3);
        let (state, warnings, new_id) = apply_alone(&base, &op);

        let new_id = new_id.expect("adding a period returns the id it issued");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(period_at(state.get_data(), 3), new_id);
        assert_eq!(week_count(state.get_data(), new_id), 3);

        // The weeks are spliced one off the other, so the expected op list
        // walks the new period's weeks in order and anchors each on the one
        // before it.
        let mut ops = vec![Op::Period(PeriodOp::AddAfter(last))];
        let mut previous = None;
        for position in 0..3 {
            ops.push(Op::Week(match previous {
                None => WeekOp::AddFront(new_id, WeekDesc::new(true)),
                Some(prev) => WeekOp::AddAfter(prev, WeekDesc::new(true)),
            }));
            previous = Some(week_at(state.get_data(), new_id, position));
        }

        assert_eq!(state.get_data(), expected_document(&base, ops).get_data());
    }

    /// Growing a period appends weeks copied off its last one — the second
    /// period's is « Vacances de Noël », colles off, so the copy is visible.
    /// They carry no colle and no pattern skips them, so again there is
    /// nothing to repair.
    #[test]
    fn growing_a_period_copies_its_last_week_and_warns_about_nothing() {
        let base = hogwarts();
        let period = period_at(base.get_data(), 1);
        let old_week_count = week_count(base.get_data(), period);
        let last_desc = week_descs(base.get_data(), period)
            .last()
            .expect("the second period has weeks")
            .clone();
        assert!(
            !last_desc.interrogations && last_desc.annotation.is_some(),
            "the fixture's last week should be a distinctive one, got {last_desc:?}"
        );
        let last_week = week_at(base.get_data(), period, old_week_count - 1);

        let op = GeneralPlanningUpdateOp::UpdatePeriodWeekCount(period, old_week_count + 2);
        let (state, warnings, _new_id) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(week_count(state.get_data(), period), old_week_count + 2);
        let appended = week_at(state.get_data(), period, old_week_count);
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![
                    Op::Week(WeekOp::AddAfter(last_week, last_desc.clone())),
                    Op::Week(WeekOp::AddAfter(appended, last_desc)),
                ],
            )
            .get_data(),
        );
    }

    /// Shrinking drops the tail weeks last-to-first. Each of them is skipped by
    /// one of the base's two patterns, and that exclusion is what the cascade
    /// frees — the second of the two cleaning scans the old body ran here. The
    /// first one, the colloscope cells, is the fixture below.
    #[test]
    fn shrinking_a_period_frees_the_pattern_exclusions_of_the_weeks_it_drops() {
        let base = hogwarts();
        let period = period_at(base.get_data(), 0);
        let (expected_fixes, expected_ops) = week_removal_effects(base.get_data(), period, [3, 2]);

        let op = GeneralPlanningUpdateOp::UpdatePeriodWeekCount(period, 2);
        let (state, warnings, _new_id) = apply_alone(&base, &op);

        assert_eq!(fixes(&warnings), expected_fixes);
        assert_eq!(week_count(state.get_data(), period), 2);
        assert_eq!(
            state.get_data(),
            expected_document(&base, expected_ops).get_data(),
        );
    }

    /// The other half of a shrink: a colle written on a dropped week goes with
    /// it. The cell is the fixture's own setup — hogwarts has no colloscope —
    /// and it is repaired *before* the pattern exclusion, a dangling
    /// `ColloscopeInterrogation` and a dangling `WeekPatternExcludedWeek` being
    /// two sites of the same dead week, the pattern one declared first.
    #[test]
    fn shrinking_a_period_takes_the_colles_written_on_the_dropped_weeks() {
        let mut base = hogwarts();
        let period = period_at(base.get_data(), 0);
        let subject = subject_by_name(base.get_data(), "Potions");
        let slot = slots_of_subject(base.get_data(), subject)[0];
        let week = *writable_weeks(base.get_data(), slot, period)
            .last()
            .expect("Potions runs on the first period");
        prepare(
            &mut base,
            Op::Colloscope(ColloscopeOp::SetInterrogation(
                slot,
                week,
                BTreeSet::from([0]),
            )),
        );
        let position = week_descs(base.get_data(), period)
            .len()
            .checked_sub(1)
            .expect("the period has weeks");
        assert_eq!(
            week_at(base.get_data(), period, position),
            week,
            "the fixture writes its colle on the period's last week"
        );

        let pattern = pattern_skipping(base.get_data(), week);
        let mut freed = pattern_of(base.get_data(), pattern);
        freed.excluded_weeks.remove(&week);

        let op = GeneralPlanningUpdateOp::UpdatePeriodWeekCount(period, position);
        let (state, warnings, _new_id) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![
                Fix::RemoveWeekPatternExclusion {
                    pattern,
                    week,
                    rebuilt: freed.clone(),
                },
                Fix::ClearInterrogationCell { slot, week },
            ],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![
                    Op::WeekPattern(WeekPatternOp::Update(pattern, freed)),
                    Op::Colloscope(ColloscopeOp::SetInterrogation(slot, week, BTreeSet::new())),
                    Op::Week(WeekOp::Remove(week)),
                ],
            )
            .get_data(),
        );
    }

    /// The eight-phase cleaning of the old body, read as one warning list. The
    /// weeks go first and *silently* — the user asked for them — but each of
    /// them frees a pattern exclusion, and the period itself takes down its
    /// eight assignment rows and its six group-list associations.
    #[test]
    fn deleting_a_period_takes_its_weeks_silently_and_everything_keyed_on_it_loudly() {
        let base = hogwarts();
        let period = period_at(base.get_data(), 0);
        let (mut expected_fixes, mut expected_ops) =
            week_removal_effects(base.get_data(), period, (0..4).rev());
        expected_fixes.extend(period_scoped_fixes(base.get_data(), period));
        expected_ops.extend(period_removal_ops(base.get_data(), period));

        let op = GeneralPlanningUpdateOp::DeletePeriodAndWeeks(period);
        let (state, warnings, _new_id) = apply_alone(&base, &op);

        assert_eq!(fixes(&warnings), expected_fixes);
        assert!(
            state
                .get_data()
                .get_inner_data()
                .params
                .periods
                .find_period_position(period)
                .is_none(),
            "the period should be gone"
        );
        assert_eq!(
            state.get_data(),
            expected_document(&base, expected_ops).get_data(),
        );
    }

    /// ★ D13. The variant was there from the start and nothing ever built it:
    /// the arm had no precheck and the dead id died on « All data should be
    /// valid at this point », one Python call away. Now the state layer's own
    /// precheck is translated, and a rejected op leaves the session untouched.
    #[test]
    fn deleting_a_period_that_does_not_exist_is_an_error_not_a_crash() {
        let base = hogwarts();
        let dangling = dangling_period();

        let mut session = CascadeSession::new(base.clone());
        assert_eq!(
            GeneralPlanningUpdateOp::DeletePeriodAndWeeks(dangling)
                .apply_to_session(&mut session)
                .unwrap_err(),
            GeneralPlanningUpdateError::DeletePeriodAndWeeks(
                DeletePeriodAndWeeksError::InvalidPeriodId(dangling)
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
        let (_state, warnings) = session.commit((OpCategory::GeneralPlanning, "Rien".into()));
        assert!(warnings.is_empty(), "nothing was applied: {warnings:?}");
    }

    /// Cutting hands the tail weeks to a fresh period — and hands them over
    /// *with their colles*. The new period is given the old one's exclusions
    /// and associations before the first week moves, which is exactly what
    /// makes the moved cell still legal at its new coordinate: nothing is
    /// repaired, so nothing is warned.
    #[test]
    fn cutting_a_period_carries_the_tail_weeks_and_their_colles_across() {
        let mut base = hogwarts();
        let period = period_at(base.get_data(), 1);
        let subject = subject_by_name(base.get_data(), "Potions");
        let slot = slots_of_subject(base.get_data(), subject)[0];
        let tail_week = *writable_weeks(base.get_data(), slot, period)
            .last()
            .expect("Potions runs on the second period");
        prepare(
            &mut base,
            Op::Colloscope(ColloscopeOp::SetInterrogation(
                slot,
                tail_week,
                BTreeSet::from([0]),
            )),
        );

        let old_week_count = week_count(base.get_data(), period);
        let kept = 7;
        let tail: Vec<WeekId> = (kept..old_week_count)
            .map(|position| week_at(base.get_data(), period, position))
            .collect();
        assert!(
            tail.contains(&tail_week),
            "the fixture's colle should sit on a week the cut hands over"
        );

        let op = GeneralPlanningUpdateOp::CutPeriod(period, kept);
        let (state, warnings, new_id) = apply_alone(&base, &op);

        let new_id = new_id.expect("cutting a period returns the id of the tail period");
        assert!(warnings.is_empty(), "a cut loses nothing: {warnings:?}");
        assert_eq!(week_count(state.get_data(), period), kept);
        assert_eq!(week_count(state.get_data(), new_id), old_week_count - kept);
        assert_eq!(
            state
                .get_data()
                .get_inner_data()
                .colloscope
                .interrogation(slot, tail_week),
            Some(&BTreeSet::from([0])),
            "the colle should have travelled with its week"
        );

        let subjects = subjects_in_id_order(base.get_data());
        let mut ops = vec![Op::Period(PeriodOp::AddAfter(period))];
        for subject in subjects {
            let students = base
                .get_data()
                .get_inner_data()
                .params
                .assignments
                .students(period, subject)
                .expect("every subject holds a row on the second period")
                .clone();
            ops.push(Op::Assignment(AssignmentOp::SetRow(
                new_id, subject, students,
            )));
        }
        for subject in &subjects[..6] {
            let group_list = *base
                .get_data()
                .get_inner_data()
                .params
                .group_lists
                .subjects_associations
                .get(&(period, *subject))
                .expect("the first six subjects hold an association");
            ops.push(Op::GroupList(GroupListOp::AssignToSubject(
                new_id,
                *subject,
                Some(group_list),
            )));
        }
        for (destination, week) in tail.into_iter().enumerate() {
            ops.push(Op::Week(WeekOp::Move(week, new_id, destination)));
        }

        assert_eq!(state.get_data(), expected_document(&base, ops).get_data());
    }

    /// The two loops the fixture above cannot reach: hogwarts excludes nobody
    /// from anything, so the cut's subject and student exclusion copies never
    /// run on the plain base. Here Quidditch stops running on the period and
    /// Harry stops being there, and both exclusions must reach the tail period
    /// — not out of tidiness, but because the moved weeks are read against
    /// them.
    #[test]
    fn cutting_a_period_gives_the_tail_period_the_same_exclusions() {
        let mut base = hogwarts();
        let period = period_at(base.get_data(), 1);
        let quidditch = subject_by_name(base.get_data(), "Entrainement de Quidditch");
        let harry = student_by_name(base.get_data(), "Potter", "Harry");

        // A subject can only stop running on a period once nobody is enrolled
        // in it there, and a student can only be absent from a period once no
        // row of that period holds them.
        prepare(
            &mut base,
            Op::Assignment(AssignmentOp::SetRow(period, quidditch, BTreeSet::new())),
        );
        for subject in subjects_in_id_order(base.get_data()) {
            let Some(row) = base
                .get_data()
                .get_inner_data()
                .params
                .assignments
                .students(period, subject)
            else {
                continue;
            };
            if !row.contains(&harry) {
                continue;
            }
            let mut without_harry = row.clone();
            without_harry.remove(&harry);
            prepare(
                &mut base,
                Op::Assignment(AssignmentOp::SetRow(period, subject, without_harry)),
            );
        }

        let mut absent_subject = subject_of(base.get_data(), quidditch);
        absent_subject.excluded_periods.insert(period);
        prepare(
            &mut base,
            Op::Subject(SubjectOp::Update(quidditch, absent_subject.clone())),
        );
        let mut absent_student = student_of(base.get_data(), harry);
        absent_student.excluded_periods.insert(period);
        prepare(
            &mut base,
            Op::Student(StudentOp::Update(harry, absent_student.clone())),
        );

        let old_week_count = week_count(base.get_data(), period);
        let kept = 7;

        let op = GeneralPlanningUpdateOp::CutPeriod(period, kept);
        let (state, warnings, new_id) = apply_alone(&base, &op);

        let new_id = new_id.expect("cutting a period returns the id of the tail period");
        assert!(warnings.is_empty(), "a cut loses nothing: {warnings:?}");
        assert!(
            subject_of(state.get_data(), quidditch)
                .excluded_periods
                .contains(&new_id),
            "the tail period should inherit the subject's exclusion"
        );
        assert!(
            student_of(state.get_data(), harry)
                .excluded_periods
                .contains(&new_id),
            "the tail period should inherit the student's exclusion"
        );

        let mut ops = vec![Op::Period(PeriodOp::AddAfter(period))];
        absent_subject.excluded_periods.insert(new_id);
        ops.push(Op::Subject(SubjectOp::Update(quidditch, absent_subject)));
        absent_student.excluded_periods.insert(new_id);
        ops.push(Op::Student(StudentOp::Update(harry, absent_student)));
        for subject in subjects_in_id_order(base.get_data()) {
            let Some(students) = base
                .get_data()
                .get_inner_data()
                .params
                .assignments
                .students(period, subject)
            else {
                continue;
            };
            ops.push(Op::Assignment(AssignmentOp::SetRow(
                new_id,
                subject,
                students.clone(),
            )));
        }
        for subject in &subjects_in_id_order(base.get_data())[..6] {
            let group_list = *base
                .get_data()
                .get_inner_data()
                .params
                .group_lists
                .subjects_associations
                .get(&(period, *subject))
                .expect("the first six subjects hold an association");
            ops.push(Op::GroupList(GroupListOp::AssignToSubject(
                new_id,
                *subject,
                Some(group_list),
            )));
        }
        for (destination, position) in (kept..old_week_count).enumerate() {
            ops.push(Op::Week(WeekOp::Move(
                week_at(base.get_data(), period, position),
                new_id,
                destination,
            )));
        }

        assert_eq!(state.get_data(), expected_document(&base, ops).get_data());
    }

    /// The divergence, and the bug of `docs/todos/fixme_ops.md`. The weeks of
    /// the merged period are appended to the previous one — the two are
    /// neighbours, so the global week order does not move and no pattern
    /// changes meaning — and their colles travel with them. Both periods use
    /// the same group list for Potions here, so the moved cell is as legal at
    /// its new coordinate as it was at the old one: **it survives**, where the
    /// old body erased every cell its reconciliation could not carry.
    ///
    /// The dead period's own keyed material — its eight assignment rows and its
    /// six associations — is dropped rather than reconciled (the plan's
    /// divergence 5), and **in silence**: every one of them is byte-identical to
    /// the surviving period's, so the merged weeks read exactly as they did and
    /// there is nothing to tell the user about.
    #[test]
    fn merging_two_periods_keeps_the_colles_of_the_one_that_goes() {
        let mut base = hogwarts();
        let previous = period_at(base.get_data(), 0);
        let merged = period_at(base.get_data(), 1);
        let subject = subject_by_name(base.get_data(), "Potions");
        let slot = slots_of_subject(base.get_data(), subject)[0];
        let week = writable_weeks(base.get_data(), slot, merged)[0];
        prepare(
            &mut base,
            Op::Colloscope(ColloscopeOp::SetInterrogation(
                slot,
                week,
                BTreeSet::from([0]),
            )),
        );

        let append_start = week_count(base.get_data(), previous);
        assert_eq!(append_start, 4, "the base's first period holds four weeks");
        let moved: Vec<WeekId> = (0..week_count(base.get_data(), merged))
            .map(|position| week_at(base.get_data(), merged, position))
            .collect();

        let op = GeneralPlanningUpdateOp::MergeWithPreviousPeriod(merged);
        let (state, warnings, _new_id) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            Vec::new(),
            "the two periods carry the same rows and the same associations, so \
             dropping the dying period's copies changes nothing and says nothing",
        );
        assert_eq!(
            state
                .get_data()
                .get_inner_data()
                .colloscope
                .interrogation(slot, week),
            Some(&BTreeSet::from([0])),
            "the colle should have survived the merge"
        );
        assert_eq!(
            week_count(state.get_data(), previous),
            append_start + moved.len()
        );

        let mut ops: Vec<Op> = moved
            .iter()
            .enumerate()
            .map(|(offset, week)| Op::Week(WeekOp::Move(*week, previous, append_start + offset)))
            .collect();
        ops.extend(period_removal_ops(base.get_data(), merged));

        assert_eq!(state.get_data(), expected_document(&base, ops).get_data());
    }

    /// The other side of the same coin: a colle survives only if the surviving
    /// period's context still allows it. Here Potions is bound by the base's
    /// five-group Divination list on the first period and by its eight-group
    /// main list on the second, so of the two colles the fixture writes, the
    /// one on group 6 no longer fits and the one on group 0 does. Exactly the
    /// first is trimmed, as its week arrives — before the dead period's own
    /// material is dropped.
    #[test]
    fn merging_clears_only_the_colles_the_surviving_period_invalidates() {
        let mut base = hogwarts();
        let previous = period_at(base.get_data(), 0);
        let merged = period_at(base.get_data(), 1);
        let subject = subject_by_name(base.get_data(), "Potions");
        let smaller_list = *base
            .get_data()
            .get_inner_data()
            .params
            .group_lists
            .subjects_associations
            .get(&(previous, subject_by_name(base.get_data(), "Divination")))
            .expect("Divination holds an association on the first period");
        prepare(
            &mut base,
            Op::GroupList(GroupListOp::AssignToSubject(
                previous,
                subject,
                Some(smaller_list),
            )),
        );

        let slots = slots_of_subject(base.get_data(), subject);
        let (kept_slot, trimmed_slot) = (slots[0], slots[1]);
        let week = writable_weeks(base.get_data(), kept_slot, merged)[0];
        prepare(
            &mut base,
            Op::Colloscope(ColloscopeOp::SetInterrogation(
                kept_slot,
                week,
                BTreeSet::from([0]),
            )),
        );
        prepare(
            &mut base,
            Op::Colloscope(ColloscopeOp::SetInterrogation(
                trimmed_slot,
                week,
                BTreeSet::from([6]),
            )),
        );

        let append_start = week_count(base.get_data(), previous);
        let moved: Vec<WeekId> = (0..week_count(base.get_data(), merged))
            .map(|position| week_at(base.get_data(), merged, position))
            .collect();

        let op = GeneralPlanningUpdateOp::MergeWithPreviousPeriod(merged);
        let (state, warnings, _new_id) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![
                Fix::RemoveGroupsFromInterrogationCell {
                    slot: trimmed_slot,
                    week,
                    groups: BTreeSet::from([6]),
                    rebuilt: BTreeSet::new(),
                },
                // The one association the two periods disagree on: Potions is
                // bound by the small Divination list on the surviving period and
                // by the main list on the dying one, so dropping the dying copy
                // really does change what bounds the moved weeks. The five
                // associations they share, and all eight rows, go silently.
                Fix::UnassignGroupList {
                    period: merged,
                    subject,
                },
            ],
        );

        let colloscope = &state.get_data().get_inner_data().colloscope;
        assert_eq!(
            colloscope.interrogation(kept_slot, week),
            Some(&BTreeSet::from([0])),
            "the colle the smaller list still bounds should survive"
        );
        assert_eq!(
            colloscope.interrogation(trimmed_slot, week),
            None,
            "the colle on a group the smaller list does not have should be gone"
        );

        let mut ops = vec![Op::Colloscope(ColloscopeOp::SetInterrogation(
            trimmed_slot,
            week,
            BTreeSet::new(),
        ))];
        ops.extend(
            moved.iter().enumerate().map(|(offset, week)| {
                Op::Week(WeekOp::Move(*week, previous, append_start + offset))
            }),
        );
        ops.extend(period_removal_ops(base.get_data(), merged));

        assert_eq!(state.get_data(), expected_document(&base, ops).get_data());
    }

    /// The four « périodes exclues » sets, both ways round. An exclusion the
    /// surviving period repeats is dropped in silence: the merged weeks stay
    /// exactly as excluded as they were, because the period they land on
    /// excludes the same thing. An exclusion only the dying period holds is a
    /// real change — those weeks start being covered — and that is the one the
    /// user is told about.
    ///
    /// The four reported repairs come in the canonical reference order: subject,
    /// student, pairing rule, slot pairing rule. The base's own rows and
    /// associations are identical across the two periods, so they add nothing.
    #[test]
    fn merging_reports_only_the_exclusions_the_surviving_period_does_not_repeat() {
        /// Excluding a subject from a period contradicts the row and the
        /// association it holds there, so both go first — by the preparation,
        /// not by the merge.
        fn clear_subject_on(base: &mut AppState<Data, Desc>, subject: SubjectId, period: PeriodId) {
            prepare(
                base,
                Op::Assignment(AssignmentOp::SetRow(period, subject, BTreeSet::new())),
            );
            prepare(
                base,
                Op::GroupList(GroupListOp::AssignToSubject(period, subject, None)),
            );
        }

        /// Same for a student: they may not be listed in an assignment row of a
        /// period they skip, so take them out of the period's rows first. The
        /// rows only ever shrink, so each stays a subset of the surviving
        /// period's and none of them turns into a warning of its own.
        fn withdraw_student_from(
            base: &mut AppState<Data, Desc>,
            student: StudentId,
            period: PeriodId,
        ) {
            let rows: Vec<(SubjectId, BTreeSet<StudentId>)> = base
                .get_data()
                .get_inner_data()
                .params
                .assignments
                .subjects_for_period(period)
                .filter(|(_subject, students)| students.contains(&student))
                .map(|(subject, students)| {
                    let mut without = students.clone();
                    without.remove(&student);
                    (subject, without)
                })
                .collect();
            for (subject, students) in rows {
                prepare(
                    base,
                    Op::Assignment(AssignmentOp::SetRow(period, subject, students)),
                );
            }
        }

        fn pairing_rule_of(data: &Data, rule: PairingRuleId) -> PairingRule {
            data.get_inner_data()
                .params
                .pairings
                .pairing_rule_map
                .get(&rule)
                .expect("the fixture's pairing rule should be live")
                .clone()
        }

        fn slot_pairing_rule_of(data: &Data, rule: SlotPairingRuleId) -> SlotPairingRule {
            data.get_inner_data()
                .params
                .slot_pairings
                .slot_pairing_rule_map
                .get(&rule)
                .expect("the fixture's slot pairing rule should be live")
                .clone()
        }

        fn pairing_rule_excluding(
            data: &Data,
            rule: PairingRuleId,
            periods: BTreeSet<PeriodId>,
        ) -> PairingRule {
            let (antecedent, consequent, _excluded, soft) =
                pairing_rule_of(data, rule).into_parts();
            PairingRule::new(antecedent, consequent, periods, soft)
                .expect("the parts are the fixture's own, so they name two subjects")
        }

        fn slot_pairing_rule_excluding(
            data: &Data,
            rule: SlotPairingRuleId,
            periods: BTreeSet<PeriodId>,
        ) -> SlotPairingRule {
            let (antecedent, consequent, _excluded, soft) =
                slot_pairing_rule_of(data, rule).into_parts();
            SlotPairingRule::new(antecedent, consequent, periods, soft)
                .expect("the parts are the fixture's own, so they name two slots")
        }

        let mut base = hogwarts();
        let previous = period_at(base.get_data(), 0);
        let merged = period_at(base.get_data(), 1);

        // The base's last two subjects are the ones that hold no group-list
        // association, so excluding them costs one row apiece.
        let subjects = subjects_in_id_order(base.get_data());
        let (shared_subject, lone_subject) = (subjects[6], subjects[7]);
        for period in [previous, merged] {
            clear_subject_on(&mut base, shared_subject, period);
        }
        clear_subject_on(&mut base, lone_subject, merged);
        for (subject, periods) in [
            (shared_subject, BTreeSet::from([previous, merged])),
            (lone_subject, BTreeSet::from([merged])),
        ] {
            let mut rebuilt = subject_of(base.get_data(), subject);
            rebuilt.excluded_periods = periods;
            prepare(&mut base, Op::Subject(SubjectOp::Update(subject, rebuilt)));
        }

        let shared_student = student_by_name(base.get_data(), "Granger", "Hermione");
        let lone_student = student_by_name(base.get_data(), "Weasley", "Ron");
        for period in [previous, merged] {
            withdraw_student_from(&mut base, shared_student, period);
        }
        withdraw_student_from(&mut base, lone_student, merged);
        for (student, periods) in [
            (shared_student, BTreeSet::from([previous, merged])),
            (lone_student, BTreeSet::from([merged])),
        ] {
            let mut rebuilt = student_of(base.get_data(), student);
            rebuilt.excluded_periods = periods;
            prepare(&mut base, Op::Student(StudentOp::Update(student, rebuilt)));
        }

        // The base holds no subject pairing rule at all, so this fixture writes
        // its own two: « avoir Potions ⇒ avoir Métamorphose », excluded from
        // both periods and then from the dying one only.
        let part = |subject| RulePart {
            subject_id: subject,
            should_have: true,
        };
        let rule_parts = (
            part(subject_by_name(base.get_data(), "Potions")),
            part(subject_by_name(base.get_data(), "Métamorphose")),
        );
        let mut pairing_rules = Vec::new();
        for periods in [BTreeSet::from([previous, merged]), BTreeSet::from([merged])] {
            let rule = PairingRule::new(rule_parts.0.clone(), rule_parts.1.clone(), periods, false)
                .expect("the two parts name two different subjects");
            match prepare_new(&mut base, Op::Pairing(PairingOp::Add(rule))) {
                NewId::PairingRuleId(id) => pairing_rules.push(id),
                other => panic!("Unexpected id after adding a pairing rule: {other:?}"),
            }
        }
        let (shared_rule, lone_rule) = (pairing_rules[0], pairing_rules[1]);

        // The base's two slot pairing rules exclude nothing yet, so they take
        // the same two roles.
        let slot_rules: Vec<SlotPairingRuleId> = base
            .get_data()
            .get_inner_data()
            .params
            .slot_pairings
            .slot_pairing_rule_map
            .keys()
            .collect();
        assert_eq!(slot_rules.len(), 2, "the base holds two slot pairing rules");
        let (shared_slot_rule, lone_slot_rule) = (slot_rules[0], slot_rules[1]);
        for (rule, periods) in [
            (shared_slot_rule, BTreeSet::from([previous, merged])),
            (lone_slot_rule, BTreeSet::from([merged])),
        ] {
            let rebuilt = slot_pairing_rule_excluding(base.get_data(), rule, periods);
            prepare(
                &mut base,
                Op::SlotPairing(SlotPairingOp::Update(rule, rebuilt)),
            );
        }

        let append_start = week_count(base.get_data(), previous);
        let moved: Vec<WeekId> = (0..week_count(base.get_data(), merged))
            .map(|position| week_at(base.get_data(), merged, position))
            .collect();

        // What each of the eight entities looks like once the dying period is
        // out of its set — the payload of the repair, or of the authored drop.
        let without_merged_subject = |subject| {
            let mut rebuilt = subject_of(base.get_data(), subject);
            rebuilt.excluded_periods.remove(&merged);
            rebuilt
        };
        let (shared_subject_rebuilt, lone_subject_rebuilt) = (
            without_merged_subject(shared_subject),
            without_merged_subject(lone_subject),
        );
        let without_merged_student = |student| {
            let mut rebuilt = student_of(base.get_data(), student);
            rebuilt.excluded_periods.remove(&merged);
            rebuilt
        };
        let (shared_student_rebuilt, lone_student_rebuilt) = (
            without_merged_student(shared_student),
            without_merged_student(lone_student),
        );
        let shared_rule_rebuilt =
            pairing_rule_excluding(base.get_data(), shared_rule, BTreeSet::from([previous]));
        let lone_rule_rebuilt = pairing_rule_excluding(base.get_data(), lone_rule, BTreeSet::new());
        let shared_slot_rule_rebuilt = slot_pairing_rule_excluding(
            base.get_data(),
            shared_slot_rule,
            BTreeSet::from([previous]),
        );
        let lone_slot_rule_rebuilt =
            slot_pairing_rule_excluding(base.get_data(), lone_slot_rule, BTreeSet::new());

        let op = GeneralPlanningUpdateOp::MergeWithPreviousPeriod(merged);
        let (state, warnings, _new_id) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![
                Fix::RemoveSubjectPeriodExclusion {
                    subject: lone_subject,
                    period: merged,
                    rebuilt: lone_subject_rebuilt.clone(),
                },
                Fix::RemoveStudentPeriodExclusion {
                    student: lone_student,
                    period: merged,
                    rebuilt: lone_student_rebuilt.clone(),
                },
                Fix::RemovePairingRulePeriodExclusion {
                    rule: lone_rule,
                    period: merged,
                    rebuilt: lone_rule_rebuilt.clone(),
                },
                Fix::RemoveSlotPairingRulePeriodExclusion {
                    rule: lone_slot_rule,
                    period: merged,
                    rebuilt: lone_slot_rule_rebuilt.clone(),
                },
            ],
            "only the four exclusions the surviving period does not repeat are worth a word",
        );

        // The shared four lost the dying period from their sets just the same —
        // silently. The document is what it would have been either way, which is
        // the whole claim.
        let mut ops: Vec<Op> = moved
            .iter()
            .enumerate()
            .map(|(offset, week)| Op::Week(WeekOp::Move(*week, previous, append_start + offset)))
            .collect();
        ops.push(Op::Subject(SubjectOp::Update(
            shared_subject,
            shared_subject_rebuilt,
        )));
        ops.push(Op::Subject(SubjectOp::Update(
            lone_subject,
            lone_subject_rebuilt,
        )));
        ops.push(Op::Student(StudentOp::Update(
            shared_student,
            shared_student_rebuilt,
        )));
        ops.push(Op::Student(StudentOp::Update(
            lone_student,
            lone_student_rebuilt,
        )));
        ops.push(Op::Pairing(PairingOp::Update(
            shared_rule,
            shared_rule_rebuilt,
        )));
        ops.push(Op::Pairing(PairingOp::Update(lone_rule, lone_rule_rebuilt)));
        ops.push(Op::SlotPairing(SlotPairingOp::Update(
            shared_slot_rule,
            shared_slot_rule_rebuilt,
        )));
        ops.push(Op::SlotPairing(SlotPairingOp::Update(
            lone_slot_rule,
            lone_slot_rule_rebuilt,
        )));
        // The two subjects cleared above hold no row and no association on the
        // dying period any more, so their entries here are no-ops.
        ops.extend(period_removal_ops(base.get_data(), merged));

        assert_eq!(state.get_data(), expected_document(&base, ops).get_data());
    }

    /// Turning the colles off on a week contradicts every colle already written
    /// there — the checker's `InterrogationOnInactiveWeek` — and the cascade
    /// clears exactly those. Turning them back on contradicts nothing.
    #[test]
    fn deactivating_a_week_clears_the_colles_written_on_it() {
        let mut base = hogwarts();
        let period = period_at(base.get_data(), 1);
        let subject = subject_by_name(base.get_data(), "Potions");
        let slot = slots_of_subject(base.get_data(), subject)[0];
        let week = writable_weeks(base.get_data(), slot, period)[0];
        prepare(
            &mut base,
            Op::Colloscope(ColloscopeOp::SetInterrogation(
                slot,
                week,
                BTreeSet::from([0]),
            )),
        );
        let position = (0..week_count(base.get_data(), period))
            .find(|position| week_at(base.get_data(), period, *position) == week)
            .expect("the colle sits on a week of the period");

        let mut inactive = week_descs(base.get_data(), period)[position].clone();
        inactive.interrogations = false;

        let op = GeneralPlanningUpdateOp::UpdateWeekStatus(period, position, false);
        let (state, warnings, _new_id) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![Fix::ClearInterrogationCell { slot, week }]
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![
                    Op::Colloscope(ColloscopeOp::SetInterrogation(slot, week, BTreeSet::new())),
                    Op::Week(WeekOp::Update(week, inactive.clone())),
                ],
            )
            .get_data(),
        );

        let mut active = inactive;
        active.interrogations = true;
        let op = GeneralPlanningUpdateOp::UpdateWeekStatus(period, position, true);
        let (reactivated, warnings, _new_id) = apply_alone(&state, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            reactivated.get_data(),
            expected_document(&state, vec![Op::Week(WeekOp::Update(week, active))]).get_data(),
        );
    }

    /// A week's annotation is free text nothing else in the document reads.
    #[test]
    fn annotating_a_week_warns_about_nothing() {
        let base = hogwarts();
        let period = period_at(base.get_data(), 1);
        let week = week_at(base.get_data(), period, 0);
        let mut annotated = week_descs(base.get_data(), period)[0].clone();
        annotated.annotation = Some("Semaine des BUSE".parse().expect("non-empty"));

        let op = GeneralPlanningUpdateOp::UpdateWeekAnnotation(
            period,
            0,
            Some("Semaine des BUSE".parse().expect("non-empty")),
        );
        let (state, warnings, _new_id) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(&base, vec![Op::Week(WeekOp::Update(week, annotated))]).get_data(),
        );
    }

    /// Every way the family's own address checks can answer, in one place. They
    /// are ops-level prechecks, not cleaning guards: they decide *which*
    /// elementary ops the composite may emit at all, so they stay exactly where
    /// and as they were.
    #[test]
    fn the_family_reports_every_coordinate_it_cannot_make_sense_of() {
        let base = hogwarts();
        let dangling = dangling_period();
        let first = period_at(base.get_data(), 0);
        let second = period_at(base.get_data(), 1);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            GeneralPlanningUpdateOp::UpdatePeriodWeekCount(dangling, 3)
                .apply_to_session(&mut session)
                .unwrap_err(),
            GeneralPlanningUpdateError::UpdatePeriodWeekCount(
                UpdatePeriodWeekCountError::InvalidPeriodId(dangling)
            ),
        );
        assert_eq!(
            GeneralPlanningUpdateOp::CutPeriod(dangling, 1)
                .apply_to_session(&mut session)
                .unwrap_err(),
            GeneralPlanningUpdateError::CutPeriod(CutPeriodError::InvalidPeriodId(dangling)),
        );
        assert_eq!(
            GeneralPlanningUpdateOp::CutPeriod(first, 5)
                .apply_to_session(&mut session)
                .unwrap_err(),
            GeneralPlanningUpdateError::CutPeriod(CutPeriodError::RemainingWeekCountTooBig(5, 4)),
        );
        assert_eq!(
            GeneralPlanningUpdateOp::MergeWithPreviousPeriod(dangling)
                .apply_to_session(&mut session)
                .unwrap_err(),
            GeneralPlanningUpdateError::MergeWithPreviousPeriod(
                MergeWithPreviousPeriodError::InvalidPeriodId(dangling)
            ),
        );
        assert_eq!(
            GeneralPlanningUpdateOp::MergeWithPreviousPeriod(first)
                .apply_to_session(&mut session)
                .unwrap_err(),
            GeneralPlanningUpdateError::MergeWithPreviousPeriod(
                MergeWithPreviousPeriodError::NoPreviousPeriodToMergeWith
            ),
        );
        assert_eq!(
            GeneralPlanningUpdateOp::UpdateWeekStatus(dangling, 0, false)
                .apply_to_session(&mut session)
                .unwrap_err(),
            GeneralPlanningUpdateError::UpdateWeekStatus(UpdateWeekStatusError::InvalidPeriodId(
                dangling
            )),
        );
        assert_eq!(
            GeneralPlanningUpdateOp::UpdateWeekStatus(second, 14, false)
                .apply_to_session(&mut session)
                .unwrap_err(),
            GeneralPlanningUpdateError::UpdateWeekStatus(UpdateWeekStatusError::InvalidWeekNumber(
                14, 14
            )),
        );
        assert_eq!(
            GeneralPlanningUpdateOp::UpdateWeekAnnotation(dangling, 0, None)
                .apply_to_session(&mut session)
                .unwrap_err(),
            GeneralPlanningUpdateError::UpdateWeekAnnotation(
                UpdateWeekAnnotationError::InvalidPeriodId(dangling)
            ),
        );
        assert_eq!(
            GeneralPlanningUpdateOp::UpdateWeekAnnotation(second, 14, None)
                .apply_to_session(&mut session)
                .unwrap_err(),
            GeneralPlanningUpdateError::UpdateWeekAnnotation(
                UpdateWeekAnnotationError::InvalidWeekNumber(14, 14)
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
        let (_state, warnings) = session.commit((OpCategory::GeneralPlanning, "Rien".into()));
        assert!(warnings.is_empty(), "nothing was applied: {warnings:?}");
    }
}
