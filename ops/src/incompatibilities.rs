use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum IncompatibilitiesUpdateWarning {}

impl IncompatibilitiesUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<String> {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IncompatibilitiesUpdateOp {
    AddNewIncompat(collomatique_state_colloscopes::incompats::Incompatibility),
    DeleteIncompat(collomatique_state_colloscopes::IncompatId),
    UpdateIncompat(
        collomatique_state_colloscopes::IncompatId,
        collomatique_state_colloscopes::incompats::Incompatibility,
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
        _data: &T,
    ) -> Option<CleaningOp<IncompatibilitiesUpdateWarning>> {
        match self {
            Self::AddNewIncompat(_incompat) => None,
            Self::UpdateIncompat(_incompat_id, _incompat) => None,
            Self::DeleteIncompat(_incompat_id) => None,
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
                    .try_apply(
                        collomatique_state_colloscopes::Op::Incompat(
                            collomatique_state_colloscopes::IncompatOp::Add(incompat.clone()),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            ApplyError, FixableInvariant, Reference, SubjectRefSite,
                            WeekPatternRefSite,
                        };
                        match e {
                            // The pre-op state was valid, so any dangle in the set
                            // was introduced by this Add. Old validator order:
                            // subject id before week pattern id.
                            ApplyError::Invariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Subject {
                                        target,
                                        site: SubjectRefSite::IncompatSubject(_),
                                    }) = inv
                                    {
                                        return AddNewIncompatError::InvalidSubjectId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::WeekPattern {
                                        target,
                                        site: WeekPatternRefSite::IncompatWeekPattern(_),
                                    }) = inv
                                    {
                                        return AddNewIncompatError::InvalidWeekPatternId(*target);
                                    }
                                }
                                panic!(
                                    "Unexpected invariant breaks during AddNewIncompat: {set:?}"
                                );
                            }
                            _ => panic!("Unexpected error during AddNewIncompat: {e:?}"),
                        }
                    })?;
                let Some(collomatique_state_colloscopes::NewId::IncompatId(new_id)) = result else {
                    panic!("Unexpected result from IncompatOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateIncompat(incompat_id, incompat) => {
                let result = data
                    .try_apply(
                        collomatique_state_colloscopes::Op::Incompat(
                            collomatique_state_colloscopes::IncompatOp::Update(
                                *incompat_id,
                                incompat.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            ApplyError, FixableInvariant, IncompatPrecheckError, PrecheckError,
                            Reference, SubjectRefSite, WeekPatternRefSite,
                        };
                        match e {
                            ApplyError::Precheck(PrecheckError::Incompat(
                                IncompatPrecheckError::InvalidIncompatId(id),
                            )) => UpdateIncompatError::InvalidIncompatId(id),
                            // Old validator order: subject id before week pattern id.
                            ApplyError::Invariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Subject {
                                        target,
                                        site: SubjectRefSite::IncompatSubject(_),
                                    }) = inv
                                    {
                                        return UpdateIncompatError::InvalidSubjectId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::WeekPattern {
                                        target,
                                        site: WeekPatternRefSite::IncompatWeekPattern(_),
                                    }) = inv
                                    {
                                        return UpdateIncompatError::InvalidWeekPatternId(*target);
                                    }
                                }
                                panic!(
                                    "Unexpected invariant breaks during UpdateIncompat: {set:?}"
                                );
                            }
                            _ => panic!("Unexpected error during UpdateIncompat: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteIncompat(incompat_id) => {
                let result = data
                    .try_apply(
                        collomatique_state_colloscopes::Op::Incompat(
                            collomatique_state_colloscopes::IncompatOp::Remove(*incompat_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            ApplyError, IncompatPrecheckError, PrecheckError,
                        };
                        match e {
                            ApplyError::Precheck(PrecheckError::Incompat(
                                IncompatPrecheckError::InvalidIncompatId(id),
                            )) => DeleteIncompatError::InvalidIncompatId(id),
                            _ => panic!("Unexpected error during DeleteIncompat: {e:?}"),
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
