//! Ops module
//!
//! This modules defines all modification operations that can
//! be done *in UI*. These are *natural* oeprations that a user
//! might want to do rather than elementary operations that appear
//! in [collomatique_state_colloscopes] and that are assembled into
//! more complete operations.
//!
//! Concretly any op defined here is consistituted of [collomatique_state_colloscopes::Op]
//! but these are more *natural* operations that will correspond
//! to a simple command in a cli or a click of a button in a gui.
//!

use collomatique_state::{traits::Manager, AppState};
use collomatique_state_colloscopes::Data;

#[derive(Debug)]
pub enum UpdateOp {
    GeneralPlanning(GeneralPlanningUpdateOp),
}

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

impl GeneralPlanningUpdateOp {
    pub fn apply(
        &self,
        data: &mut AppState<Data>,
    ) -> Result<(), collomatique_state_colloscopes::Error> {
        match self {
            GeneralPlanningUpdateOp::DeleteFirstWeek => data.apply(
                collomatique_state_colloscopes::Op::Period(
                    collomatique_state_colloscopes::PeriodOp::ChangeStartDate(None),
                ),
                "Effacer le début des colles".into(),
            ),
            GeneralPlanningUpdateOp::UpdateFirstWeek(date) => data.apply(
                collomatique_state_colloscopes::Op::Period(
                    collomatique_state_colloscopes::PeriodOp::ChangeStartDate(Some(date.clone())),
                ),
                "Changer le début des colles".into(),
            ),
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
            }
            GeneralPlanningUpdateOp::UpdatePeriodWeekCount(period_id, week_count) => {
                let pos = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .expect("period id should be valid");
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
            }
            GeneralPlanningUpdateOp::DeletePeriod(period_id) => data.apply(
                collomatique_state_colloscopes::Op::Period(
                    collomatique_state_colloscopes::PeriodOp::Remove(*period_id),
                ),
                "Supprimer une période".into(),
            ),
            GeneralPlanningUpdateOp::CutPeriod(period_id, new_week_count) => {
                let pos = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .expect("period id should be valid");
                let mut desc = data.get_data().get_periods().ordered_period_list[pos]
                    .1
                    .clone();
                let new_desc = desc.split_off(*new_week_count);

                let mut session = collomatique_state::AppSession::new(data.clone());

                session.apply(
                    collomatique_state_colloscopes::Op::Period(
                        collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
                    ),
                    "Racourcir une période".into(),
                )?;
                session.apply(
                    collomatique_state_colloscopes::Op::Period(
                        collomatique_state_colloscopes::PeriodOp::AddAfter(*period_id, new_desc),
                    ),
                    "Ajouter une période".into(),
                )?;

                *data = session.commit("Découper une période".into());
                Ok(())
            }
            GeneralPlanningUpdateOp::MergeWithPreviousPeriod(period_id) => {
                let pos = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .expect("period id should be valid");
                assert!(pos >= 1);
                let previous_id = data.get_data().get_periods().ordered_period_list[pos - 1].0;

                let mut prev_desc = data.get_data().get_periods().ordered_period_list[pos - 1]
                    .1
                    .clone();
                let desc = data.get_data().get_periods().ordered_period_list[pos]
                    .1
                    .clone();

                prev_desc.extend(desc);

                let mut session = collomatique_state::AppSession::new(data.clone());

                session.apply(
                    collomatique_state_colloscopes::Op::Period(
                        collomatique_state_colloscopes::PeriodOp::Update(previous_id, prev_desc),
                    ),
                    "Prolongement d'une période".into(),
                )?;
                session.apply(
                    collomatique_state_colloscopes::Op::Period(
                        collomatique_state_colloscopes::PeriodOp::Remove(*period_id),
                    ),
                    "Suppression d'une période".into(),
                )?;

                *data = session.commit("Fusionner deux périodes".into());
                Ok(())
            }
            GeneralPlanningUpdateOp::UpdateWeekStatus(period_id, week_num, state) => {
                let pos = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .expect("period id should be valid");
                let mut desc = data.get_data().get_periods().ordered_period_list[pos]
                    .1
                    .clone();
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
            }
        }
    }
}
