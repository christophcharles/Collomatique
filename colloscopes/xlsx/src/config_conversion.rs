//! Conversion from the document's stored export configuration to the resolved
//! configuration this crate consumes.
//!
//! The two types stay separate on purpose. `export_config::ExportConfig` keeps
//! its `*_enabled` flags beside the values they gate, so the interface can
//! remember what was chosen before a section was switched off. `Config` is the
//! resolved form with `Option<T>`, so a sheet builder cannot read a disabled
//! value by accident. This module is the step that resolves one into the other.

use collomatique_state_colloscopes::export_config;

fn optional<T>(enabled: bool, val: T) -> Option<T> {
    enabled.then_some(val)
}

impl From<&export_config::Color> for crate::Color {
    fn from(c: &export_config::Color) -> Self {
        crate::Color::new(c.red, c.green, c.blue)
    }
}

impl From<&export_config::PageOrientation> for crate::PageOrientation {
    fn from(o: &export_config::PageOrientation) -> Self {
        match o {
            export_config::PageOrientation::Portrait => crate::PageOrientation::Portrait,
            export_config::PageOrientation::Landscape => crate::PageOrientation::Landscape,
        }
    }
}

impl From<&export_config::PerStudentGroupsConfig> for crate::PerStudentGroupsConfig {
    fn from(psg: &export_config::PerStudentGroupsConfig) -> Self {
        crate::PerStudentGroupsConfig {
            sheet_name: psg.sheet_name.clone(),
            orientation: psg.orientation.as_ref().map(Into::into),
            show_emails: psg.show_emails,
            show_tel: psg.show_tel,
        }
    }
}

impl From<&export_config::ExportConfig> for crate::Config {
    fn from(ec: &export_config::ExportConfig) -> Self {
        crate::Config {
            global: crate::GlobalConfig {
                background_color: (&ec.global.background_color).into(),
                stripes_color: optional(
                    ec.global.stripes_color_enabled,
                    (&ec.global.stripes_color).into(),
                ),
            },
            colloscope: if ec.colloscope_enabled {
                let cc = &ec.colloscope_config;
                Some(crate::ColloscopeConfig {
                    sheet_name: cc.sheet_name.clone(),
                    extra_info_column_name: optional(
                        cc.extra_info_column_enabled,
                        cc.extra_info_column_name.clone(),
                    ),
                    teacher_email: optional(cc.teacher_email_enabled, cc.teacher_email.clone()),
                    teacher_tel: optional(cc.teacher_tel_enabled, cc.teacher_tel.clone()),
                    orientation: (&cc.orientation).into(),
                    display_week_dates: cc.display_week_dates,
                    display_annotations: cc.display_annotations,
                    no_interrogation_color: (&cc.no_interrogation_color).into(),
                    annotation_color: optional(
                        cc.annotation_color_enabled,
                        (&cc.annotation_color).into(),
                    ),
                    extra_colors: cc
                        .extra_colors
                        .iter()
                        .map(|(k, v)| (k.clone(), v.into()))
                        .collect(),
                })
            } else {
                None
            },
            all_groups: if ec.all_groups_enabled {
                Some((&ec.all_groups_config).into())
            } else {
                None
            },
            automatic_groups: if ec.automatic_groups_enabled {
                Some((&ec.automatic_groups_config).into())
            } else {
                None
            },
            prefilled_groups: if ec.prefilled_groups_enabled {
                Some((&ec.prefilled_groups_config).into())
            } else {
                None
            },
            per_group_list: if ec.per_group_list_enabled {
                let pgl = &ec.per_group_list_config;
                Some(crate::PerGroupListConfig {
                    orientation: (&pgl.orientation).into(),
                    show_emails: pgl.show_emails,
                    show_tel: pgl.show_tel,
                    center_vertically: pgl.center_vertically,
                })
            } else {
                None
            },
        }
    }
}
