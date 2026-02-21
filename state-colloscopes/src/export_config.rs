use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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
