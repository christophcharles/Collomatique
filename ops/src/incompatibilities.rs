use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum IncompatibilitiesUpdateWarning {
    LooseColloscopeLinkWithIncompat(
        collomatique_state_colloscopes::ColloscopeId,
        collomatique_state_colloscopes::IncompatId,
    ),
}

impl IncompatibilitiesUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            Self::LooseColloscopeLinkWithIncompat(colloscope_id, incompat_id) => {
                let Some(colloscope) = data
                    .get_data()
                    .get_inner_data()
                    .colloscopes
                    .colloscope_map
                    .get(colloscope_id)
                else {
                    return None;
                };
                let Some(incompat) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .incompats
                    .incompat_map
                    .get(incompat_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte de la possibilité de mettre à jour le colloscope \"{}\" pour l'incompatibilité \"{}\"",
                    colloscope.name,
                    incompat.name,
                ))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IncompatibilitiesUpdateOp {
    AddNewIncompat(
        collomatique_state_colloscopes::incompats::Incompatibility<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    ),
    DeleteIncompat(collomatique_state_colloscopes::IncompatId),
    UpdateIncompat(
        collomatique_state_colloscopes::IncompatId,
        collomatique_state_colloscopes::incompats::Incompatibility<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum IncompatibilitiesUpdateError {
    #[error(transparent)]
    AddNewIncompat(#[from] AddNewIncompatError),
    #[error(transparent)]
    DeleteIncompat(#[from] DeleteIncompatError),
    #[error(transparent)]
    UpdateIncompat(#[from] UpdateIncompatError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddNewIncompatError {
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(collomatique_state_colloscopes::WeekPatternId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeleteIncompatError {
    #[error("invalid incompat id ({0:?})")]
    InvalidIncompatId(collomatique_state_colloscopes::IncompatId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
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
        data: &T,
    ) -> Option<CleaningOp<IncompatibilitiesUpdateWarning>> {
        match self {
            Self::AddNewIncompat(_incompat) => None,
            Self::UpdateIncompat(_incompat_id, _incompat) => None,
            Self::DeleteIncompat(incompat_id) => {
                for (colloscope_id, colloscope) in
                    &data.get_data().get_inner_data().colloscopes.colloscope_map
                {
                    if colloscope.id_maps.incompats.contains_key(incompat_id) {
                        let mut new_colloscope = colloscope.clone();
                        new_colloscope.id_maps.incompats.remove(incompat_id);

                        return Some(CleaningOp {
                            warning:
                                IncompatibilitiesUpdateWarning::LooseColloscopeLinkWithIncompat(
                                    *colloscope_id,
                                    *incompat_id,
                                ),
                            op: UpdateOp::Colloscopes(ColloscopesUpdateOp::UpdateColloscope(
                                *colloscope_id,
                                new_colloscope,
                            )),
                        });
                    }
                }
                None
            }
        }
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
}
