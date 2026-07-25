use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BalancingUpdateWarning {}

impl BalancingUpdateWarning {
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
pub enum BalancingUpdateOp {
    UpdateGlobalOptions(collomatique_state_colloscopes::balancing::BalancingOptions),
    UpdateSubjectOptions(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::balancing::BalancingOptions,
    ),
    RemoveSubjectOptions(collomatique_state_colloscopes::SubjectId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum BalancingUpdateError {
    #[error(transparent)]
    UpdateSubjectOptions(#[from] UpdateSubjectOptionsError),
    #[error(transparent)]
    RemoveSubjectOptions(#[from] RemoveSubjectOptionsError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateSubjectOptionsError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemoveSubjectOptionsError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("No options defined for subject {0:?}")]
    NoOptionsForSubject(collomatique_state_colloscopes::SubjectId),
}

impl BalancingUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<CleaningOp<BalancingUpdateWarning>> {
        match self {
            BalancingUpdateOp::UpdateGlobalOptions(_) => None,
            BalancingUpdateOp::UpdateSubjectOptions(_, _) => None,
            BalancingUpdateOp::RemoveSubjectOptions(_) => None,
        }
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<(), BalancingUpdateError> {
        match self {
            Self::UpdateGlobalOptions(options) => {
                let mut new_balancing = data.get_data().get_inner_data().params.balancing.clone();
                new_balancing.global = options.clone();

                let result = data
                    .try_apply(
                        collomatique_state_colloscopes::Op::Balancing(
                            collomatique_state_colloscopes::BalancingOp::Update(new_balancing),
                        ),
                        self.get_desc(),
                    )
                    .expect("BalancingOp::Update should never fail");

                assert!(result.is_none());

                Ok(())
            }
            Self::UpdateSubjectOptions(subject_id, options) => {
                if data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                    .is_none()
                {
                    return Err(UpdateSubjectOptionsError::InvalidSubjectId(*subject_id).into());
                }

                let mut new_balancing = data.get_data().get_inner_data().params.balancing.clone();
                new_balancing.subjects.insert(*subject_id, options.clone());

                let result = data
                    .try_apply(
                        collomatique_state_colloscopes::Op::Balancing(
                            collomatique_state_colloscopes::BalancingOp::Update(new_balancing),
                        ),
                        self.get_desc(),
                    )
                    .expect("BalancingOp::Update should not fail");

                assert!(result.is_none());

                Ok(())
            }
            Self::RemoveSubjectOptions(subject_id) => {
                if data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                    .is_none()
                {
                    return Err(RemoveSubjectOptionsError::InvalidSubjectId(*subject_id).into());
                }

                let mut new_balancing = data.get_data().get_inner_data().params.balancing.clone();
                if new_balancing.subjects.remove(subject_id).is_none() {
                    return Err(RemoveSubjectOptionsError::NoOptionsForSubject(*subject_id).into());
                }

                let result = data
                    .try_apply(
                        collomatique_state_colloscopes::Op::Balancing(
                            collomatique_state_colloscopes::BalancingOp::Update(new_balancing),
                        ),
                        self.get_desc(),
                    )
                    .expect("BalancingOp::Update should not fail");

                assert!(result.is_none());

                Ok(())
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Balancing,
            match self {
                BalancingUpdateOp::UpdateGlobalOptions(_) => {
                    "Mettre à jour les paramètres généraux d'équilibrage".into()
                }
                BalancingUpdateOp::UpdateSubjectOptions(_, _) => {
                    "Mettre à jour les paramètres d'équilibrage d'une matière".into()
                }
                BalancingUpdateOp::RemoveSubjectOptions(_) => {
                    "Supprimer les paramètres d'équilibrage d'une matière".into()
                }
            },
        )
    }
}
