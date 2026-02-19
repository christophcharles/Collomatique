use gtk::prelude::{BoxExt, ButtonExt, WidgetExt};
use relm4::gtk::prelude::OrientableExt;
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};
use relm4::{adw, gtk};
use std::path::PathBuf;

use collomatique_state_colloscopes::export_config;

use crate::tools;

pub struct ExportPanel {
    export_config: export_config::ExportConfig,
    file_name: Option<PathBuf>,
}

#[derive(Debug)]
pub enum ExportPanelInput {
    Update(export_config::ExportConfig, Option<PathBuf>),
    ExportClicked,
    ExportSqliteClicked,

    UpdateColloscopeEnabled(bool),
    UpdateAllGroupsEnabled(bool),
    UpdatePrefilledEnabled(bool),
    UpdateAutomaticEnabled(bool),
    UpdatePerGroupListEnabled(bool),

    RestoreDefaultGeneralConfigClicked,
    RestoreDefaultColloscopeConfigClicked,
    RestoreDefaultAllGroupsConfigClicked,
    RestoreDefaultPrefilledConfigClicked,
    RestoreDefaultAutomaticConfigClicked,
    RestoreDefaultPerGroupListConfigClicked,
}

#[derive(Debug)]
pub enum ExportPanelOutput {
    UpdateExportConfig(collomatique_ops::ExportConfigUpdateOp),
    ExportColloscopeAs(PathBuf, collomatique_xlsx::Config),
    ExportSqliteAs(PathBuf),
}

#[derive(Debug)]
pub enum ExportPanelCommandOutput {
    FileChosen(PathBuf),
    FileNotChosen,
    SqliteFileChosen(PathBuf),
    SqliteFileNotChosen,
}

#[relm4::component(pub)]
impl Component for ExportPanel {
    type Input = ExportPanelInput;
    type Output = ExportPanelOutput;
    type Init = ();
    type CommandOutput = ExportPanelCommandOutput;

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hexpand: true,
            set_vexpand: true,
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_hexpand: true,
                set_spacing: 30,
                gtk::Box {
                    set_hexpand: true,
                    set_spacing: 5,
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Label {
                        set_label: "<b><i><big>Configuration globale</big></i></b>",
                        set_use_markup: true,
                        set_margin_all: 5,
                    },
                },
                // Section: Couleurs de fond
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 5,
                        gtk::Label {
                            set_label: "Couleurs de fond",
                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                            set_margin_all: 5,
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Button {
                            set_icon_name: "edit-delete-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Restaurer les valeurs par défaut"),
                            #[watch]
                            set_sensitive: model.export_config.global != collomatique_state_colloscopes::export_config::GlobalConfig::default(),
                            connect_clicked => ExportPanelInput::RestoreDefaultGeneralConfigClicked,
                        },
                    },
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_use_markup: true,
                        set_label: "<i>Configuration à venir...</i>",
                        set_margin_all: 5,
                    },
                },
                gtk::Box {
                    set_hexpand: true,
                    set_spacing: 5,
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Label {
                        set_label: "<b><i><big>Feuilles à inclure</big></i></b>",
                        set_use_markup: true,
                        set_margin_all: 5,
                    },
                },
                // Section: Colloscope
                #[name(colloscope_box)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 5,
                        #[name(colloscope_switch)]
                        gtk::Switch {
                            set_margin_start: 10,
                            set_valign: gtk::Align::Center,
                            #[watch]
                            set_tooltip: if model.export_config.colloscope_enabled {
                                "Exclure la section de l'export"
                            } else {
                                "Inclure la section dans l'export"
                            },
                            #[track(model.export_config.colloscope_enabled != colloscope_switch.is_active())]
                            set_active: model.export_config.colloscope_enabled,
                            connect_state_set[sender] => move |_widget,state| {
                                sender.input(ExportPanelInput::UpdateColloscopeEnabled(state));
                                gtk::glib::Propagation::Proceed
                            },
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: "Colloscope",
                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                            set_margin_all: 5,
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Button {
                            set_icon_name: "edit-delete-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Restaurer les valeurs par défaut"),
                            #[watch]
                            set_visible: model.export_config.colloscope_enabled,
                            #[watch]
                            set_sensitive: model.export_config.colloscope_config != collomatique_state_colloscopes::export_config::ColloscopeConfig::default(),
                            connect_clicked => ExportPanelInput::RestoreDefaultColloscopeConfigClicked,
                        },
                    },
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_use_markup: true,
                        set_label: "<i>Configuration à venir...</i>",
                        set_margin_all: 5,
                        #[watch]
                        set_visible: model.export_config.colloscope_enabled,
                    },
                },
                // Section: Tous les groupes
                #[name(all_groups_box)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 5,
                        #[name(all_groups_switch)]
                        gtk::Switch {
                            set_margin_start: 10,
                            set_valign: gtk::Align::Center,
                            #[watch]
                            set_tooltip: if model.export_config.all_groups_enabled {
                                "Exclure la section de l'export"
                            } else {
                                "Inclure la section dans l'export"
                            },
                            #[track(model.export_config.all_groups_enabled != all_groups_switch.is_active())]
                            set_active: model.export_config.all_groups_enabled,
                            connect_state_set[sender] => move |_widget,state| {
                                sender.input(ExportPanelInput::UpdateAllGroupsEnabled(state));
                                gtk::glib::Propagation::Proceed
                            },
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: "Tous les groupes",
                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                            set_margin_all: 5,
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Button {
                            set_icon_name: "edit-delete-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Restaurer les valeurs par défaut"),
                            #[watch]
                            set_visible: model.export_config.all_groups_enabled,
                            #[watch]
                            set_sensitive: model.export_config.all_groups_config != collomatique_state_colloscopes::export_config::PerStudentGroupsConfig::default_all_groups(),
                            connect_clicked => ExportPanelInput::RestoreDefaultAllGroupsConfigClicked,
                        },
                    },
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_use_markup: true,
                        set_label: "<i>Configuration à venir...</i>",
                        set_margin_all: 5,
                        #[watch]
                        set_visible: model.export_config.all_groups_enabled,
                    },
                },
                // Section: Groupes préremplis
                #[name(prefilled_box)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 5,
                        #[name(prefilled_switch)]
                        gtk::Switch {
                            set_margin_start: 10,
                            set_valign: gtk::Align::Center,
                            #[watch]
                            set_tooltip: if model.export_config.prefilled_groups_enabled {
                                "Exclure la section de l'export"
                            } else {
                                "Inclure la section dans l'export"
                            },
                            #[track(model.export_config.prefilled_groups_enabled != prefilled_switch.is_active())]
                            set_active: model.export_config.prefilled_groups_enabled,
                            connect_state_set[sender] => move |_widget,state| {
                                sender.input(ExportPanelInput::UpdatePrefilledEnabled(state));
                                gtk::glib::Propagation::Proceed
                            },
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: "Groupes préremplis",
                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                            set_margin_all: 5,
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Button {
                            set_icon_name: "edit-delete-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Restaurer les valeurs par défaut"),
                            #[watch]
                            set_visible: model.export_config.prefilled_groups_enabled,
                            #[watch]
                            set_sensitive: model.export_config.prefilled_groups_config != collomatique_state_colloscopes::export_config::PerStudentGroupsConfig::default_prefilled_groups(),
                            connect_clicked => ExportPanelInput::RestoreDefaultPrefilledConfigClicked,
                        },
                    },
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_use_markup: true,
                        set_label: "<i>Configuration à venir...</i>",
                        set_margin_all: 5,
                        #[watch]
                        set_visible: model.export_config.prefilled_groups_enabled,
                    },
                },
                // Section: Groupes automatiques
                #[name(automatic_box)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 5,
                        #[name(automatic_switch)]
                        gtk::Switch {
                            set_margin_start: 10,
                            set_valign: gtk::Align::Center,
                            #[watch]
                            set_tooltip: if model.export_config.automatic_groups_enabled {
                                "Exclure la section de l'export"
                            } else {
                                "Inclure la section dans l'export"
                            },
                            #[track(model.export_config.automatic_groups_enabled != automatic_switch.is_active())]
                            set_active: model.export_config.automatic_groups_enabled,
                            connect_state_set[sender] => move |_widget,state| {
                                sender.input(ExportPanelInput::UpdateAutomaticEnabled(state));
                                gtk::glib::Propagation::Proceed
                            },
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: "Groupes automatiques",
                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                            set_margin_all: 5,
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Button {
                            set_icon_name: "edit-delete-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Restaurer les valeurs par défaut"),
                            #[watch]
                            set_visible: model.export_config.automatic_groups_enabled,
                            #[watch]
                            set_sensitive: model.export_config.automatic_groups_config != collomatique_state_colloscopes::export_config::PerStudentGroupsConfig::default_automatic_groups(),
                            connect_clicked => ExportPanelInput::RestoreDefaultAutomaticConfigClicked,
                        },
                    },
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_use_markup: true,
                        set_label: "<i>Configuration à venir...</i>",
                        set_margin_all: 5,
                        #[watch]
                        set_visible: model.export_config.automatic_groups_enabled,
                    },
                },
                // Section: Par liste de groupes
                #[name(per_group_list_box)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 5,
                        #[name(per_group_list_switch)]
                        gtk::Switch {
                            set_margin_start: 10,
                            set_valign: gtk::Align::Center,
                            #[watch]
                            set_tooltip: if model.export_config.per_group_list_enabled {
                                "Exclure la section de l'export"
                            } else {
                                "Inclure la section dans l'export"
                            },
                            #[track(model.export_config.per_group_list_enabled != per_group_list_switch.is_active())]
                            set_active: model.export_config.per_group_list_enabled,
                            connect_state_set[sender] => move |_widget,state| {
                                sender.input(ExportPanelInput::UpdatePerGroupListEnabled(state));
                                gtk::glib::Propagation::Proceed
                            },
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: "Par liste de groupes",
                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                            set_margin_all: 5,
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Button {
                            set_icon_name: "edit-delete-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Restaurer les valeurs par défaut"),
                            #[watch]
                            set_visible: model.export_config.per_group_list_enabled,
                            #[watch]
                            set_sensitive: model.export_config.per_group_list_config != collomatique_state_colloscopes::export_config::PerGroupListConfig::default(),
                            connect_clicked => ExportPanelInput::RestoreDefaultPerGroupListConfigClicked,
                        },
                    },
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_use_markup: true,
                        set_label: "<i>Configuration à venir...</i>",
                        set_margin_all: 5,
                        #[watch]
                        set_visible: model.export_config.per_group_list_enabled,
                    },
                },
                gtk::Button {
                    add_css_class: "frame",
                    add_css_class: "accent",
                    set_hexpand: true,
                    set_margin_all: 5,
                    adw::ButtonContent {
                        set_icon_name: "document-export-symbolic",
                        set_label: "Exporter le colloscope",
                    },
                    connect_clicked => ExportPanelInput::ExportClicked,
                },
                gtk::Box {
                    set_hexpand: true,
                    set_spacing: 5,
                    set_margin_top: 40,
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Separator {
                        set_orientation: gtk::Orientation::Horizontal,
                    },
                    gtk::Label {
                        set_label: "<b><i><big>Options de débogage</big></i></b>",
                        set_use_markup: true,
                        set_margin_all: 5,
                    },
                    gtk::Button {
                        add_css_class: "frame",
                        add_css_class: "warning",
                        set_hexpand: true,
                        set_margin_all: 5,
                        adw::ButtonContent {
                            set_icon_name: "document-export-symbolic",
                            set_label: "Exporter la base de donnée SQL",
                        },
                        connect_clicked => ExportPanelInput::ExportSqliteClicked,
                    },
                },
            },
        }
    }

    fn init(
        _params: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ExportPanel {
            export_config: export_config::ExportConfig::default(),
            file_name: None,
        };
        let widgets = view_output!();

        if !model.export_config.colloscope_enabled {
            widgets.colloscope_box.add_css_class("dimmed");
        }
        if !model.export_config.all_groups_enabled {
            widgets.all_groups_box.add_css_class("dimmed");
        }
        if !model.export_config.prefilled_groups_enabled {
            widgets.prefilled_box.add_css_class("dimmed");
        }
        if !model.export_config.automatic_groups_enabled {
            widgets.automatic_box.add_css_class("dimmed");
        }
        if !model.export_config.per_group_list_enabled {
            widgets.per_group_list_box.add_css_class("dimmed");
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            ExportPanelInput::Update(config, file_name) => {
                self.export_config = config;
                self.file_name = file_name;
            }
            ExportPanelInput::ExportClicked => {
                let default = match &self.file_name {
                    Some(path) => {
                        let mut xlsx_path = path.clone();
                        xlsx_path.set_extension("xlsx");
                        tools::open_save::DefaultSaveFile::ExistingFile(xlsx_path)
                    }
                    None => tools::open_save::DefaultSaveFile::SuggestedName(
                        format!("{}.xlsx", super::DEFAULT_FILE_STEM).into(),
                    ),
                };
                sender.oneshot_command(async move {
                    match tools::open_save::save_xlsx_dialog(default).await {
                        Some(path) => ExportPanelCommandOutput::FileChosen(path),
                        None => ExportPanelCommandOutput::FileNotChosen,
                    }
                });
            }
            ExportPanelInput::ExportSqliteClicked => {
                let default = match &self.file_name {
                    Some(path) => {
                        let mut sqlite_path = path.clone();
                        sqlite_path.set_extension("sqlite");
                        tools::open_save::DefaultSaveFile::ExistingFile(sqlite_path)
                    }
                    None => tools::open_save::DefaultSaveFile::SuggestedName(
                        format!("{}.sqlite", super::DEFAULT_FILE_STEM).into(),
                    ),
                };
                sender.oneshot_command(async move {
                    match tools::open_save::save_sqlite_dialog(default).await {
                        Some(path) => ExportPanelCommandOutput::SqliteFileChosen(path),
                        None => ExportPanelCommandOutput::SqliteFileNotChosen,
                    }
                });
            }
            ExportPanelInput::UpdateColloscopeEnabled(enabled) => {
                if self.export_config.colloscope_enabled == enabled {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateColloscopeEnabled(enabled),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdateAllGroupsEnabled(enabled) => {
                if self.export_config.all_groups_enabled == enabled {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateAllGroupsEnabled(enabled),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdatePrefilledEnabled(enabled) => {
                if self.export_config.prefilled_groups_enabled == enabled {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdatePrefilledGroupsEnabled(
                            enabled,
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdateAutomaticEnabled(enabled) => {
                if self.export_config.automatic_groups_enabled == enabled {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateAutomaticGroupsEnabled(
                            enabled,
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdatePerGroupListEnabled(enabled) => {
                if self.export_config.per_group_list_enabled == enabled {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdatePerGroupListEnabled(enabled),
                    ))
                    .unwrap();
            }
            ExportPanelInput::RestoreDefaultGeneralConfigClicked => {
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateGlobalConfig(
                            collomatique_state_colloscopes::export_config::GlobalConfig::default(),
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::RestoreDefaultColloscopeConfigClicked => {
                sender.output(ExportPanelOutput::UpdateExportConfig(
                    collomatique_ops::ExportConfigUpdateOp::UpdateColloscopeConfig(collomatique_state_colloscopes::export_config::ColloscopeConfig::default())
                )).unwrap();
            }
            ExportPanelInput::RestoreDefaultAllGroupsConfigClicked => {
                sender.output(ExportPanelOutput::UpdateExportConfig(
                    collomatique_ops::ExportConfigUpdateOp::UpdateAllGroupsConfig(collomatique_state_colloscopes::export_config::PerStudentGroupsConfig::default_all_groups())
                )).unwrap();
            }
            ExportPanelInput::RestoreDefaultPrefilledConfigClicked => {
                sender.output(ExportPanelOutput::UpdateExportConfig(
                    collomatique_ops::ExportConfigUpdateOp::UpdatePrefilledGroupsConfig(collomatique_state_colloscopes::export_config::PerStudentGroupsConfig::default_prefilled_groups())
                )).unwrap();
            }
            ExportPanelInput::RestoreDefaultAutomaticConfigClicked => {
                sender.output(ExportPanelOutput::UpdateExportConfig(
                    collomatique_ops::ExportConfigUpdateOp::UpdateAutomaticGroupsConfig(collomatique_state_colloscopes::export_config::PerStudentGroupsConfig::default_automatic_groups())
                )).unwrap();
            }
            ExportPanelInput::RestoreDefaultPerGroupListConfigClicked => {
                sender.output(ExportPanelOutput::UpdateExportConfig(
                    collomatique_ops::ExportConfigUpdateOp::UpdatePerGroupListConfig(collomatique_state_colloscopes::export_config::PerGroupListConfig::default())
                )).unwrap();
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            ExportPanelCommandOutput::FileNotChosen
            | ExportPanelCommandOutput::SqliteFileNotChosen => {}
            ExportPanelCommandOutput::FileChosen(path) => {
                let xlsx_config = super::export::to_xlsx_config(&self.export_config);
                sender
                    .output(ExportPanelOutput::ExportColloscopeAs(path, xlsx_config))
                    .unwrap();
            }
            ExportPanelCommandOutput::SqliteFileChosen(path) => {
                sender
                    .output(ExportPanelOutput::ExportSqliteAs(path))
                    .unwrap();
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.export_config.colloscope_enabled {
            widgets.colloscope_box.remove_css_class("dimmed");
        } else {
            widgets.colloscope_box.add_css_class("dimmed");
        }
        if self.export_config.all_groups_enabled {
            widgets.all_groups_box.remove_css_class("dimmed");
        } else {
            widgets.all_groups_box.add_css_class("dimmed");
        }
        if self.export_config.prefilled_groups_enabled {
            widgets.prefilled_box.remove_css_class("dimmed");
        } else {
            widgets.prefilled_box.add_css_class("dimmed");
        }
        if self.export_config.automatic_groups_enabled {
            widgets.automatic_box.remove_css_class("dimmed");
        } else {
            widgets.automatic_box.add_css_class("dimmed");
        }
        if self.export_config.per_group_list_enabled {
            widgets.per_group_list_box.remove_css_class("dimmed");
        } else {
            widgets.per_group_list_box.add_css_class("dimmed");
        }
    }
}
