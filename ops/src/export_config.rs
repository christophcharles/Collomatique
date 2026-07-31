use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExportConfigUpdateWarning {}

impl ExportConfigUpdateWarning {
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
pub enum ExportConfigUpdateOp {
    UpdateGlobalConfig(collomatique_state_colloscopes::export_config::GlobalConfig),
    UpdateColloscopeEnabled(bool),
    UpdateAllGroupsEnabled(bool),
    UpdatePrefilledGroupsEnabled(bool),
    UpdateAutomaticGroupsEnabled(bool),
    UpdatePerGroupListEnabled(bool),
    UpdateColloscopeConfig(collomatique_state_colloscopes::export_config::ColloscopeConfig),
    UpdateAllGroupsConfig(collomatique_state_colloscopes::export_config::PerStudentGroupsConfig),
    UpdatePrefilledGroupsConfig(
        collomatique_state_colloscopes::export_config::PerStudentGroupsConfig,
    ),
    UpdateAutomaticGroupsConfig(
        collomatique_state_colloscopes::export_config::PerStudentGroupsConfig,
    ),
    UpdatePerGroupListConfig(collomatique_state_colloscopes::export_config::PerGroupListConfig),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExportConfigUpdateError {}

impl ExportConfigUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<CleaningOp<ExportConfigUpdateWarning>> {
        None
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<(), ExportConfigUpdateError> {
        use collomatique_state_colloscopes::ExportConfigOp;

        // The eleven variants are the user-facing granularity (each carries its
        // own history description); the elementary op is one whole-struct
        // replace, so every variant reads the current config, patches its one
        // field and issues that single op.
        let mut new_config = data.get_data().get_inner_data().export_config.clone();
        match self {
            Self::UpdateGlobalConfig(v) => new_config.global = v.clone(),
            Self::UpdateColloscopeEnabled(v) => new_config.colloscope_enabled = *v,
            Self::UpdateAllGroupsEnabled(v) => new_config.all_groups_enabled = *v,
            Self::UpdatePrefilledGroupsEnabled(v) => new_config.prefilled_groups_enabled = *v,
            Self::UpdateAutomaticGroupsEnabled(v) => new_config.automatic_groups_enabled = *v,
            Self::UpdatePerGroupListEnabled(v) => new_config.per_group_list_enabled = *v,
            Self::UpdateColloscopeConfig(v) => new_config.colloscope_config = v.clone(),
            Self::UpdateAllGroupsConfig(v) => new_config.all_groups_config = v.clone(),
            Self::UpdatePrefilledGroupsConfig(v) => new_config.prefilled_groups_config = v.clone(),
            Self::UpdateAutomaticGroupsConfig(v) => new_config.automatic_groups_config = v.clone(),
            Self::UpdatePerGroupListConfig(v) => new_config.per_group_list_config = v.clone(),
        }

        let result = data
            .apply(
                collomatique_state_colloscopes::Op::ExportConfig(ExportConfigOp::Update(
                    new_config,
                )),
                self.get_desc(),
            )
            .expect("ExportConfigOp::Update should never fail");
        assert!(result.is_none());
        Ok(())
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::ExportConfig,
            match self {
                ExportConfigUpdateOp::UpdateGlobalConfig(_) => {
                    "Mettre à jour la configuration globale d'export".into()
                }
                ExportConfigUpdateOp::UpdateColloscopeEnabled(_) => {
                    "Mettre à jour l'activation de l'export du colloscope".into()
                }
                ExportConfigUpdateOp::UpdateAllGroupsEnabled(_) => {
                    "Mettre à jour l'activation de l'export de tous les groupes".into()
                }
                ExportConfigUpdateOp::UpdatePrefilledGroupsEnabled(_) => {
                    "Mettre à jour l'activation de l'export des groupes préremplis".into()
                }
                ExportConfigUpdateOp::UpdateAutomaticGroupsEnabled(_) => {
                    "Mettre à jour l'activation de l'export des groupes automatiques".into()
                }
                ExportConfigUpdateOp::UpdatePerGroupListEnabled(_) => {
                    "Mettre à jour l'activation de l'export de la liste par groupe".into()
                }
                ExportConfigUpdateOp::UpdateColloscopeConfig(_) => {
                    "Mettre à jour la configuration d'export du colloscope".into()
                }
                ExportConfigUpdateOp::UpdateAllGroupsConfig(_) => {
                    "Mettre à jour la configuration d'export de tous les groupes".into()
                }
                ExportConfigUpdateOp::UpdatePrefilledGroupsConfig(_) => {
                    "Mettre à jour la configuration d'export des groupes préremplis".into()
                }
                ExportConfigUpdateOp::UpdateAutomaticGroupsConfig(_) => {
                    "Mettre à jour la configuration d'export des groupes automatiques".into()
                }
                ExportConfigUpdateOp::UpdatePerGroupListConfig(_) => {
                    "Mettre à jour la configuration d'export de la liste par groupe".into()
                }
            },
        )
    }
}
