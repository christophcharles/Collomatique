use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ops::AnnotatedExportConfigOp;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageOrientation {
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub background_color: Color,
    pub stripes_color_enabled: bool,
    pub stripes_color: Color,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColloscopeConfig {
    pub sheet_name: String,
    pub extra_info_column_enabled: bool,
    pub extra_info_column_name: String,
    pub teacher_email_enabled: bool,
    pub teacher_email: String,
    pub teacher_tel_enabled: bool,
    pub teacher_tel: String,
    pub orientation: PageOrientation,
    pub display_week_dates: bool,
    pub display_annotations: bool,
    pub no_interrogation_color: Color,
    pub annotation_color_enabled: bool,
    pub annotation_color: Color,
    pub extra_colors: BTreeMap<String, Color>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerStudentGroupsConfig {
    pub sheet_name: String,
    /// `None` means auto-detect based on group count
    pub orientation: Option<PageOrientation>,
    pub show_emails: bool,
    pub show_tel: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerGroupListConfig {
    pub orientation: PageOrientation,
    pub show_emails: bool,
    pub show_tel: bool,
    pub center_vertically: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportConfig {
    pub global: GlobalConfig,
    pub colloscope_enabled: bool,
    pub all_groups_enabled: bool,
    pub automatic_groups_enabled: bool,
    pub prefilled_groups_enabled: bool,
    pub per_group_list_enabled: bool,
    pub colloscope_config: ColloscopeConfig,
    pub all_groups_config: PerStudentGroupsConfig,
    pub automatic_groups_config: PerStudentGroupsConfig,
    pub prefilled_groups_config: PerStudentGroupsConfig,
    pub per_group_list_config: PerGroupListConfig,
}

// One composite presentation preference: colors, orientations and toggles are
// choices, not content that can be added or removed, so the document order
// treats the whole configuration as one atom (plan step 6.5, decision 13).
// Two different configurations are incomparable — including the default
// against a modified one.
collomatique_state::impl_content_ord_atom!(ExportConfig);

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            background_color: Color {
                red: 255,
                green: 255,
                blue: 255,
            },
            stripes_color_enabled: true,
            stripes_color: Color {
                red: 220,
                green: 220,
                blue: 230,
            },
        }
    }
}

impl Default for ColloscopeConfig {
    fn default() -> Self {
        Self {
            sheet_name: "Colloscope".into(),
            extra_info_column_enabled: true,
            extra_info_column_name: "Info".into(),
            teacher_email_enabled: true,
            teacher_email: "Contact".into(),
            teacher_tel_enabled: false,
            teacher_tel: String::new(),
            orientation: PageOrientation::Landscape,
            display_week_dates: true,
            display_annotations: true,
            no_interrogation_color: Color {
                red: 140,
                green: 140,
                blue: 140,
            },
            annotation_color_enabled: true,
            annotation_color: Color {
                red: 255,
                green: 255,
                blue: 0,
            },
            extra_colors: BTreeMap::new(),
        }
    }
}

impl Default for PerGroupListConfig {
    fn default() -> Self {
        Self {
            orientation: PageOrientation::Portrait,
            show_emails: true,
            show_tel: false,
            center_vertically: false,
        }
    }
}

impl PerStudentGroupsConfig {
    pub fn default_all_groups() -> Self {
        Self {
            sheet_name: "Tous les groupes".into(),
            orientation: None,
            show_emails: true,
            show_tel: false,
        }
    }

    pub fn default_automatic_groups() -> Self {
        Self {
            sheet_name: "Groupes automatiques".into(),
            orientation: None,
            show_emails: true,
            show_tel: false,
        }
    }

    pub fn default_prefilled_groups() -> Self {
        Self {
            sheet_name: "Groupes préremplis".into(),
            orientation: None,
            show_emails: true,
            show_tel: false,
        }
    }
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            colloscope_enabled: true,
            all_groups_enabled: true,
            automatic_groups_enabled: false,
            prefilled_groups_enabled: false,
            per_group_list_enabled: true,
            colloscope_config: ColloscopeConfig::default(),
            all_groups_config: PerStudentGroupsConfig::default_all_groups(),
            automatic_groups_config: PerStudentGroupsConfig::default_automatic_groups(),
            prefilled_groups_config: PerStudentGroupsConfig::default_prefilled_groups(),
            per_group_list_config: PerGroupListConfig::default(),
        }
    }
}

/// Precondition errors of the forced export-config op — the carve-out subset
/// (step-3 survey Table 2). Export config is pure value data with no guards of
/// any kind, so this enum is empty; kept for
/// uniformity across the [crate::PrecheckError] family.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ExportConfigPrecheckError {}

impl crate::Data {
    /// Used internally by [crate::Data::force_apply]
    ///
    /// Force-applies an export-config op. Export config is pure value data
    /// with no guards of any kind, so this copy is byte-identical to the original
    /// and its [ExportConfigPrecheckError] is empty; kept for uniformity across
    /// the force_apply family.
    pub(crate) fn force_apply_export_config(
        &mut self,
        export_config_op: &AnnotatedExportConfigOp,
    ) -> std::result::Result<AnnotatedExportConfigOp, ExportConfigPrecheckError> {
        let backward = match export_config_op {
            AnnotatedExportConfigOp::Update(v) => {
                let old = std::mem::replace(&mut self.inner_data.export_config, v.clone());
                AnnotatedExportConfigOp::Update(old)
            }
            AnnotatedExportConfigOp::UpdateGlobalConfig(v) => {
                let old = std::mem::replace(&mut self.inner_data.export_config.global, v.clone());
                AnnotatedExportConfigOp::UpdateGlobalConfig(old)
            }
            AnnotatedExportConfigOp::UpdateColloscopeEnabled(v) => {
                let old =
                    std::mem::replace(&mut self.inner_data.export_config.colloscope_enabled, *v);
                AnnotatedExportConfigOp::UpdateColloscopeEnabled(old)
            }
            AnnotatedExportConfigOp::UpdateAllGroupsEnabled(v) => {
                let old =
                    std::mem::replace(&mut self.inner_data.export_config.all_groups_enabled, *v);
                AnnotatedExportConfigOp::UpdateAllGroupsEnabled(old)
            }
            AnnotatedExportConfigOp::UpdatePrefilledGroupsEnabled(v) => {
                let old = std::mem::replace(
                    &mut self.inner_data.export_config.prefilled_groups_enabled,
                    *v,
                );
                AnnotatedExportConfigOp::UpdatePrefilledGroupsEnabled(old)
            }
            AnnotatedExportConfigOp::UpdateAutomaticGroupsEnabled(v) => {
                let old = std::mem::replace(
                    &mut self.inner_data.export_config.automatic_groups_enabled,
                    *v,
                );
                AnnotatedExportConfigOp::UpdateAutomaticGroupsEnabled(old)
            }
            AnnotatedExportConfigOp::UpdatePerGroupListEnabled(v) => {
                let old = std::mem::replace(
                    &mut self.inner_data.export_config.per_group_list_enabled,
                    *v,
                );
                AnnotatedExportConfigOp::UpdatePerGroupListEnabled(old)
            }
            AnnotatedExportConfigOp::UpdateColloscopeConfig(v) => {
                let old = std::mem::replace(
                    &mut self.inner_data.export_config.colloscope_config,
                    v.clone(),
                );
                AnnotatedExportConfigOp::UpdateColloscopeConfig(old)
            }
            AnnotatedExportConfigOp::UpdateAllGroupsConfig(v) => {
                let old = std::mem::replace(
                    &mut self.inner_data.export_config.all_groups_config,
                    v.clone(),
                );
                AnnotatedExportConfigOp::UpdateAllGroupsConfig(old)
            }
            AnnotatedExportConfigOp::UpdatePrefilledGroupsConfig(v) => {
                let old = std::mem::replace(
                    &mut self.inner_data.export_config.prefilled_groups_config,
                    v.clone(),
                );
                AnnotatedExportConfigOp::UpdatePrefilledGroupsConfig(old)
            }
            AnnotatedExportConfigOp::UpdateAutomaticGroupsConfig(v) => {
                let old = std::mem::replace(
                    &mut self.inner_data.export_config.automatic_groups_config,
                    v.clone(),
                );
                AnnotatedExportConfigOp::UpdateAutomaticGroupsConfig(old)
            }
            AnnotatedExportConfigOp::UpdatePerGroupListConfig(v) => {
                let old = std::mem::replace(
                    &mut self.inner_data.export_config.per_group_list_config,
                    v.clone(),
                );
                AnnotatedExportConfigOp::UpdatePerGroupListConfig(old)
            }
        };
        Ok(backward)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Data;

    /// A configuration away from the default on every field, so a partial
    /// replace cannot pass the round-trip below by accident.
    fn non_default_config() -> ExportConfig {
        ExportConfig {
            global: GlobalConfig {
                background_color: Color {
                    red: 1,
                    green: 2,
                    blue: 3,
                },
                stripes_color_enabled: false,
                stripes_color: Color {
                    red: 4,
                    green: 5,
                    blue: 6,
                },
            },
            colloscope_enabled: false,
            all_groups_enabled: false,
            automatic_groups_enabled: true,
            prefilled_groups_enabled: true,
            per_group_list_enabled: false,
            colloscope_config: ColloscopeConfig {
                sheet_name: "Feuille".into(),
                extra_info_column_enabled: false,
                extra_info_column_name: "Notes".into(),
                teacher_email_enabled: false,
                teacher_email: "email".into(),
                teacher_tel_enabled: true,
                teacher_tel: "tel".into(),
                orientation: PageOrientation::Portrait,
                display_week_dates: false,
                display_annotations: false,
                no_interrogation_color: Color {
                    red: 7,
                    green: 8,
                    blue: 9,
                },
                annotation_color_enabled: false,
                annotation_color: Color {
                    red: 10,
                    green: 11,
                    blue: 12,
                },
                extra_colors: BTreeMap::from([(
                    "Vacances".to_string(),
                    Color {
                        red: 13,
                        green: 14,
                        blue: 15,
                    },
                )]),
            },
            all_groups_config: PerStudentGroupsConfig {
                sheet_name: "Tous".into(),
                orientation: Some(PageOrientation::Landscape),
                show_emails: false,
                show_tel: true,
            },
            automatic_groups_config: PerStudentGroupsConfig {
                sheet_name: "Auto".into(),
                orientation: Some(PageOrientation::Portrait),
                show_emails: false,
                show_tel: true,
            },
            prefilled_groups_config: PerStudentGroupsConfig {
                sheet_name: "Prérempli".into(),
                orientation: Some(PageOrientation::Landscape),
                show_emails: false,
                show_tel: true,
            },
            per_group_list_config: PerGroupListConfig {
                orientation: PageOrientation::Landscape,
                show_emails: false,
                show_tel: true,
                center_vertically: true,
            },
        }
    }

    /// The whole-struct op replaces the configuration, and the backward op it
    /// returns puts the previous one back verbatim — the reversibility pin for
    /// the new arm.
    #[test]
    fn update_replaces_the_whole_config_and_is_reversible() {
        let mut data = Data::default();
        let original = data.get_inner_data().export_config.clone();
        let new_config = non_default_config();
        assert_ne!(original, new_config);

        let backward = data
            .force_apply_export_config(&AnnotatedExportConfigOp::Update(new_config.clone()))
            .expect("export config has no preconditions");
        assert_eq!(data.get_inner_data().export_config, new_config);
        assert_eq!(backward, AnnotatedExportConfigOp::Update(original.clone()));

        data.force_apply_export_config(&backward)
            .expect("export config has no preconditions");
        assert_eq!(data.get_inner_data().export_config, original);
    }
}
