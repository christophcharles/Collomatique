mod colloscope_config_dialog;

use adw::prelude::{ActionRowExt, ComboRowExt, EditableExt, EntryRowExt, PreferencesRowExt};
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
    all_groups_sheet_name_committed: String,
    should_redraw_all_groups_sheet_name: bool,
    prefilled_sheet_name_committed: String,
    should_redraw_prefilled_sheet_name: bool,
    automatic_sheet_name_committed: String,
    should_redraw_automatic_sheet_name: bool,
    colloscope_config_dialog: Controller<colloscope_config_dialog::Dialog>,
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

    UpdateAllGroupsSheetName(String),
    UpdateAllGroupsShowEmails(bool),
    UpdateAllGroupsShowTel(bool),
    UpdateAllGroupsOrientation(Option<export_config::PageOrientation>),

    UpdatePrefilledSheetName(String),
    UpdatePrefilledShowEmails(bool),
    UpdatePrefilledShowTel(bool),
    UpdatePrefilledOrientation(Option<export_config::PageOrientation>),

    UpdateAutomaticSheetName(String),
    UpdateAutomaticShowEmails(bool),
    UpdateAutomaticShowTel(bool),
    UpdateAutomaticOrientation(Option<export_config::PageOrientation>),

    UpdatePerGroupListShowEmails(bool),
    UpdatePerGroupListShowTel(bool),
    UpdatePerGroupListOrientation(export_config::PageOrientation),
    UpdatePerGroupListCenterVertically(bool),

    EditColloscopeConfigClicked,
    ColloscopeConfigAccepted(export_config::ColloscopeConfig),
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

    fn generate_all_groups_orientation_model() -> gtk::StringList {
        gtk::StringList::new(&["Automatique", "Portrait", "Paysage"])
    }

    fn orientation_to_selected(orientation: Option<&export_config::PageOrientation>) -> u32 {
        match orientation {
            None => 0,
            Some(export_config::PageOrientation::Portrait) => 1,
            Some(export_config::PageOrientation::Landscape) => 2,
        }
    }

    fn selected_to_orientation(selected: u32) -> Option<export_config::PageOrientation> {
        match selected {
            0 => None,
            1 => Some(export_config::PageOrientation::Portrait),
            2 => Some(export_config::PageOrientation::Landscape),
            _ => panic!("Invalid selection for orientation"),
        }
    }

    fn generate_per_group_list_orientation_model() -> gtk::StringList {
        gtk::StringList::new(&["Portrait", "Paysage"])
    }

    fn mandatory_orientation_to_selected(orientation: &export_config::PageOrientation) -> u32 {
        match orientation {
            export_config::PageOrientation::Portrait => 0,
            export_config::PageOrientation::Landscape => 1,
        }
    }

    fn selected_to_mandatory_orientation(selected: u32) -> export_config::PageOrientation {
        match selected {
            0 => export_config::PageOrientation::Portrait,
            1 => export_config::PageOrientation::Landscape,
            _ => panic!("Invalid selection for mandatory orientation"),
        }
    }

    fn setup_entry_focus_commit(
        entry: &adw::EntryRow,
        sender: &ComponentSender<ExportPanel>,
        make_msg: fn(String) -> ExportPanelInput,
    ) {
        let focus_controller = gtk::EventControllerFocus::new();
        let sender_clone = sender.clone();
        let entry_ref = entry.clone();
        focus_controller.connect_leave(move |_controller| {
            let text: String = entry_ref.text().into();
            sender_clone.input(make_msg(text));
        });
        entry.add_controller(focus_controller);
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
                    adw::PreferencesGroup {
                        set_margin_all: 5,
                        set_hexpand: true,
                        #[watch]
                        set_visible: model.export_config.all_groups_enabled,

                        #[name(all_groups_sheet_name_entry)]
                        adw::EntryRow {
                            set_title: "Nom de la feuille",
                            #[track(model.should_redraw_all_groups_sheet_name)]
                            set_text: &model.export_config.all_groups_config.sheet_name,
                            connect_entry_activated[sender] => move |widget| {
                                let text: String = widget.text().into();
                                sender.input(ExportPanelInput::UpdateAllGroupsSheetName(text));
                            },
                        },

                        #[name(all_groups_orientation_combo)]
                        adw::ComboRow {
                            set_title: "Orientation de la page",
                            set_model: Some(&Self::generate_all_groups_orientation_model()),
                            #[track(Self::orientation_to_selected(model.export_config.all_groups_config.orientation.as_ref()) != all_groups_orientation_combo.selected())]
                            set_selected: Self::orientation_to_selected(model.export_config.all_groups_config.orientation.as_ref()),
                            connect_selected_notify[sender] => move |widget| {
                                let selected = widget.selected();
                                sender.input(ExportPanelInput::UpdateAllGroupsOrientation(
                                    Self::selected_to_orientation(selected)
                                ));
                            },
                        },

                        #[name(all_groups_emails_switch)]
                        adw::SwitchRow {
                            set_title: "Afficher les emails",
                            #[track(model.export_config.all_groups_config.show_emails != all_groups_emails_switch.is_active())]
                            set_active: model.export_config.all_groups_config.show_emails,
                            connect_active_notify[sender] => move |widget| {
                                sender.input(ExportPanelInput::UpdateAllGroupsShowEmails(widget.is_active()));
                            },
                        },

                        #[name(all_groups_tel_switch)]
                        adw::SwitchRow {
                            set_title: "Afficher les téléphones",
                            #[track(model.export_config.all_groups_config.show_tel != all_groups_tel_switch.is_active())]
                            set_active: model.export_config.all_groups_config.show_tel,
                            connect_active_notify[sender] => move |widget| {
                                sender.input(ExportPanelInput::UpdateAllGroupsShowTel(widget.is_active()));
                            },
                        },
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
                    adw::PreferencesGroup {
                        set_margin_all: 5,
                        set_hexpand: true,
                        #[watch]
                        set_visible: model.export_config.prefilled_groups_enabled,

                        #[name(prefilled_sheet_name_entry)]
                        adw::EntryRow {
                            set_title: "Nom de la feuille",
                            #[track(model.should_redraw_prefilled_sheet_name)]
                            set_text: &model.export_config.prefilled_groups_config.sheet_name,
                            connect_entry_activated[sender] => move |widget| {
                                let text: String = widget.text().into();
                                sender.input(ExportPanelInput::UpdatePrefilledSheetName(text));
                            },
                        },

                        #[name(prefilled_orientation_combo)]
                        adw::ComboRow {
                            set_title: "Orientation de la page",
                            set_model: Some(&Self::generate_all_groups_orientation_model()),
                            #[track(Self::orientation_to_selected(model.export_config.prefilled_groups_config.orientation.as_ref()) != prefilled_orientation_combo.selected())]
                            set_selected: Self::orientation_to_selected(model.export_config.prefilled_groups_config.orientation.as_ref()),
                            connect_selected_notify[sender] => move |widget| {
                                let selected = widget.selected();
                                sender.input(ExportPanelInput::UpdatePrefilledOrientation(
                                    Self::selected_to_orientation(selected)
                                ));
                            },
                        },

                        #[name(prefilled_emails_switch)]
                        adw::SwitchRow {
                            set_title: "Afficher les emails",
                            #[track(model.export_config.prefilled_groups_config.show_emails != prefilled_emails_switch.is_active())]
                            set_active: model.export_config.prefilled_groups_config.show_emails,
                            connect_active_notify[sender] => move |widget| {
                                sender.input(ExportPanelInput::UpdatePrefilledShowEmails(widget.is_active()));
                            },
                        },

                        #[name(prefilled_tel_switch)]
                        adw::SwitchRow {
                            set_title: "Afficher les téléphones",
                            #[track(model.export_config.prefilled_groups_config.show_tel != prefilled_tel_switch.is_active())]
                            set_active: model.export_config.prefilled_groups_config.show_tel,
                            connect_active_notify[sender] => move |widget| {
                                sender.input(ExportPanelInput::UpdatePrefilledShowTel(widget.is_active()));
                            },
                        },
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
                    adw::PreferencesGroup {
                        set_margin_all: 5,
                        set_hexpand: true,
                        #[watch]
                        set_visible: model.export_config.automatic_groups_enabled,

                        #[name(automatic_sheet_name_entry)]
                        adw::EntryRow {
                            set_title: "Nom de la feuille",
                            #[track(model.should_redraw_automatic_sheet_name)]
                            set_text: &model.export_config.automatic_groups_config.sheet_name,
                            connect_entry_activated[sender] => move |widget| {
                                let text: String = widget.text().into();
                                sender.input(ExportPanelInput::UpdateAutomaticSheetName(text));
                            },
                        },

                        #[name(automatic_orientation_combo)]
                        adw::ComboRow {
                            set_title: "Orientation de la page",
                            set_model: Some(&Self::generate_all_groups_orientation_model()),
                            #[track(Self::orientation_to_selected(model.export_config.automatic_groups_config.orientation.as_ref()) != automatic_orientation_combo.selected())]
                            set_selected: Self::orientation_to_selected(model.export_config.automatic_groups_config.orientation.as_ref()),
                            connect_selected_notify[sender] => move |widget| {
                                let selected = widget.selected();
                                sender.input(ExportPanelInput::UpdateAutomaticOrientation(
                                    Self::selected_to_orientation(selected)
                                ));
                            },
                        },

                        #[name(automatic_emails_switch)]
                        adw::SwitchRow {
                            set_title: "Afficher les emails",
                            #[track(model.export_config.automatic_groups_config.show_emails != automatic_emails_switch.is_active())]
                            set_active: model.export_config.automatic_groups_config.show_emails,
                            connect_active_notify[sender] => move |widget| {
                                sender.input(ExportPanelInput::UpdateAutomaticShowEmails(widget.is_active()));
                            },
                        },

                        #[name(automatic_tel_switch)]
                        adw::SwitchRow {
                            set_title: "Afficher les téléphones",
                            #[track(model.export_config.automatic_groups_config.show_tel != automatic_tel_switch.is_active())]
                            set_active: model.export_config.automatic_groups_config.show_tel,
                            connect_active_notify[sender] => move |widget| {
                                sender.input(ExportPanelInput::UpdateAutomaticShowTel(widget.is_active()));
                            },
                        },
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
                    adw::PreferencesGroup {
                        set_margin_all: 5,
                        set_hexpand: true,
                        #[watch]
                        set_visible: model.export_config.per_group_list_enabled,

                        #[name(per_group_list_orientation_combo)]
                        adw::ComboRow {
                            set_title: "Orientation de la page",
                            set_model: Some(&Self::generate_per_group_list_orientation_model()),
                            #[track(Self::mandatory_orientation_to_selected(&model.export_config.per_group_list_config.orientation) != per_group_list_orientation_combo.selected())]
                            set_selected: Self::mandatory_orientation_to_selected(&model.export_config.per_group_list_config.orientation),
                            connect_selected_notify[sender] => move |widget| {
                                let selected = widget.selected();
                                sender.input(ExportPanelInput::UpdatePerGroupListOrientation(
                                    Self::selected_to_mandatory_orientation(selected)
                                ));
                            },
                        },

                        #[name(per_group_list_emails_switch)]
                        adw::SwitchRow {
                            set_title: "Afficher les emails",
                            #[track(model.export_config.per_group_list_config.show_emails != per_group_list_emails_switch.is_active())]
                            set_active: model.export_config.per_group_list_config.show_emails,
                            connect_active_notify[sender] => move |widget| {
                                sender.input(ExportPanelInput::UpdatePerGroupListShowEmails(widget.is_active()));
                            },
                        },

                        #[name(per_group_list_tel_switch)]
                        adw::SwitchRow {
                            set_title: "Afficher les téléphones",
                            #[track(model.export_config.per_group_list_config.show_tel != per_group_list_tel_switch.is_active())]
                            set_active: model.export_config.per_group_list_config.show_tel,
                            connect_active_notify[sender] => move |widget| {
                                sender.input(ExportPanelInput::UpdatePerGroupListShowTel(widget.is_active()));
                            },
                        },

                        #[name(per_group_list_center_switch)]
                        adw::SwitchRow {
                            set_title: "Centrer verticalement",
                            #[track(model.export_config.per_group_list_config.center_vertically != per_group_list_center_switch.is_active())]
                            set_active: model.export_config.per_group_list_config.center_vertically,
                            connect_active_notify[sender] => move |widget| {
                                sender.input(ExportPanelInput::UpdatePerGroupListCenterVertically(widget.is_active()));
                            },
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
                        set_margin_all: 5,
                        set_hexpand: true,
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

        let model = ExportPanel {
            export_config: export_config::ExportConfig::default(),
            file_name: None,
            all_groups_sheet_name_committed:
                export_config::PerStudentGroupsConfig::default_all_groups().sheet_name,
            should_redraw_all_groups_sheet_name: false,
            prefilled_sheet_name_committed:
                export_config::PerStudentGroupsConfig::default_prefilled_groups().sheet_name,
            should_redraw_prefilled_sheet_name: false,
            automatic_sheet_name_committed:
                export_config::PerStudentGroupsConfig::default_automatic_groups().sheet_name,
            should_redraw_automatic_sheet_name: false,
            colloscope_config_dialog,
        };
        let widgets = view_output!();

        // Focus-loss auto-commit for entry rows
        Self::setup_entry_focus_commit(
            &widgets.all_groups_sheet_name_entry,
            &sender,
            ExportPanelInput::UpdateAllGroupsSheetName,
        );
        Self::setup_entry_focus_commit(
            &widgets.prefilled_sheet_name_entry,
            &sender,
            ExportPanelInput::UpdatePrefilledSheetName,
        );
        Self::setup_entry_focus_commit(
            &widgets.automatic_sheet_name_entry,
            &sender,
            ExportPanelInput::UpdateAutomaticSheetName,
        );

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
        self.should_redraw_all_groups_sheet_name = false;
        self.should_redraw_prefilled_sheet_name = false;
        self.should_redraw_automatic_sheet_name = false;

        match message {
            ExportPanelInput::Update(config, file_name) => {
                if config.all_groups_config.sheet_name != self.all_groups_sheet_name_committed {
                    self.should_redraw_all_groups_sheet_name = true;
                    self.all_groups_sheet_name_committed =
                        config.all_groups_config.sheet_name.clone();
                }
                if config.prefilled_groups_config.sheet_name != self.prefilled_sheet_name_committed
                {
                    self.should_redraw_prefilled_sheet_name = true;
                    self.prefilled_sheet_name_committed =
                        config.prefilled_groups_config.sheet_name.clone();
                }
                if config.automatic_groups_config.sheet_name != self.automatic_sheet_name_committed
                {
                    self.should_redraw_automatic_sheet_name = true;
                    self.automatic_sheet_name_committed =
                        config.automatic_groups_config.sheet_name.clone();
                }
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
            ExportPanelInput::UpdateAllGroupsSheetName(new_name) => {
                if self.export_config.all_groups_config.sheet_name == new_name {
                    return;
                }
                self.all_groups_sheet_name_committed = new_name.clone();
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateAllGroupsConfig(
                            export_config::PerStudentGroupsConfig {
                                sheet_name: new_name,
                                ..self.export_config.all_groups_config.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdateAllGroupsShowEmails(show_emails) => {
                if self.export_config.all_groups_config.show_emails == show_emails {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateAllGroupsConfig(
                            export_config::PerStudentGroupsConfig {
                                show_emails,
                                ..self.export_config.all_groups_config.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdateAllGroupsShowTel(show_tel) => {
                if self.export_config.all_groups_config.show_tel == show_tel {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateAllGroupsConfig(
                            export_config::PerStudentGroupsConfig {
                                show_tel,
                                ..self.export_config.all_groups_config.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdateAllGroupsOrientation(orientation) => {
                if self.export_config.all_groups_config.orientation == orientation {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateAllGroupsConfig(
                            export_config::PerStudentGroupsConfig {
                                orientation,
                                ..self.export_config.all_groups_config.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdatePrefilledSheetName(new_name) => {
                if self.export_config.prefilled_groups_config.sheet_name == new_name {
                    return;
                }
                self.prefilled_sheet_name_committed = new_name.clone();
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdatePrefilledGroupsConfig(
                            export_config::PerStudentGroupsConfig {
                                sheet_name: new_name,
                                ..self.export_config.prefilled_groups_config.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdatePrefilledShowEmails(show_emails) => {
                if self.export_config.prefilled_groups_config.show_emails == show_emails {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdatePrefilledGroupsConfig(
                            export_config::PerStudentGroupsConfig {
                                show_emails,
                                ..self.export_config.prefilled_groups_config.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdatePrefilledShowTel(show_tel) => {
                if self.export_config.prefilled_groups_config.show_tel == show_tel {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdatePrefilledGroupsConfig(
                            export_config::PerStudentGroupsConfig {
                                show_tel,
                                ..self.export_config.prefilled_groups_config.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdatePrefilledOrientation(orientation) => {
                if self.export_config.prefilled_groups_config.orientation == orientation {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdatePrefilledGroupsConfig(
                            export_config::PerStudentGroupsConfig {
                                orientation,
                                ..self.export_config.prefilled_groups_config.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdateAutomaticSheetName(new_name) => {
                if self.export_config.automatic_groups_config.sheet_name == new_name {
                    return;
                }
                self.automatic_sheet_name_committed = new_name.clone();
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateAutomaticGroupsConfig(
                            export_config::PerStudentGroupsConfig {
                                sheet_name: new_name,
                                ..self.export_config.automatic_groups_config.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdateAutomaticShowEmails(show_emails) => {
                if self.export_config.automatic_groups_config.show_emails == show_emails {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateAutomaticGroupsConfig(
                            export_config::PerStudentGroupsConfig {
                                show_emails,
                                ..self.export_config.automatic_groups_config.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdateAutomaticShowTel(show_tel) => {
                if self.export_config.automatic_groups_config.show_tel == show_tel {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateAutomaticGroupsConfig(
                            export_config::PerStudentGroupsConfig {
                                show_tel,
                                ..self.export_config.automatic_groups_config.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdateAutomaticOrientation(orientation) => {
                if self.export_config.automatic_groups_config.orientation == orientation {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdateAutomaticGroupsConfig(
                            export_config::PerStudentGroupsConfig {
                                orientation,
                                ..self.export_config.automatic_groups_config.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdatePerGroupListShowEmails(show_emails) => {
                if self.export_config.per_group_list_config.show_emails == show_emails {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdatePerGroupListConfig(
                            export_config::PerGroupListConfig {
                                show_emails,
                                ..self.export_config.per_group_list_config.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdatePerGroupListShowTel(show_tel) => {
                if self.export_config.per_group_list_config.show_tel == show_tel {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdatePerGroupListConfig(
                            export_config::PerGroupListConfig {
                                show_tel,
                                ..self.export_config.per_group_list_config.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdatePerGroupListOrientation(orientation) => {
                if self.export_config.per_group_list_config.orientation == orientation {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdatePerGroupListConfig(
                            export_config::PerGroupListConfig {
                                orientation,
                                ..self.export_config.per_group_list_config.clone()
                            },
                        ),
                    ))
                    .unwrap();
            }
            ExportPanelInput::UpdatePerGroupListCenterVertically(center_vertically) => {
                if self.export_config.per_group_list_config.center_vertically == center_vertically {
                    return;
                }
                sender
                    .output(ExportPanelOutput::UpdateExportConfig(
                        collomatique_ops::ExportConfigUpdateOp::UpdatePerGroupListConfig(
                            export_config::PerGroupListConfig {
                                center_vertically,
                                ..self.export_config.per_group_list_config.clone()
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
