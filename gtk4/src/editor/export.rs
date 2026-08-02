use collomatique_state_colloscopes::export_config;

fn to_xlsx_color(c: &export_config::Color) -> collomatique_xlsx::Color {
    collomatique_xlsx::Color::new(c.red, c.green, c.blue)
}

fn optional<T>(enabled: bool, val: T) -> Option<T> {
    enabled.then_some(val)
}

fn to_xlsx_orientation(o: &export_config::PageOrientation) -> collomatique_xlsx::PageOrientation {
    match o {
        export_config::PageOrientation::Portrait => collomatique_xlsx::PageOrientation::Portrait,
        export_config::PageOrientation::Landscape => collomatique_xlsx::PageOrientation::Landscape,
    }
}

pub fn to_xlsx_config(ec: &export_config::ExportConfig) -> collomatique_xlsx::Config {
    collomatique_xlsx::Config {
        global: collomatique_xlsx::GlobalConfig {
            background_color: to_xlsx_color(&ec.global.background_color),
            stripes_color: optional(
                ec.global.stripes_color_enabled,
                to_xlsx_color(&ec.global.stripes_color),
            ),
        },
        colloscope: if ec.colloscope_enabled {
            let cc = &ec.colloscope_config;
            Some(collomatique_xlsx::ColloscopeConfig {
                sheet_name: cc.sheet_name.clone(),
                extra_info_column_name: optional(
                    cc.extra_info_column_enabled,
                    cc.extra_info_column_name.clone(),
                ),
                teacher_email: optional(cc.teacher_email_enabled, cc.teacher_email.clone()),
                teacher_tel: optional(cc.teacher_tel_enabled, cc.teacher_tel.clone()),
                orientation: to_xlsx_orientation(&cc.orientation),
                display_week_dates: cc.display_week_dates,
                display_annotations: cc.display_annotations,
                no_interrogation_color: to_xlsx_color(&cc.no_interrogation_color),
                annotation_color: optional(
                    cc.annotation_color_enabled,
                    to_xlsx_color(&cc.annotation_color),
                ),
                extra_colors: cc
                    .extra_colors
                    .iter()
                    .map(|(k, v)| (k.clone(), to_xlsx_color(v)))
                    .collect(),
            })
        } else {
            None
        },
        all_groups: if ec.all_groups_enabled {
            Some(to_xlsx_per_student_groups(&ec.all_groups_config))
        } else {
            None
        },
        automatic_groups: if ec.automatic_groups_enabled {
            Some(to_xlsx_per_student_groups(&ec.automatic_groups_config))
        } else {
            None
        },
        prefilled_groups: if ec.prefilled_groups_enabled {
            Some(to_xlsx_per_student_groups(&ec.prefilled_groups_config))
        } else {
            None
        },
        per_group_list: if ec.per_group_list_enabled {
            let pgl = &ec.per_group_list_config;
            Some(collomatique_xlsx::PerGroupListConfig {
                orientation: to_xlsx_orientation(&pgl.orientation),
                show_emails: pgl.show_emails,
                show_tel: pgl.show_tel,
                center_vertically: pgl.center_vertically,
            })
        } else {
            None
        },
    }
}

fn to_xlsx_per_student_groups(
    psg: &export_config::PerStudentGroupsConfig,
) -> collomatique_xlsx::PerStudentGroupsConfig {
    collomatique_xlsx::PerStudentGroupsConfig {
        sheet_name: psg.sheet_name.clone(),
        orientation: psg.orientation.as_ref().map(to_xlsx_orientation),
        show_emails: psg.show_emails,
        show_tel: psg.show_tel,
    }
}

pub fn export_to_xlsx(
    data: &collomatique_state_colloscopes::InnerData,
    path: &std::path::Path,
    xlsx_config: &collomatique_xlsx::Config,
) -> Result<(), anyhow::Error> {
    collomatique_xlsx::write_xlsx(data, path, xlsx_config)
        .map_err(|e| anyhow::anyhow!("Failed to write XLSX: {e}"))?;
    Ok(())
}
