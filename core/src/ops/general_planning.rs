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
    pub fn apply(&self, data: &mut AppState<Data>) -> Result<(), GeneralPlanningUpdateError> {
        match self {
            GeneralPlanningUpdateOp::DeleteFirstWeek => {
                data.apply(
                    collomatique_state_colloscopes::Op::Period(
                        collomatique_state_colloscopes::PeriodOp::ChangeStartDate(None),
                    ),
                    "Effacer le début des colles".into(),
                )
                .expect("Deleting first week should always work");
                Ok(())
            }
            GeneralPlanningUpdateOp::UpdateFirstWeek(date) => {
                data.apply(
                    collomatique_state_colloscopes::Op::Period(
                        collomatique_state_colloscopes::PeriodOp::ChangeStartDate(Some(
                            date.clone(),
                        )),
                    ),
                    "Changer le début des colles".into(),
                )
                .expect("Updating first week should always work");
                Ok(())
            }
            GeneralPlanningUpdateOp::AddNewPeriod(week_count) => {
                let new_desc = vec![true; *week_count];
                data.apply(
                    collomatique_state_colloscopes::Op::Period(
                        match data.get_data().get_periods().ordered_period_list.last() {
                            Some((id, _)) => {
                                collomatique_state_colloscopes::PeriodOp::AddAfter(*id, new_desc)
                            }
                            None => collomatique_state_colloscopes::PeriodOp::AddFront(new_desc),
                        },
                    ),
                    "Ajouter une période".into(),
                )
                .expect("Adding a period should never fail");
                Ok(())
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

                data.apply(
                    collomatique_state_colloscopes::Op::Period(
                        collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
                    ),
                    "Modifier une période".into(),
                )
                .expect("Period id should be valid at this point");
                Ok(())
            }
            GeneralPlanningUpdateOp::DeletePeriod(period_id) => {
                data.apply(
                    collomatique_state_colloscopes::Op::Period(
                        collomatique_state_colloscopes::PeriodOp::Remove(*period_id),
                    ),
                    "Supprimer une période".into(),
                )
                .map_err(|e| match e {
                    collomatique_state_colloscopes::Error::InvalidPeriodId(id) => {
                        DeletePeriodError::InvalidPeriodId(id)
                    }
                    _ => panic!("Unexpected error {:?}", e),
                })?;
                Ok(())
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

                session
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
                        ),
                        "Racourcir une période".into(),
                    )
                    .expect("At this point, period id should be valid");
                session
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::AddAfter(
                                *period_id, new_desc,
                            ),
                        ),
                        "Ajouter une période".into(),
                    )
                    .expect("At this point, period id should be valid");

                *data = session.commit("Découper une période".into());
                Ok(())
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

                session
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
                session
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::Remove(*period_id),
                        ),
                        "Suppression d'une période".into(),
                    )
                    .expect("At this point, period id should be valid");

                *data = session.commit("Fusionner deux périodes".into());
                Ok(())
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

                data.apply(
                    collomatique_state_colloscopes::Op::Period(
                        collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
                    ),
                    if *state {
                        "Ajouter une semaine de colle".into()
                    } else {
                        "Supprimer une semaine de colle".into()
                    },
                )
                .expect("At this point, parameters should be valid");
                Ok(())
            }
        }
    }
}
