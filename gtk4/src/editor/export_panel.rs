mod colloscope_config_dialog;
mod per_group_list_config_dialog;
mod per_student_groups_config_dialog;

use adw::prelude::{ActionRowExt, PreferencesRowExt};
use gtk::prelude::{BoxExt, ButtonExt, WidgetExt};
use relm4::gtk::prelude::OrientableExt;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};
use relm4::{adw, gtk};
use std::path::PathBuf;

use collomatique_state_colloscopes::export_config;

use crate::tools;

pub struct ExportPanel {
    export_config: export_config::ExportConfig,
    file_name: Option<PathBuf>,
    colloscope_config_dialog: Controller<colloscope_config_dialog::Dialog>,
    all_groups_config_dialog: Controller<per_student_groups_config_dialog::Dialog>,
    prefilled_config_dialog: Controller<per_student_groups_config_dialog::Dialog>,
    automatic_config_dialog: Controller<per_student_groups_config_dialog::Dialog>,
    per_group_list_config_dialog: Controller<per_group_list_config_dialog::Dialog>,
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

    UpdateStripesEnabled(bool),
    UpdateStripesColor(collomatique_state_colloscopes::export_config::Color),
    UpdateBackgroundColor(collomatique_state_colloscopes::export_config::Color),

    EditColloscopeConfigClicked,
    ColloscopeConfigAccepted(export_config::ColloscopeConfig),
    EditAllGroupsConfigClicked,
    AllGroupsConfigAccepted(export_config::PerStudentGroupsConfig),
    EditPrefilledConfigClicked,
    PrefilledConfigAccepted(export_config::PerStudentGroupsConfig),
    EditAutomaticConfigClicked,
    AutomaticConfigAccepted(export_config::PerStudentGroupsConfig),
    EditPerGroupListConfigClicked,
    PerGroupListConfigAccepted(export_config::PerGroupListConfig),
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

impl ExportPanel {
    fn compute_gtk_color(
        color: &collomatique_state_colloscopes::export_config::Color,
    ) -> gtk::gdk::RGBA {
        gtk::gdk::RGBA::new(
            color.red as f32 / 255.0f32,
            color.green as f32 / 255.0f32,
            color.blue as f32 / 255.0f32,
            1.0f32,
        )
    }

    fn compute_internal_color(
        gtk_color: &gtk::gdk::RGBA,
    ) -> collomatique_state_colloscopes::export_config::Color {
        collomatique_state_colloscopes::export_config::Color {
            red: (gtk_color.red() * 255.0f32) as u8,
            green: (gtk_color.green() * 255.0f32) as u8,
            blue: (gtk_color.blue() * 255.0f32) as u8,
        }
    }
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
                    set_margin_top: 15,
                    set_orientation: gtk::Orientation::Horizontal,
                    gtk::Label {
                        set_label: "<b><i><big>Feuilles à inclure</big></i></b>",
                        set_use_markup: true,
                        set_margin_all: 5,
                        set_hexpand: true,
                    },
                },
                // Section: Colloscope
                #[name(colloscope_box)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 5,
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
                        gtk::Label {
                            set_label: "<i>affectation des groupes dans les créneaux de colles</i>",
                            set_use_markup: true,
                            set_attributes: Some(&gtk::pango::AttrList::from_string("scale 0.85").unwrap()),
                            set_valign: gtk::Align::Center,
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Button {
                            set_icon_name: "document-edit-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Configurer la feuille"),
                            #[watch]
                            set_sensitive: model.export_config.colloscope_enabled,
                            connect_clicked => ExportPanelInput::EditColloscopeConfigClicked,
                        },
                        gtk::Button {
                            set_icon_name: "edit-delete-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Restaurer les valeurs par défaut"),
                            #[watch]
                            set_sensitive: model.export_config.colloscope_enabled && model.export_config.colloscope_config != collomatique_state_colloscopes::export_config::ColloscopeConfig::default(),
                            connect_clicked => ExportPanelInput::RestoreDefaultColloscopeConfigClicked,
                        },
                    },
                },
                // Section: Tous les groupes
                #[name(all_groups_box)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 5,
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
                        gtk::Label {
                            set_label: "<i>groupes de colles (une ligne par élève)</i>",
                            set_use_markup: true,
                            set_attributes: Some(&gtk::pango::AttrList::from_string("scale 0.85").unwrap()),
                            set_valign: gtk::Align::Center,
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Button {
                            set_icon_name: "document-edit-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Configurer la feuille"),
                            #[watch]
                            set_sensitive: model.export_config.all_groups_enabled,
                            connect_clicked => ExportPanelInput::EditAllGroupsConfigClicked,
                        },
                        gtk::Button {
                            set_icon_name: "edit-delete-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Restaurer les valeurs par défaut"),
                            #[watch]
                            set_sensitive: model.export_config.all_groups_enabled && model.export_config.all_groups_config != collomatique_state_colloscopes::export_config::PerStudentGroupsConfig::default_all_groups(),
                            connect_clicked => ExportPanelInput::RestoreDefaultAllGroupsConfigClicked,
                        },
                    },
                },
                // Section: Groupes préremplis
                #[name(prefilled_box)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 5,
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
                        gtk::Label {
                            set_label: "<i>groupes de colles (une ligne par élève - groupes préremplis uniquement)</i>",
                            set_use_markup: true,
                            set_attributes: Some(&gtk::pango::AttrList::from_string("scale 0.85").unwrap()),
                            set_valign: gtk::Align::Center,
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Button {
                            set_icon_name: "document-edit-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Configurer la feuille"),
                            #[watch]
                            set_sensitive: model.export_config.prefilled_groups_enabled,
                            connect_clicked => ExportPanelInput::EditPrefilledConfigClicked,
                        },
                        gtk::Button {
                            set_icon_name: "edit-delete-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Restaurer les valeurs par défaut"),
                            #[watch]
                            set_sensitive: model.export_config.prefilled_groups_enabled && model.export_config.prefilled_groups_config != collomatique_state_colloscopes::export_config::PerStudentGroupsConfig::default_prefilled_groups(),
                            connect_clicked => ExportPanelInput::RestoreDefaultPrefilledConfigClicked,
                        },
                    },
                },
                // Section: Groupes automatiques
                #[name(automatic_box)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 5,
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
                        gtk::Label {
                            set_label: "<i>groupes de colles (une ligne par élève - groupes automatiques uniquement)</i>",
                            set_use_markup: true,
                            set_attributes: Some(&gtk::pango::AttrList::from_string("scale 0.85").unwrap()),
                            set_valign: gtk::Align::Center,
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Button {
                            set_icon_name: "document-edit-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Configurer la feuille"),
                            #[watch]
                            set_sensitive: model.export_config.automatic_groups_enabled,
                            connect_clicked => ExportPanelInput::EditAutomaticConfigClicked,
                        },
                        gtk::Button {
                            set_icon_name: "edit-delete-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Restaurer les valeurs par défaut"),
                            #[watch]
                            set_sensitive: model.export_config.automatic_groups_enabled && model.export_config.automatic_groups_config != collomatique_state_colloscopes::export_config::PerStudentGroupsConfig::default_automatic_groups(),
                            connect_clicked => ExportPanelInput::RestoreDefaultAutomaticConfigClicked,
                        },
                    },
                },
                // Section: Par liste de groupes
                #[name(per_group_list_box)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 5,
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
                        gtk::Label {
                            set_label: "<i>groupes de colles (une feuille par liste de groupes)</i>",
                            set_use_markup: true,
                            set_attributes: Some(&gtk::pango::AttrList::from_string("scale 0.85").unwrap()),
                            set_valign: gtk::Align::Center,
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Button {
                            set_icon_name: "document-edit-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Configurer la feuille"),
                            #[watch]
                            set_sensitive: model.export_config.per_group_list_enabled,
                            connect_clicked => ExportPanelInput::EditPerGroupListConfigClicked,
                        },
                        gtk::Button {
                            set_icon_name: "edit-delete-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Restaurer les valeurs par défaut"),
                            #[watch]
                            set_sensitive: model.export_config.per_group_list_enabled && model.export_config.per_group_list_config != collomatique_state_colloscopes::export_config::PerGroupListConfig::default(),
                            connect_clicked => ExportPanelInput::RestoreDefaultPerGroupListConfigClicked,
                        },
                    },
                },
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
                    set_margin_all: 5,
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 5,
                        gtk::Label {
                            set_label: "Couleurs de fond",
                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                            set_margin_all: 5,
                        },
                        gtk::Label {
                            set_label: "<i>Couleurs de fond des cellules du tableur</i>",
                            set_use_markup: true,
                            set_attributes: Some(&gtk::pango::AttrList::from_string("scale 0.85").unwrap()),
                            set_valign: gtk::Align::Center,
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
                    adw::PreferencesGroup {
                        set_hexpand: true,
                        set_margin_all: 5,
                        adw::ActionRow {
                            set_title: "Couleur de fond (par défaut)",
                            add_suffix = &gtk::ColorDialogButton {
                                set_margin_all: 5,
                                #[watch]
                                set_rgba: &Self::compute_gtk_color(&model.export_config.global.background_color),
                                set_dialog = &gtk::ColorDialog {
                                    set_title: "Choisir la couleur de fond par défaut",
                                    set_with_alpha: false,
                                },
                                connect_rgba_notify[sender] => move |widget| {
                                    let rgba = widget.rgba();
                                    sender.input(ExportPanelInput::UpdateBackgroundColor(
                                        Self::compute_internal_color(&rgba)
                                    ));
                                },
                            },
                        },
                        #[name(stripes_switch)]
                        adw::SwitchRow {
                            set_title: "Activer le zébrage",
                            #[track(model.export_config.global.stripes_color_enabled != stripes_switch.is_active())]
                            set_active: model.export_config.global.stripes_color_enabled,
                            connect_active_notify[sender] => move |widget| {
                                let status = widget.is_active();
                                sender.input(ExportPanelInput::UpdateStripesEnabled(status));
                            },
                        },
                        adw::ActionRow {
                            set_title: "Couleur de zébrage",
                            add_suffix = &gtk::ColorDialogButton {
                                set_margin_all: 5,
                                #[watch]
                                set_rgba: &Self::compute_gtk_color(&model.export_config.global.stripes_color),
                                set_dialog = &gtk::ColorDialog {
                                    set_title: "Choisir la couleur de zébrage",
                                    set_with_alpha: false,
                                },
                                connect_rgba_notify[sender] => move |widget| {
                                    let rgba = widget.rgba();
                                    sender.input(ExportPanelInput::UpdateStripesColor(
                                        Self::compute_internal_color(&rgba)
                                    ));
                                },
                            },
                        },
                    },
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
                        set_margin_all: 10,
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
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let colloscope_config_dialog = colloscope_config_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                colloscope_config_dialog::DialogOutput::Accepted(config) => {
                    ExportPanelInput::ColloscopeConfigAccepted(config)
                }
            });

        let all_groups_config_dialog = per_student_groups_config_dialog::Dialog::builder()
            .transient_for(&root)
            .launch("tous les groupes".to_string())
            .forward(sender.input_sender(), |msg| match msg {
                per_student_groups_config_dialog::DialogOutput::Accepted(config) => {
                    ExportPanelInput::AllGroupsConfigAccepted(config)
                }
            });

        let prefilled_config_dialog = per_student_groups_config_dialog::Dialog::builder()
            .transient_for(&root)
            .launch("groupes préremplis".to_string())
            .forward(sender.input_sender(), |msg| match msg {
                per_student_groups_config_dialog::DialogOutput::Accepted(config) => {
                    ExportPanelInput::PrefilledConfigAccepted(config)
                }
            });

        let automatic_config_dialog = per_student_groups_config_dialog::Dialog::builder()
            .transient_for(&root)
            .launch("groupes automatiques".to_string())
            .forward(sender.input_sender(), |msg| match msg {
                per_student_groups_config_dialog::DialogOutput::Accepted(config) => {
                    ExportPanelInput::AutomaticConfigAccepted(config)
                }
            });

        let per_group_list_config_dialog = per_group_list_config_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                per_group_list_config_dialog::DialogOutput::Accepted(config) => {
                    ExportPanelInput::PerGroupListConfigAccepted(config)
                }
            });

        let model = ExportPanel {
            export_config: export_config::ExportConfig::default(),
            file_name: None,
            colloscope_config_dialog,
            all_groups_config_dialog,
            prefilled_config_dialog,
            automatic_config_dialog,
            per_group_list_config_dialog,
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
            ExportPanelInput::UpdateStripesEnabled(stripes_enabled) => {
                if self.export_config.global.stripes_color_enabled == stripes_enabled {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateGlobalConfig(
                            collomatique_state_colloscopes::export_config::GlobalConfig {
                                stripes_color_enabled: stripes_enabled,
                                ..self.export_config.global.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdateStripesColor(stripes_color) => {
                if self.export_config.global.stripes_color == stripes_color {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateGlobalConfig(
                            collomatique_state_colloscopes::export_config::GlobalConfig {
                                stripes_color,
                                ..self.export_config.global.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdateBackgroundColor(background_color) => {
                if self.export_config.global.background_color == background_color {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateGlobalConfig(
                            collomatique_state_colloscopes::export_config::GlobalConfig {
                                background_color,
                                ..self.export_config.global.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::EditColloscopeConfigClicked => {
                self.colloscope_config_dialog
                    .sender()
                    .send(colloscope_config_dialog::DialogInput::Show(
                        self.export_config.colloscope_config.clone(),
                    ))
                    .unwrap();
            }
            ExportPanelInput::ColloscopeConfigAccepted(config) => {
                if self.export_config.colloscope_config == config {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateColloscopeConfig(config),
                    ))
                    .unwrap();
            }
            ExportPanelInput::EditAllGroupsConfigClicked => {
                self.all_groups_config_dialog
                    .sender()
                    .send(per_student_groups_config_dialog::DialogInput::Show(
                        self.export_config.all_groups_config.clone(),
                    ))
                    .unwrap();
            }
            ExportPanelInput::AllGroupsConfigAccepted(config) => {
                if self.export_config.all_groups_config == config {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateAllGroupsConfig(config),
                    ))
                    .unwrap();
            }
            ExportPanelInput::EditPrefilledConfigClicked => {
                self.prefilled_config_dialog
                    .sender()
                    .send(per_student_groups_config_dialog::DialogInput::Show(
                        self.export_config.prefilled_groups_config.clone(),
                    ))
                    .unwrap();
            }
            ExportPanelInput::PrefilledConfigAccepted(config) => {
                if self.export_config.prefilled_groups_config == config {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdatePrefilledGroupsConfig(config),
                    ))
                    .unwrap();
            }
            ExportPanelInput::EditAutomaticConfigClicked => {
                self.automatic_config_dialog
                    .sender()
                    .send(per_student_groups_config_dialog::DialogInput::Show(
                        self.export_config.automatic_groups_config.clone(),
                    ))
                    .unwrap();
            }
            ExportPanelInput::AutomaticConfigAccepted(config) => {
                if self.export_config.automatic_groups_config == config {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateAutomaticGroupsConfig(config),
                    ))
                    .unwrap();
            }
            ExportPanelInput::EditPerGroupListConfigClicked => {
                self.per_group_list_config_dialog
                    .sender()
                    .send(per_group_list_config_dialog::DialogInput::Show(
                        self.export_config.per_group_list_config.clone(),
                    ))
                    .unwrap();
            }
            ExportPanelInput::PerGroupListConfigAccepted(config) => {
                if self.export_config.per_group_list_config == config {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdatePerGroupListConfig(config),
                    ))
                    .unwrap();
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
