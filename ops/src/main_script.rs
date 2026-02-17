use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MainScriptUpdateWarning {}

impl MainScriptUpdateWarning {
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
pub enum MainScriptUpdateOp {
    UpdateScript(Option<String>),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum MainScriptUpdateError {}

impl MainScriptUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<CleaningOp<MainScriptUpdateWarning>> {
        None
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<(), MainScriptUpdateError> {
        match self {
            Self::UpdateScript(script) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::MainScript(
                            collomatique_state_colloscopes::MainScriptOp::Update(script.clone()),
                        ),
                        self.get_desc(),
                    )
                    .expect("MainScriptOp::Update should never fail");
                assert!(result.is_none());
                Ok(())
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::MainScript,
            match self {
                MainScriptUpdateOp::UpdateScript(Some(_)) => {
                    "Mettre à jour le script ColloML".into()
                }
                MainScriptUpdateOp::UpdateScript(None) => {
                    "Utiliser le script ColloML par défaut".into()
                }
            },
        )
    }
}
