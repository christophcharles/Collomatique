use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum IncompatibilitiesUpdateWarning {}

impl IncompatibilitiesUpdateWarning {
    pub fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<String> {
        None
    }
}

#[derive(Debug, Clone)]
pub enum IncompatibilitiesUpdateOp {
    AddNewIncompat(collomatique_state_colloscopes::incompats::Incompatibility),
    DeleteIncompat(collomatique_state_colloscopes::IncompatId),
    UpdateIncompat(
        collomatique_state_colloscopes::IncompatId,
        collomatique_state_colloscopes::incompats::Incompatibility,
    ),
}

#[derive(Debug, Error)]
pub enum IncompatibilitiesUpdateError {
    #[error(transparent)]
    AddNewIncompat(#[from] AddNewIncompatError),
    #[error(transparent)]
    DeleteIncompat(#[from] DeleteIncompatError),
    #[error(transparent)]
    UpdateIncompat(#[from] UpdateIncompatError),
}

#[derive(Debug, Error)]
pub enum AddNewIncompatError {
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(collomatique_state_colloscopes::WeekPatternId),
}

#[derive(Debug, Error)]
pub enum DeleteIncompatError {
    #[error("invalid incompat id ({0:?})")]
    InvalidIncompatId(collomatique_state_colloscopes::IncompatId),
}

#[derive(Debug, Error)]
pub enum UpdateIncompatError {
    #[error("invalid incompat id ({0:?})")]
    InvalidIncompatId(collomatique_state_colloscopes::IncompatId),
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(collomatique_state_colloscopes::WeekPatternId),
}

impl IncompatibilitiesUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<PreCleaningOp<IncompatibilitiesUpdateWarning>> {
        None
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::IncompatId>, IncompatibilitiesUpdateError>
    {
        match self {
            Self::AddNewIncompat(incompat) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Incompat(
                            collomatique_state_colloscopes::IncompatOp::Add(
                                incompat.clone()
                            )
                        ),
                        self.get_desc(),
                    ).map_err(|e| if let collomatique_state_colloscopes::Error::Incompat(ie) = e {
                        match ie {
                            collomatique_state_colloscopes::IncompatError::InvalidSubjectId(id) => AddNewIncompatError::InvalidSubjectId(id),
                            collomatique_state_colloscopes::IncompatError::InvalidWeekPatternId(id) => AddNewIncompatError::InvalidWeekPatternId(id),
                            _ => panic!("Unexpected incompatibility error during AddNewIncompat: {:?}", ie),
                        }
                    } else {
                        panic!("Unexpected error during AddNewIncompat: {:?}", e);
                    })?;
                let Some(collomatique_state_colloscopes::NewId::IncompatId(new_id)) = result else {
                    panic!("Unexpected result from IncompatOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateIncompat(incompat_id, incompat) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Incompat(
                            collomatique_state_colloscopes::IncompatOp::Update(
                                *incompat_id,
                                incompat.clone()
                            )
                        ),
                        self.get_desc(),
                    ).map_err(|e| if let collomatique_state_colloscopes::Error::Incompat(ie) = e {
                        match ie {
                            collomatique_state_colloscopes::IncompatError::InvalidIncompatId(id) => UpdateIncompatError::InvalidIncompatId(id),
                            collomatique_state_colloscopes::IncompatError::InvalidSubjectId(id) => UpdateIncompatError::InvalidSubjectId(id),
                            collomatique_state_colloscopes::IncompatError::InvalidWeekPatternId(id) => UpdateIncompatError::InvalidWeekPatternId(id),
                            _ => panic!("Unexpected incompatibility error during UpdateIncompat: {:?}", ie),
                        }
                    } else {
                        panic!("Unexpected error during UpdateIncompat: {:?}", e);
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteIncompat(incompat_id) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Incompat(
                            collomatique_state_colloscopes::IncompatOp::Remove(*incompat_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Incompat(ie) = e {
                            match ie {
                                collomatique_state_colloscopes::IncompatError::InvalidIncompatId(id) => {
                                    DeleteIncompatError::InvalidIncompatId(id)
                                }
                                _ => panic!("Unexpected slot error during DeleteIncompat: {:?}", ie),
                            }
                        } else {
                            panic!("Unexpected error during DeleteIncompat: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Incompatibilities,
            match self {
                IncompatibilitiesUpdateOp::AddNewIncompat(_) => {
                    "Ajouter une incompatibilité horaire".into()
                }
                IncompatibilitiesUpdateOp::DeleteIncompat(_) => {
                    "Supprimer une incompatibilité horaire".into()
                }
                IncompatibilitiesUpdateOp::UpdateIncompat(_, _) => {
                    "Modifier une incompatibilité horaire".into()
                }
            },
        )
    }

    pub fn get_warnings<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        _data: &T,
    ) -> Vec<IncompatibilitiesUpdateWarning> {
        vec![]
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::IncompatId>, IncompatibilitiesUpdateError>
    {
        match self {
            Self::AddNewIncompat(incompat) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Incompat(
                            collomatique_state_colloscopes::IncompatOp::Add(
                                incompat.clone()
                            )
                        ),
                        self.get_desc(),
                    ).map_err(|e| if let collomatique_state_colloscopes::Error::Incompat(ie) = e {
                        match ie {
                            collomatique_state_colloscopes::IncompatError::InvalidSubjectId(id) => AddNewIncompatError::InvalidSubjectId(id),
                            collomatique_state_colloscopes::IncompatError::InvalidWeekPatternId(id) => AddNewIncompatError::InvalidWeekPatternId(id),
                            _ => panic!("Unexpected incompatibility error during AddNewIncompat: {:?}", ie),
                        }
                    } else {
                        panic!("Unexpected error during AddNewIncompat: {:?}", e);
                    })?;
                let Some(collomatique_state_colloscopes::NewId::IncompatId(new_id)) = result else {
                    panic!("Unexpected result from IncompatOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateIncompat(incompat_id, incompat) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Incompat(
                            collomatique_state_colloscopes::IncompatOp::Update(
                                *incompat_id,
                                incompat.clone()
                            )
                        ),
                        self.get_desc(),
                    ).map_err(|e| if let collomatique_state_colloscopes::Error::Incompat(ie) = e {
                        match ie {
                            collomatique_state_colloscopes::IncompatError::InvalidIncompatId(id) => UpdateIncompatError::InvalidIncompatId(id),
                            collomatique_state_colloscopes::IncompatError::InvalidSubjectId(id) => UpdateIncompatError::InvalidSubjectId(id),
                            collomatique_state_colloscopes::IncompatError::InvalidWeekPatternId(id) => UpdateIncompatError::InvalidWeekPatternId(id),
                            _ => panic!("Unexpected incompatibility error during UpdateIncompat: {:?}", ie),
                        }
                    } else {
                        panic!("Unexpected error during UpdateIncompat: {:?}", e);
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteIncompat(incompat_id) => {
                let mut session = collomatique_state::AppSession::<_, String>::new(data.clone());

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Incompat(
                            collomatique_state_colloscopes::IncompatOp::Remove(*incompat_id),
                        ),
                        "Suppression effective de l'incompatibilité horaire".into(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Incompat(ie) = e {
                            match ie {
                                collomatique_state_colloscopes::IncompatError::InvalidIncompatId(id) => {
                                    DeleteIncompatError::InvalidIncompatId(id)
                                }
                                _ => panic!("Unexpected slot error during DeleteIncompat: {:?}", ie),
                            }
                        } else {
                            panic!("Unexpected error during DeleteIncompat: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                *data = session.commit(self.get_desc());

                Ok(None)
            }
        }
    }
}
