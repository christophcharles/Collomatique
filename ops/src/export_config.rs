use super::*;

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
    pub(crate) fn apply_to_session<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<(), ExportConfigUpdateError> {
        use collomatique_state_colloscopes::ExportConfigOp;

        // The eleven variants are the user-facing granularity (each carries its
        // own history description); the elementary op is one whole-struct
        // replace, so every variant reads the current config, patches its one
        // field and issues that single op.
        //
        // The read is on the *session*: in a composite, the config to patch is
        // the one the previous ops left behind, not the one the composite
        // started on.
        let mut new_config = session.get_data().get_inner_data().export_config.clone();
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

        // Export config is pure presentation data: it references no entity, so
        // it can neither be rejected nor make the cascade repair anything.
        let result = session
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

#[cfg(test)]
mod tests {
    //! The export config references no entity, so the whole family is a
    //! read-patch-write over one struct: a document with nothing in it says
    //! everything these tests have to say, and the frozen hogwarts base
    //! (`tests/fixtures/`) would only add noise.

    use super::*;
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::export_config::{
        Color, ExportConfig, GlobalConfig, PageOrientation, PerGroupListConfig,
        PerStudentGroupsConfig,
    };

    fn empty_document() -> CascadeSession<AppState<Data, Desc>> {
        CascadeSession::new(AppState::new(Data::default()))
    }

    fn export_config_of<T: Manager<Data = Data, Desc = Desc>>(state: &T) -> ExportConfig {
        state.get_data().get_inner_data().export_config.clone()
    }

    /// Applies `op` alone to an empty document and hands back what the document
    /// ended up with, warnings included.
    fn apply_alone(op: &ExportConfigUpdateOp) -> (ExportConfig, Vec<CascadeWarning>) {
        let mut session = empty_document();
        op.apply_to_session(&mut session)
            .expect("an export config op cannot be rejected");
        let (state, warnings) = session.commit(op.get_desc());

        (export_config_of(&state), warnings)
    }

    /// Each of the eleven user-facing variants writes its own field and copies
    /// the ten others over untouched — the point of reading the current config
    /// before issuing the whole-struct op. And no variant can warn: the config
    /// names no entity, so the cascade has nothing to repair.
    #[test]
    fn every_variant_patches_its_own_field_and_leaves_the_rest_alone() {
        let base = ExportConfig::default();

        // A distinguishable value for every field: flipped booleans, and
        // structs the defaults cannot be mistaken for.
        let global = GlobalConfig {
            background_color: Color {
                red: 1,
                green: 2,
                blue: 3,
            },
            ..GlobalConfig::default()
        };
        let colloscope_config = collomatique_state_colloscopes::export_config::ColloscopeConfig {
            sheet_name: "Colles".into(),
            ..Default::default()
        };
        let per_student_config = PerStudentGroupsConfig {
            sheet_name: "Groupes".into(),
            orientation: Some(PageOrientation::Landscape),
            show_emails: false,
            show_tel: true,
        };
        let per_group_list_config = PerGroupListConfig {
            center_vertically: true,
            ..PerGroupListConfig::default()
        };

        let cases = [
            (
                ExportConfigUpdateOp::UpdateGlobalConfig(global.clone()),
                ExportConfig {
                    global: global.clone(),
                    ..base.clone()
                },
            ),
            (
                ExportConfigUpdateOp::UpdateColloscopeEnabled(!base.colloscope_enabled),
                ExportConfig {
                    colloscope_enabled: !base.colloscope_enabled,
                    ..base.clone()
                },
            ),
            (
                ExportConfigUpdateOp::UpdateAllGroupsEnabled(!base.all_groups_enabled),
                ExportConfig {
                    all_groups_enabled: !base.all_groups_enabled,
                    ..base.clone()
                },
            ),
            (
                ExportConfigUpdateOp::UpdatePrefilledGroupsEnabled(!base.prefilled_groups_enabled),
                ExportConfig {
                    prefilled_groups_enabled: !base.prefilled_groups_enabled,
                    ..base.clone()
                },
            ),
            (
                ExportConfigUpdateOp::UpdateAutomaticGroupsEnabled(!base.automatic_groups_enabled),
                ExportConfig {
                    automatic_groups_enabled: !base.automatic_groups_enabled,
                    ..base.clone()
                },
            ),
            (
                ExportConfigUpdateOp::UpdatePerGroupListEnabled(!base.per_group_list_enabled),
                ExportConfig {
                    per_group_list_enabled: !base.per_group_list_enabled,
                    ..base.clone()
                },
            ),
            (
                ExportConfigUpdateOp::UpdateColloscopeConfig(colloscope_config.clone()),
                ExportConfig {
                    colloscope_config: colloscope_config.clone(),
                    ..base.clone()
                },
            ),
            (
                ExportConfigUpdateOp::UpdateAllGroupsConfig(per_student_config.clone()),
                ExportConfig {
                    all_groups_config: per_student_config.clone(),
                    ..base.clone()
                },
            ),
            (
                ExportConfigUpdateOp::UpdatePrefilledGroupsConfig(per_student_config.clone()),
                ExportConfig {
                    prefilled_groups_config: per_student_config.clone(),
                    ..base.clone()
                },
            ),
            (
                ExportConfigUpdateOp::UpdateAutomaticGroupsConfig(per_student_config.clone()),
                ExportConfig {
                    automatic_groups_config: per_student_config.clone(),
                    ..base.clone()
                },
            ),
            (
                ExportConfigUpdateOp::UpdatePerGroupListConfig(per_group_list_config.clone()),
                ExportConfig {
                    per_group_list_config: per_group_list_config.clone(),
                    ..base.clone()
                },
            ),
        ];

        for (op, expected) in &cases {
            let (config, warnings) = apply_alone(op);

            assert!(
                warnings.is_empty(),
                "the export config references nothing, so {op:?} cannot warn: {warnings:?}"
            );
            assert_eq!(config, *expected, "unexpected result for {op:?}");
        }
    }

    /// Two ops in one session: the second reads what the first left behind, so
    /// both patches survive. Reading the *pre-state* instead would silently
    /// undo the first one.
    #[test]
    fn a_second_op_patches_the_config_the_first_one_left() {
        let base = ExportConfig::default();

        let mut session = empty_document();
        ExportConfigUpdateOp::UpdateColloscopeEnabled(!base.colloscope_enabled)
            .apply_to_session(&mut session)
            .expect("an export config op cannot be rejected");
        ExportConfigUpdateOp::UpdateAllGroupsEnabled(!base.all_groups_enabled)
            .apply_to_session(&mut session)
            .expect("an export config op cannot be rejected");
        let (state, warnings) = session.commit((OpCategory::ExportConfig, "Deux réglages".into()));

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            export_config_of(&state),
            ExportConfig {
                colloscope_enabled: !base.colloscope_enabled,
                all_groups_enabled: !base.all_groups_enabled,
                ..base
            },
        );
    }
}
