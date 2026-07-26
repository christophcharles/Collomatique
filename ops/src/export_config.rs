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

        match self {
            Self::UpdateGlobalConfig(v) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::ExportConfig(
                            ExportConfigOp::UpdateGlobalConfig(v.clone()),
                        ),
                        self.get_desc(),
                    )
                    .expect("ExportConfigOp::UpdateGlobalConfig should never fail");
                assert!(result.is_none());
                Ok(())
            }
            Self::UpdateColloscopeEnabled(v) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::ExportConfig(
                            ExportConfigOp::UpdateColloscopeEnabled(*v),
                        ),
                        self.get_desc(),
                    )
                    .expect("ExportConfigOp::UpdateColloscopeEnabled should never fail");
                assert!(result.is_none());
                Ok(())
            }
            Self::UpdateAllGroupsEnabled(v) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::ExportConfig(
                            ExportConfigOp::UpdateAllGroupsEnabled(*v),
                        ),
                        self.get_desc(),
                    )
                    .expect("ExportConfigOp::UpdateAllGroupsEnabled should never fail");
                assert!(result.is_none());
                Ok(())
            }
            Self::UpdatePrefilledGroupsEnabled(v) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::ExportConfig(
                            ExportConfigOp::UpdatePrefilledGroupsEnabled(*v),
                        ),
                        self.get_desc(),
                    )
                    .expect("ExportConfigOp::UpdatePrefilledGroupsEnabled should never fail");
                assert!(result.is_none());
                Ok(())
            }
            Self::UpdateAutomaticGroupsEnabled(v) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::ExportConfig(
                            ExportConfigOp::UpdateAutomaticGroupsEnabled(*v),
                        ),
                        self.get_desc(),
                    )
                    .expect("ExportConfigOp::UpdateAutomaticGroupsEnabled should never fail");
                assert!(result.is_none());
                Ok(())
            }
            Self::UpdatePerGroupListEnabled(v) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::ExportConfig(
                            ExportConfigOp::UpdatePerGroupListEnabled(*v),
                        ),
                        self.get_desc(),
                    )
                    .expect("ExportConfigOp::UpdatePerGroupListEnabled should never fail");
                assert!(result.is_none());
                Ok(())
            }
            Self::UpdateColloscopeConfig(v) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::ExportConfig(
                            ExportConfigOp::UpdateColloscopeConfig(v.clone()),
                        ),
                        self.get_desc(),
                    )
                    .expect("ExportConfigOp::UpdateColloscopeConfig should never fail");
                assert!(result.is_none());
                Ok(())
            }
            Self::UpdateAllGroupsConfig(v) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::ExportConfig(
                            ExportConfigOp::UpdateAllGroupsConfig(v.clone()),
                        ),
                        self.get_desc(),
                    )
                    .expect("ExportConfigOp::UpdateAllGroupsConfig should never fail");
                assert!(result.is_none());
                Ok(())
            }
            Self::UpdatePrefilledGroupsConfig(v) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::ExportConfig(
                            ExportConfigOp::UpdatePrefilledGroupsConfig(v.clone()),
                        ),
                        self.get_desc(),
                    )
                    .expect("ExportConfigOp::UpdatePrefilledGroupsConfig should never fail");
                assert!(result.is_none());
                Ok(())
            }
            Self::UpdateAutomaticGroupsConfig(v) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::ExportConfig(
                            ExportConfigOp::UpdateAutomaticGroupsConfig(v.clone()),
                        ),
                        self.get_desc(),
                    )
                    .expect("ExportConfigOp::UpdateAutomaticGroupsConfig should never fail");
                assert!(result.is_none());
                Ok(())
            }
            Self::UpdatePerGroupListConfig(v) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::ExportConfig(
                            ExportConfigOp::UpdatePerGroupListConfig(v.clone()),
                        ),
                        self.get_desc(),
                    )
                    .expect("ExportConfigOp::UpdatePerGroupListConfig should never fail");
                assert!(result.is_none());
                Ok(())
            }
        }
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
