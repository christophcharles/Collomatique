use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SettingsUpdateWarning {}

impl SettingsUpdateWarning {
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
pub enum SettingsUpdateOp {
    UpdateGlobalLimits(collomatique_state_colloscopes::settings::Limits),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum SettingsUpdateError {}

impl SettingsUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<CleaningOp<WeekPatternsUpdateWarning>> {
        match self {
            SettingsUpdateOp::UpdateGlobalLimits(_) => None,
        }
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<(), SettingsUpdateError> {
        match self {
            Self::UpdateGlobalLimits(limits) => {
                let mut new_settings = data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .settings
                    .clone();
                new_settings.global = limits.clone();

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Settings(
                            collomatique_state_colloscopes::SettingsOp::Update(new_settings),
                        ),
                        self.get_desc(),
                    )
                    .expect("SettingsOp::Update should never fail");

                assert!(result.is_none());

                Ok(())
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Settings,
            match self {
                SettingsUpdateOp::UpdateGlobalLimits(_) => {
                    "Mettre à jour les paramètres généraux de limites".into()
                }
            },
        )
    }
}
