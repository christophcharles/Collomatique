use super::*;

#[derive(Debug)]
pub enum WeekPatternsUpdateWarning {}

impl WeekPatternsUpdateWarning {
    pub fn build_desc<T: collomatique_state::traits::Manager<Data = Data>>(
        &self,
        _data: &T,
    ) -> String {
        String::new()
    }
}

#[derive(Debug)]
pub enum WeekPatternsUpdateOp {
    AddNewWeekPattern(collomatique_state_colloscopes::week_patterns::WeekPattern),
    UpdateWeekPattern(
        collomatique_state_colloscopes::WeekPatternId,
        collomatique_state_colloscopes::week_patterns::WeekPattern,
    ),
    DeleteWeekPattern(collomatique_state_colloscopes::WeekPatternId),
}

#[derive(Debug, Error)]
pub enum WeekPatternsUpdateError {
    #[error(transparent)]
    UpdateWeekPattern(#[from] UpdateWeekPatternError),
    #[error(transparent)]
    DeleteWeekPattern(#[from] DeleteWeekPatternError),
}

#[derive(Debug, Error)]
pub enum UpdateWeekPatternError {
    #[error("Week pattern ID {0:?} is invalid")]
    InvalidWeekPatternId(collomatique_state_colloscopes::WeekPatternId),
}

#[derive(Debug, Error)]
pub enum DeleteWeekPatternError {
    #[error("Week pattern ID {0:?} is invalid")]
    InvalidWeekPatternId(collomatique_state_colloscopes::WeekPatternId),
}

impl WeekPatternsUpdateOp {
    pub fn get_desc(&self) -> String {
        match self {
            WeekPatternsUpdateOp::AddNewWeekPattern(_desc) => {
                "Ajouter un modèle de périodicité".into()
            }
            WeekPatternsUpdateOp::UpdateWeekPattern(_id, _desc) => {
                "Modifier un modèle de périodicité".into()
            }
            WeekPatternsUpdateOp::DeleteWeekPattern(_id) => {
                "Supprimer un modèle de périodicité".into()
            }
        }
    }

    pub fn get_warnings<T: collomatique_state::traits::Manager<Data = Data>>(
        &self,
        _data: &T,
    ) -> Vec<WeekPatternsUpdateWarning> {
        vec![]
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data>>(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::WeekPatternId>, WeekPatternsUpdateError>
    {
        match self {
            Self::AddNewWeekPattern(week_pattern) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::WeekPattern(
                            collomatique_state_colloscopes::WeekPatternOp::Add(
                                week_pattern.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("Unexpected error during AddNewWeekPattern");
                let Some(collomatique_state_colloscopes::NewId::WeekPatternId(new_id)) = result
                else {
                    panic!("Unexpected result from WeekPatternOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateWeekPattern(week_pattern_id, week_pattern) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::WeekPattern(
                            collomatique_state_colloscopes::WeekPatternOp::Update(
                                *week_pattern_id,
                                week_pattern.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::WeekPattern(wpe) = e {
                            match wpe {
                                collomatique_state_colloscopes::WeekPatternError::InvalidWeekPatternId(id) =>
                                    UpdateWeekPatternError::InvalidWeekPatternId(id),
                                _ => panic!(
                                    "Unexpected week pattern error during UpdateWeekPattern: {:?}",
                                    wpe
                                ),
                            }
                        } else {
                            panic!("Unexpected error during UpdateWeekPattern: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteWeekPattern(week_pattern_id) => {
                let mut session = collomatique_state::AppSession::new(data.clone());

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::WeekPattern(
                            collomatique_state_colloscopes::WeekPatternOp::Remove(*week_pattern_id),
                        ),
                        "Suppression effective du modèle de périodicité".into(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::WeekPattern(wpe) = e {
                            match wpe {
                                collomatique_state_colloscopes::WeekPatternError::InvalidWeekPatternId(id) =>
                                    DeleteWeekPatternError::InvalidWeekPatternId(id),
                                _ => panic!(
                                    "Unexpected week pattern error during DeleteWeekPattern: {:?}",
                                    wpe
                                ),
                            }
                        } else {
                            panic!("Unexpected error during DeleteWeekPattern: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                *data = session.commit(self.get_desc());

                Ok(None)
            }
        }
    }
}
