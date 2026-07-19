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

/// Errors for export configuration operations
///
/// These errors can be returned when trying to modify [crate::Data] with an export config op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ExportConfigError {}

/// Precondition errors of the forced export-config op — the carve-out subset
/// (step-3 survey Table 2). Export config is pure value data with no guards of
/// any kind, so this enum is empty (as is [ExportConfigError]); kept for
/// uniformity across the [crate::PrecheckError] family.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ExportConfigPrecheckError {}

impl crate::Data {
    /// Used internally
    ///
    /// Apply export configuration operations
    pub(crate) fn apply_export_config(
        &mut self,
        export_config_op: &AnnotatedExportConfigOp,
    ) -> std::result::Result<AnnotatedExportConfigOp, ExportConfigError> {
        let backward = match export_config_op {
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
