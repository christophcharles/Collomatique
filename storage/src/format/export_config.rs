//! The `ExportConfig` block (spec §4.16)

use serde::{Deserialize, Serialize};

use super::keyed::{KeyedRow, KeyedVec};
use super::scalars::{Color, explicit_option};

/// Presentation settings for spreadsheet export
///
/// No field references ids; everything is local. The default is the full
/// record spelled out in the spec (§4.16), frozen forever.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

impl Default for ExportConfig {
    fn default() -> Self {
        ExportConfig {
            global: GlobalConfig::default(),
            colloscope_enabled: true,
            all_groups_enabled: true,
            automatic_groups_enabled: false,
            prefilled_groups_enabled: false,
            per_group_list_enabled: true,
            colloscope_config: ColloscopeConfig::default(),
            all_groups_config: PerStudentGroupsConfig {
                sheet_name: "Tous les groupes".to_string(),
                orientation: None,
                show_emails: true,
                show_tel: false,
            },
            automatic_groups_config: PerStudentGroupsConfig {
                sheet_name: "Groupes automatiques".to_string(),
                orientation: None,
                show_emails: true,
                show_tel: false,
            },
            prefilled_groups_config: PerStudentGroupsConfig {
                sheet_name: "Groupes préremplis".to_string(),
                orientation: None,
                show_emails: true,
                show_tel: false,
            },
            per_group_list_config: PerGroupListConfig::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfig {
    pub background_color: Color,
    pub stripes_color_enabled: bool,
    pub stripes_color: Color,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        GlobalConfig {
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

/// A page orientation, encoded as a bare string
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Orientation {
    Portrait,
    Landscape,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColloscopeConfig {
    pub sheet_name: String,
    pub extra_info_column_enabled: bool,
    pub extra_info_column_name: String,
    pub teacher_email_enabled: bool,
    pub teacher_email: String,
    pub teacher_tel_enabled: bool,
    pub teacher_tel: String,
    pub orientation: Orientation,
    pub display_week_dates: bool,
    pub display_annotations: bool,
    pub no_interrogation_color: Color,
    pub annotation_color_enabled: bool,
    pub annotation_color: Color,
    /// Maps annotation names to colors, keyed by `name`
    pub extra_colors: KeyedVec<ExtraColor>,
}

impl Default for ColloscopeConfig {
    fn default() -> Self {
        ColloscopeConfig {
            sheet_name: "Colloscope".to_string(),
            extra_info_column_enabled: true,
            extra_info_column_name: "Info".to_string(),
            teacher_email_enabled: true,
            teacher_email: "Contact".to_string(),
            teacher_tel_enabled: false,
            teacher_tel: String::new(),
            orientation: Orientation::Landscape,
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
            extra_colors: KeyedVec::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtraColor {
    pub name: String,
    pub color: Color,
}

impl KeyedRow for ExtraColor {
    type Key = String;

    fn key(&self) -> String {
        self.name.clone()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerStudentGroupsConfig {
    pub sheet_name: String,
    /// `null` means auto-detect from the group count
    #[serde(deserialize_with = "explicit_option")]
    pub orientation: Option<Orientation>,
    pub show_emails: bool,
    pub show_tel: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerGroupListConfig {
    pub orientation: Orientation,
    pub show_emails: bool,
    pub show_tel: bool,
    pub center_vertically: bool,
}

impl Default for PerGroupListConfig {
    fn default() -> Self {
        PerGroupListConfig {
            orientation: Orientation::Portrait,
            show_emails: true,
            show_tel: false,
            center_vertically: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec_default() -> serde_json::Value {
        json!({
            "global": {
                "background_color": { "red": 255, "green": 255, "blue": 255 },
                "stripes_color_enabled": true,
                "stripes_color": { "red": 220, "green": 220, "blue": 230 }
            },
            "colloscope_enabled": true,
            "all_groups_enabled": true,
            "automatic_groups_enabled": false,
            "prefilled_groups_enabled": false,
            "per_group_list_enabled": true,
            "colloscope_config": {
                "sheet_name": "Colloscope",
                "extra_info_column_enabled": true,
                "extra_info_column_name": "Info",
                "teacher_email_enabled": true,
                "teacher_email": "Contact",
                "teacher_tel_enabled": false,
                "teacher_tel": "",
                "orientation": "Landscape",
                "display_week_dates": true,
                "display_annotations": true,
                "no_interrogation_color": { "red": 140, "green": 140, "blue": 140 },
                "annotation_color_enabled": true,
                "annotation_color": { "red": 255, "green": 255, "blue": 0 },
                "extra_colors": []
            },
            "all_groups_config": {
                "sheet_name": "Tous les groupes",
                "orientation": null,
                "show_emails": true,
                "show_tel": false
            },
            "automatic_groups_config": {
                "sheet_name": "Groupes automatiques",
                "orientation": null,
                "show_emails": true,
                "show_tel": false
            },
            "prefilled_groups_config": {
                "sheet_name": "Groupes préremplis",
                "orientation": null,
                "show_emails": true,
                "show_tel": false
            },
            "per_group_list_config": {
                "orientation": "Portrait",
                "show_emails": true,
                "show_tel": false,
                "center_vertically": false
            }
        })
    }

    #[test]
    fn default_is_pinned() {
        assert_eq!(
            serde_json::to_value(ExportConfig::default()).unwrap(),
            spec_default()
        );
    }

    #[test]
    fn spec_default_round_trips() {
        let block: ExportConfig = serde_json::from_value(spec_default()).unwrap();
        assert_eq!(block, ExportConfig::default());
        assert_eq!(serde_json::to_value(&block).unwrap(), spec_default());
    }

    #[test]
    fn orientation_is_a_bare_string() {
        let orientation: Orientation = serde_json::from_value(json!("Portrait")).unwrap();
        assert_eq!(orientation, Orientation::Portrait);
        assert_eq!(
            serde_json::to_value(Orientation::Landscape).unwrap(),
            json!("Landscape")
        );
        assert!(serde_json::from_value::<Orientation>(json!("landscape")).is_err());
    }

    #[test]
    fn extra_colors_reject_duplicate_names() {
        let value = json!([
            { "name": "DS", "color": { "red": 0, "green": 0, "blue": 255 } },
            { "name": "DS", "color": { "red": 255, "green": 0, "blue": 0 } }
        ]);
        assert!(serde_json::from_value::<KeyedVec<ExtraColor>>(value).is_err());
    }

    #[test]
    fn extra_colors_with_distinct_names_round_trip() {
        let value = json!([
            { "name": "DS", "color": { "red": 0, "green": 0, "blue": 255 } },
            { "name": "TP", "color": { "red": 255, "green": 0, "blue": 0 } }
        ]);
        let colors: KeyedVec<ExtraColor> = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(&colors).unwrap(), value);
    }

    #[test]
    fn missing_field_is_rejected() {
        let mut value = spec_default();
        value.as_object_mut().unwrap().remove("colloscope_config");
        assert!(serde_json::from_value::<ExportConfig>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        let mut value = spec_default();
        value
            .as_object_mut()
            .unwrap()
            .insert("extra".to_string(), json!(1));
        assert!(serde_json::from_value::<ExportConfig>(value).is_err());
    }

    // The block's only `Option` field, carried by the three
    // [PerStudentGroupsConfig] sections. It rejects a missing key solely
    // because of its `explicit_option` attribute; without it serde would
    // silently default to `None`, i.e. to "auto-detect the orientation".
    #[test]
    fn missing_per_student_groups_orientation_is_rejected() {
        for section in [
            "all_groups_config",
            "automatic_groups_config",
            "prefilled_groups_config",
        ] {
            let mut value = spec_default();
            value
                .as_object_mut()
                .unwrap()
                .get_mut(section)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove("orientation");
            assert!(
                serde_json::from_value::<ExportConfig>(value).is_err(),
                "a missing orientation in {section} should be rejected"
            );
        }
    }
}
