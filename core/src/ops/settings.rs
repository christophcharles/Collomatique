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

#[derive(Debug, Clone)]
pub enum SettingsUpdateOp {
    UpdateStrictLimits(collomatique_state_colloscopes::settings::StrictLimits),
}

#[derive(Debug, Error)]
pub enum SettingsUpdateError {}

impl SettingsUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<CleaningOp<WeekPatternsUpdateWarning>> {
        match self {
            SettingsUpdateOp::UpdateStrictLimits(_) => None,
        }
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<(), SettingsUpdateError> {
        match self {
            Self::UpdateStrictLimits(strict_limits) => {
                let mut new_settings = data.get_data().get_settings().clone();
                new_settings.strict_limits = strict_limits.clone();

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
                SettingsUpdateOp::UpdateStrictLimits(_) => {
                    "Mettre à jour les paramètres de limites strictes".into()
                }
            },
        )
    }
}
