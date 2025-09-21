use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColloscopesUpdateWarning {}

impl ColloscopesUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<String> {
        None
    }
}

#[derive(Debug, Clone)]
pub enum ColloscopesUpdateOp {
    AddEmptyColloscope(String),
    CopyColloscope(collomatique_state_colloscopes::ColloscopeId, String),
    UpdateColloscope(
        collomatique_state_colloscopes::ColloscopeId,
        collomatique_state_colloscopes::colloscopes::Colloscope,
    ),
    DeleteColloscope(collomatique_state_colloscopes::ColloscopeId),
}

#[derive(Debug, Error)]
pub enum ColloscopesUpdateError {
    #[error(transparent)]
    AddEmptyColloscope(#[from] AddEmptyColloscopeError),
    #[error(transparent)]
    CopyColloscope(#[from] CopyColloscopeError),
    #[error(transparent)]
    UpdateColloscope(#[from] UpdateColloscopeError),
    #[error(transparent)]
    DeleteColloscope(#[from] DeleteColloscopeError),
}

#[derive(Debug, Error)]
pub enum AddEmptyColloscopeError {}

#[derive(Debug, Error)]
pub enum CopyColloscopeError {
    #[error("Colloscope ID {0:?} is invalid")]
    InvalidColloscopeId(collomatique_state_colloscopes::ColloscopeId),
}

#[derive(Debug, Error)]
pub enum UpdateColloscopeError {
    #[error("Colloscope ID {0:?} is invalid")]
    InvalidColloscopeId(collomatique_state_colloscopes::ColloscopeId),
    #[error("Bad invariant in colloscope")]
    BadInvariantInColloscope(collomatique_state_colloscopes::InvariantError),
}

#[derive(Debug, Error)]
pub enum DeleteColloscopeError {
    #[error("Colloscope ID {0:?} is invalid")]
    InvalidColloscopeId(collomatique_state_colloscopes::ColloscopeId),
}

impl ColloscopesUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<CleaningOp<RulesUpdateWarning>> {
        None
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::ColloscopeId>, ColloscopesUpdateError> {
        match self {
            Self::AddEmptyColloscope(name) => {
                let (params, id_maps) = data.get_data().copy_main_params();
                let collo_data = collomatique_state_colloscopes::colloscopes::ColloscopeData::new_empty_from_params(&params);
                let new_colloscope = collomatique_state_colloscopes::colloscopes::Colloscope {
                    name: name.clone(),
                    params,
                    id_maps,
                    data: collo_data,
                };

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Colloscopes(
                            collomatique_state_colloscopes::ColloscopeOp::Add(new_colloscope),
                        ),
                        self.get_desc(),
                    )
                    .expect("An empty colloscope should always be valid");
                let Some(collomatique_state_colloscopes::NewId::ColloscopeId(new_id)) = result
                else {
                    panic!("Unexpected result from ColloscopeOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::CopyColloscope(colloscope_id, new_name) => {
                let Some(orig_colloscope) = data
                    .get_data()
                    .get_inner_data()
                    .colloscopes
                    .colloscope_map
                    .get(colloscope_id)
                else {
                    return Err(CopyColloscopeError::InvalidColloscopeId(*colloscope_id).into());
                };

                let mut new_colloscope = orig_colloscope.clone();
                new_colloscope.name = new_name.clone();

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Colloscopes(
                            collomatique_state_colloscopes::ColloscopeOp::Add(new_colloscope),
                        ),
                        self.get_desc(),
                    )
                    .expect("A copied colloscope should always be valid");
                let Some(collomatique_state_colloscopes::NewId::ColloscopeId(new_id)) = result
                else {
                    panic!("Unexpected result from ColloscopeOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateColloscope(colloscope_id, colloscope) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Colloscopes(
                            collomatique_state_colloscopes::ColloscopeOp::Update(
                                *colloscope_id,
                                colloscope.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Colloscope(ce) = e {
                            match ce {
                                collomatique_state_colloscopes::ColloscopeError::InvalidColloscopeId(id) => {
                                    UpdateColloscopeError::InvalidColloscopeId(id)
                                }
                                collomatique_state_colloscopes::ColloscopeError::InvariantErrorInParameters(invariant_error) => {
                                    UpdateColloscopeError::BadInvariantInColloscope(invariant_error)
                                }
                                _ => panic!("Unexpected colloscope error during UpdateColloscope: {:?}", ce),
                            }
                        } else {
                            panic!("Unexpected error during UpdateColloscope: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteColloscope(colloscope_id) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Colloscopes(
                            collomatique_state_colloscopes::ColloscopeOp::Remove(*colloscope_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| if let collomatique_state_colloscopes::Error::Colloscope(ce) = e {
                            match ce {
                                collomatique_state_colloscopes::ColloscopeError::InvalidColloscopeId(id) => {
                                    DeleteColloscopeError::InvalidColloscopeId(id)
                                }
                                _ => panic!("Unexpected colloscope error during DeleteColloscope: {:?}", ce),
                            }
                        } else {
                            panic!("Unexpected error during DeleteColloscope: {:?}", e);
                        }
                    )?;

                assert!(result.is_none());

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Rules,
            match self {
                ColloscopesUpdateOp::AddEmptyColloscope(_name) => "Créer un colloscope vide".into(),
                ColloscopesUpdateOp::CopyColloscope(_id, _name) => "Dupliquer un colloscope".into(),
                ColloscopesUpdateOp::DeleteColloscope(_id) => "Supprimer un colloscope".into(),
                ColloscopesUpdateOp::UpdateColloscope(_id, _colloscope) => {
                    "Modifier un colloscope".into()
                }
            },
        )
    }
}
