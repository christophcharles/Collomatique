use super::*;

#[derive(Debug)]
pub enum GeneralPlanningUpdateOp {
    DeleteFirstWeek,
    UpdateFirstWeek(collomatique_time::NaiveMondayDate),
    AddNewPeriod(usize),
    UpdatePeriodWeekCount(collomatique_state_colloscopes::PeriodId, usize),
    DeletePeriod(collomatique_state_colloscopes::PeriodId),
    CutPeriod(collomatique_state_colloscopes::PeriodId, usize),
    MergeWithPreviousPeriod(collomatique_state_colloscopes::PeriodId),
    UpdateWeekStatus(collomatique_state_colloscopes::PeriodId, usize, bool),
}

#[derive(Debug, Error)]
pub enum GeneralPlanningUpdateError {
    #[error(transparent)]
    UpdatePeriodWeekCount(#[from] UpdatePeriodWeekCountError),
    #[error(transparent)]
    DeletePeriod(#[from] DeletePeriodError),
    #[error(transparent)]
    CutPeriod(#[from] CutPeriodError),
    #[error(transparent)]
    MergeWithPreviousPeriod(#[from] MergeWithPreviousPeriodError),
    #[error(transparent)]
    UpdateWeekStatus(#[from] UpdateWeekStatusError),
}

#[derive(Debug, Error)]
pub enum UpdatePeriodWeekCountError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("Subject {0:?} implies a minimum total number of weeks of {1}")]
    SubjectImpliesMinimumWeekCount(collomatique_state_colloscopes::SubjectId, usize),
}

#[derive(Debug, Error)]
pub enum DeletePeriodError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

#[derive(Debug, Error)]
pub enum CutPeriodError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("Remaining week count ({0}) is larger than available week count ({1})")]
    RemainingWeekCountTooBig(usize, usize),
}

#[derive(Debug, Error)]
pub enum MergeWithPreviousPeriodError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("This is the first period and cannot be merged with the non-existent previous one")]
    NoPreviousPeriodToMergeWith,
}

#[derive(Debug, Error)]
pub enum UpdateWeekStatusError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("Week number {0} is larger that the number of available weeks ({1})")]
    InvalidWeekNumber(usize, usize),
}

impl GeneralPlanningUpdateOp {
    pub fn get_desc(&self) -> String {
        match self {
            GeneralPlanningUpdateOp::DeleteFirstWeek => "Effacer le début des colles".into(),
            GeneralPlanningUpdateOp::UpdateFirstWeek(_date) => "Changer le début des colles".into(),
            GeneralPlanningUpdateOp::AddNewPeriod(_week_count) => "Ajouter une période".into(),
            GeneralPlanningUpdateOp::UpdatePeriodWeekCount(_period_id, _week_count) => {
                "Modifier une période".into()
            }
            GeneralPlanningUpdateOp::DeletePeriod(_period_id) => "Supprimer une période".into(),
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
        }
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data>>(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::PeriodId>, GeneralPlanningUpdateError> {
        match self {
            GeneralPlanningUpdateOp::DeleteFirstWeek => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::ChangeStartDate(None),
                        ),
                        self.get_desc(),
                    )
                    .expect("Deleting first week should always work");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
            GeneralPlanningUpdateOp::UpdateFirstWeek(date) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::ChangeStartDate(Some(
                                date.clone(),
                            )),
                        ),
                        self.get_desc(),
                    )
                    .expect("Updating first week should always work");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
            GeneralPlanningUpdateOp::AddNewPeriod(week_count) => {
                let new_desc = vec![true; *week_count];
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            match data.get_data().get_periods().ordered_period_list.last() {
                                Some((id, _)) => {
                                    collomatique_state_colloscopes::PeriodOp::AddAfter(
                                        *id, new_desc,
                                    )
                                }
                                None => {
                                    collomatique_state_colloscopes::PeriodOp::AddFront(new_desc)
                                }
                            },
                        ),
                        self.get_desc(),
                    )
                    .expect("Adding a period should never fail");
                match result {
                    Some(collomatique_state_colloscopes::NewId::PeriodId(id)) => Ok(Some(id)),
                    _ => panic!("Unexpected result! {:?}", result),
                }
            }
            GeneralPlanningUpdateOp::UpdatePeriodWeekCount(period_id, week_count) => {
                let pos = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .ok_or(UpdatePeriodWeekCountError::InvalidPeriodId(*period_id))?;
                let mut desc = data.get_data().get_periods().ordered_period_list[pos]
                    .1
                    .clone();

                desc.resize(*week_count, desc.last().copied().unwrap_or(true));

                let result = match data.apply(
                    collomatique_state_colloscopes::Op::Period(
                        collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
                    ),
                    self.get_desc(),
                ) {
                    Ok(r) => r,
                    Err(collomatique_state_colloscopes::Error::Period(
                        collomatique_state_colloscopes::PeriodError::InvalidPeriodId(_),
                    )) => {
                        panic!(
                                "Period Id {:?} should be valid at this point but InvalidPeriodId received", *period_id
                            )
                    }
                    Err(e) => {
                        panic!("Unexpected error for UpdatePeriodWeekCount! {:?}", e);
                    }
                };
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
            GeneralPlanningUpdateOp::DeletePeriod(period_id) => {
                let mut session = collomatique_state::AppSession::new(data.clone());

                for (subject_id, subject) in &data.get_data().get_subjects().ordered_subject_list {
                    if subject.excluded_periods.contains(period_id) {
                        let mut new_subject = subject.clone();
                        new_subject.excluded_periods.remove(period_id);
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Subject(
                                    collomatique_state_colloscopes::SubjectOp::Update(
                                        *subject_id,
                                        new_subject,
                                    ),
                                ),
                                "Enlever une référence à la période pour une matière".into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                for (student_id, student) in &data.get_data().get_students().student_map {
                    if student.excluded_periods.contains(period_id) {
                        let mut new_student = student.clone();
                        new_student.excluded_periods.remove(period_id);
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Student(
                                    collomatique_state_colloscopes::StudentOp::Update(
                                        *student_id,
                                        new_student,
                                    ),
                                ),
                                "Enlever une référence à la période pour un élève".into(),
                            )
                            .expect("All data should be valid at this point");
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
                        "Suppression effective de la période".into(),
                    )
                    .map_err(|e| match e {
                        collomatique_state_colloscopes::Error::Period(period_e) => match period_e {
                            collomatique_state_colloscopes::PeriodError::InvalidPeriodId(id) => {
                                DeletePeriodError::InvalidPeriodId(id)
                            }
                            _ => panic!("Unexpected error {:?}", period_e),
                        },
                        _ => panic!("Unexpected error {:?}", e),
                    })?;
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }

                *data = session.commit(self.get_desc());
                Ok(None)
            }
            GeneralPlanningUpdateOp::CutPeriod(period_id, new_week_count) => {
                let pos = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .ok_or(CutPeriodError::InvalidPeriodId(*period_id))?;
                let mut desc = data.get_data().get_periods().ordered_period_list[pos]
                    .1
                    .clone();

                if *new_week_count > desc.len() {
                    Err(CutPeriodError::RemainingWeekCountTooBig(
                        *new_week_count,
                        desc.len(),
                    ))?;
                }

                let new_desc = desc.split_off(*new_week_count);

                let mut session = collomatique_state::AppSession::new(data.clone());

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
                        ),
                        "Raccourcir une période".into(),
                    )
                    .expect("At this point, period id should be valid");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::AddAfter(
                                *period_id, new_desc,
                            ),
                        ),
                        "Ajouter une période".into(),
                    )
                    .expect("At this point, period id should be valid");
                let new_id = match result {
                    Some(collomatique_state_colloscopes::NewId::PeriodId(id)) => id,
                    _ => panic!("Unexpected result! {:?}", result),
                };

                for (subject_id, subject) in &data.get_data().get_subjects().ordered_subject_list {
                    if subject.excluded_periods.contains(period_id) {
                        let mut new_subject = subject.clone();
                        new_subject.excluded_periods.insert(new_id.clone());
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Subject(
                                    collomatique_state_colloscopes::SubjectOp::Update(
                                        *subject_id,
                                        new_subject,
                                    ),
                                ),
                                "Dupliquer l'état de la période découpée sur un sujet".into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                *data = session.commit(self.get_desc());
                Ok(Some(new_id))
            }
            GeneralPlanningUpdateOp::MergeWithPreviousPeriod(period_id) => {
                let pos = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .ok_or(MergeWithPreviousPeriodError::InvalidPeriodId(*period_id))?;
                if pos == 0 {
                    Err(MergeWithPreviousPeriodError::NoPreviousPeriodToMergeWith)?;
                }
                let previous_id = data.get_data().get_periods().ordered_period_list[pos - 1].0;

                let mut prev_desc = data.get_data().get_periods().ordered_period_list[pos - 1]
                    .1
                    .clone();
                let desc = data.get_data().get_periods().ordered_period_list[pos]
                    .1
                    .clone();

                prev_desc.extend(desc);

                let mut session = collomatique_state::AppSession::new(data.clone());

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::Update(
                                previous_id,
                                prev_desc,
                            ),
                        ),
                        "Prolongement d'une période".into(),
                    )
                    .expect("At this point, period id should be valid");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }

                for (subject_id, subject) in &data.get_data().get_subjects().ordered_subject_list {
                    if subject.excluded_periods.contains(period_id) {
                        let mut new_subject = subject.clone();
                        new_subject.excluded_periods.remove(period_id);
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Subject(
                                    collomatique_state_colloscopes::SubjectOp::Update(
                                        *subject_id,
                                        new_subject,
                                    ),
                                ),
                                "Enlever une référence à la période à effacer".into(),
                            )
                            .expect("All data should be valid at this point");
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
                        "Suppression d'une période".into(),
                    )
                    .expect("At this point, period id should be valid");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }

                *data = session.commit(self.get_desc());
                Ok(None)
            }
            GeneralPlanningUpdateOp::UpdateWeekStatus(period_id, week_num, state) => {
                let pos = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .ok_or(UpdateWeekStatusError::InvalidPeriodId(*period_id))?;
                let mut desc = data.get_data().get_periods().ordered_period_list[pos]
                    .1
                    .clone();

                if *week_num >= desc.len() {
                    Err(UpdateWeekStatusError::InvalidWeekNumber(
                        *week_num,
                        desc.len(),
                    ))?;
                }

                desc[*week_num] = *state;

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
                        ),
                        self.get_desc(),
                    )
                    .expect("At this point, parameters should be valid");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
        }
    }
}
