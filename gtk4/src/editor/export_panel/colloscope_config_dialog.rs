use adw::prelude::{
    ActionRowExt, ComboRowExt, EditableExt, PreferencesGroupExt, PreferencesRowExt,
};
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, WidgetExt};
use relm4::gtk::prelude::OrientableExt;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use collomatique_state_colloscopes::export_config;

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    config: export_config::ColloscopeConfig,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(export_config::ColloscopeConfig),
    Cancel,
    Accept,

    UpdateSheetName(String),
    UpdateOrientation(export_config::PageOrientation),
    UpdateExtraInfoColumnEnabled(bool),
    UpdateExtraInfoColumnName(String),
    UpdateTeacherEmailEnabled(bool),
    UpdateTeacherEmail(String),
    UpdateTeacherTelEnabled(bool),
    UpdateTeacherTel(String),
    UpdateDisplayWeekDates(bool),
    UpdateDisplayAnnotations(bool),
    UpdateNoInterrogationColor(export_config::Color),
    UpdateAnnotationColorEnabled(bool),
    UpdateAnnotationColor(export_config::Color),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(export_config::ColloscopeConfig),
}

impl Dialog {
    fn compute_gtk_color(color: &export_config::Color) -> gtk::gdk::RGBA {
        gtk::gdk::RGBA::new(
            color.red as f32 / 255.0f32,
            color.green as f32 / 255.0f32,
            color.blue as f32 / 255.0f32,
            1.0f32,
        )
    }

    fn compute_internal_color(gtk_color: &gtk::gdk::RGBA) -> export_config::Color {
        export_config::Color {
            red: (gtk_color.red() * 255.0f32) as u8,
            green: (gtk_color.green() * 255.0f32) as u8,
            blue: (gtk_color.blue() * 255.0f32) as u8,
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
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        #[root]
        adw::Window {
            set_modal: true,
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Configuration : colloscope"),
            set_default_size: (500, 600),
            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        set_label: "Annuler",
                        connect_clicked => DialogInput::Cancel,
                    },
                    pack_end = &gtk::Button {
                        set_label: "Valider",
                        add_css_class: "suggested-action",
                        connect_clicked => DialogInput::Accept,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_hexpand: true,
                    set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                    gtk::Box {
                        set_hexpand: true,
                        set_margin_all: 5,
                        set_spacing: 10,
                        set_orientation: gtk::Orientation::Vertical,
                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,
                            set_title: "Paramètres de la feuille",

                            #[name(sheet_name_entry)]
                            adw::EntryRow {
                                set_title: "Nom de la feuille",
                                #[track(model.should_redraw)]
                                set_text: &model.config.sheet_name,
                                connect_text_notify[sender] => move |widget| {
                                    let text: String = widget.text().into();
                                    sender.input(DialogInput::UpdateSheetName(text));
                                },
                            },

                            #[name(orientation_combo)]
                            adw::ComboRow {
                                set_title: "Orientation de la page",
                                set_model: Some(&Self::generate_per_group_list_orientation_model()),
                                #[track(Self::mandatory_orientation_to_selected(&model.config.orientation) != orientation_combo.selected())]
                                set_selected: Self::mandatory_orientation_to_selected(&model.config.orientation),
                                connect_selected_notify[sender] => move |widget| {
                                    let selected = widget.selected();
                                    sender.input(DialogInput::UpdateOrientation(
                                        Self::selected_to_mandatory_orientation(selected)
                                    ));
                                },
                            },
                        },

                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,
                            set_title: "Paramètres des colonnes",

                            #[name(extra_info_column_enabled_switch)]
                            adw::SwitchRow {
                                set_title: "Afficher la colonne d'info supplémentaire",
                                #[track(model.config.extra_info_column_enabled != extra_info_column_enabled_switch.is_active())]
                                set_active: model.config.extra_info_column_enabled,
                                connect_active_notify[sender] => move |widget| {
                                    sender.input(DialogInput::UpdateExtraInfoColumnEnabled(widget.is_active()));
                                },
                            },

                            #[name(extra_info_column_name_entry)]
                            adw::EntryRow {
                                set_title: "Nom de la colonne d'info supplémentaire",
                                #[track(model.should_redraw)]
                                set_text: &model.config.extra_info_column_name,
                                connect_text_notify[sender] => move |widget| {
                                    let text: String = widget.text().into();
                                    sender.input(DialogInput::UpdateExtraInfoColumnName(text));
                                },
                            },

                            #[name(teacher_email_enabled_switch)]
                            adw::SwitchRow {
                                set_title: "Afficher l'email du colleur",
                                #[track(model.config.teacher_email_enabled != teacher_email_enabled_switch.is_active())]
                                set_active: model.config.teacher_email_enabled,
                                connect_active_notify[sender] => move |widget| {
                                    sender.input(DialogInput::UpdateTeacherEmailEnabled(widget.is_active()));
                                },
                            },

                            #[name(teacher_email_entry)]
                            adw::EntryRow {
                                set_title: "Nom de la colonne email",
                                #[track(model.should_redraw)]
                                set_text: &model.config.teacher_email,
                                connect_text_notify[sender] => move |widget| {
                                    let text: String = widget.text().into();
                                    sender.input(DialogInput::UpdateTeacherEmail(text));
                                },
                            },

                            #[name(teacher_tel_enabled_switch)]
                            adw::SwitchRow {
                                set_title: "Afficher le téléphone du colleur",
                                #[track(model.config.teacher_tel_enabled != teacher_tel_enabled_switch.is_active())]
                                set_active: model.config.teacher_tel_enabled,
                                connect_active_notify[sender] => move |widget| {
                                    sender.input(DialogInput::UpdateTeacherTelEnabled(widget.is_active()));
                                },
                            },

                            #[name(teacher_tel_entry)]
                            adw::EntryRow {
                                set_title: "Nom de la colonne téléphone",
                                #[track(model.should_redraw)]
                                set_text: &model.config.teacher_tel,
                                connect_text_notify[sender] => move |widget| {
                                    let text: String = widget.text().into();
                                    sender.input(DialogInput::UpdateTeacherTel(text));
                                },
                            },
                        },

                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,
                            set_title: "Affichages supplémentaires",

                            #[name(display_week_dates_switch)]
                            adw::SwitchRow {
                                set_title: "Afficher les dates des semaines",
                                #[track(model.config.display_week_dates != display_week_dates_switch.is_active())]
                                set_active: model.config.display_week_dates,
                                connect_active_notify[sender] => move |widget| {
                                    sender.input(DialogInput::UpdateDisplayWeekDates(widget.is_active()));
                                },
                            },

                            #[name(display_annotations_switch)]
                            adw::SwitchRow {
                                set_title: "Afficher les annotations",
                                #[track(model.config.display_annotations != display_annotations_switch.is_active())]
                                set_active: model.config.display_annotations,
                                connect_active_notify[sender] => move |widget| {
                                    sender.input(DialogInput::UpdateDisplayAnnotations(widget.is_active()));
                                },
                            },
                        },

                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,
                            set_title: "Couleurs",

                            adw::ActionRow {
                                set_title: "Couleur sans interrogation",
                                add_suffix = &gtk::ColorDialogButton {
                                    set_margin_all: 5,
                                    #[watch]
                                    set_rgba: &Self::compute_gtk_color(&model.config.no_interrogation_color),
                                    set_dialog = &gtk::ColorDialog {
                                        set_title: "Choisir la couleur sans interrogation",
                                        set_with_alpha: false,
                                    },
                                    connect_rgba_notify[sender] => move |widget| {
                                        let rgba = widget.rgba();
                                        sender.input(DialogInput::UpdateNoInterrogationColor(
                                            Self::compute_internal_color(&rgba)
                                        ));
                                    },
                                },
                            },

                            #[name(annotation_color_enabled_switch)]
                            adw::SwitchRow {
                                set_title: "Activer la couleur d'annotation",
                                #[track(model.config.annotation_color_enabled != annotation_color_enabled_switch.is_active())]
                                set_active: model.config.annotation_color_enabled,
                                connect_active_notify[sender] => move |widget| {
                                    sender.input(DialogInput::UpdateAnnotationColorEnabled(widget.is_active()));
                                },
                            },

                            adw::ActionRow {
                                set_title: "Couleur d'annotation",
                                add_suffix = &gtk::ColorDialogButton {
                                    set_margin_all: 5,
                                    #[watch]
                                    set_rgba: &Self::compute_gtk_color(&model.config.annotation_color),
                                    set_dialog = &gtk::ColorDialog {
                                        set_title: "Choisir la couleur d'annotation",
                                        set_with_alpha: false,
                                    },
                                    connect_rgba_notify[sender] => move |widget| {
                                        let rgba = widget.rgba();
                                        sender.input(DialogInput::UpdateAnnotationColor(
                                            Self::compute_internal_color(&rgba)
                                        ));
                                    },
                                },
                            },
                        },
                    },
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Dialog {
            hidden: true,
            should_redraw: false,
            config: export_config::ColloscopeConfig::default(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(config) => {
                self.config = config;
                self.hidden = false;
                self.should_redraw = true;
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.config.clone()))
                    .unwrap();
            }
            DialogInput::UpdateSheetName(new_name) => {
                if self.config.sheet_name == new_name {
                    return;
                }
                self.config.sheet_name = new_name;
            }
            DialogInput::UpdateOrientation(orientation) => {
                if self.config.orientation == orientation {
                    return;
                }
                self.config.orientation = orientation;
            }
            DialogInput::UpdateExtraInfoColumnEnabled(enabled) => {
                if self.config.extra_info_column_enabled == enabled {
                    return;
                }
                self.config.extra_info_column_enabled = enabled;
            }
            DialogInput::UpdateExtraInfoColumnName(new_name) => {
                if self.config.extra_info_column_name == new_name {
                    return;
                }
                self.config.extra_info_column_name = new_name;
            }
            DialogInput::UpdateTeacherEmailEnabled(enabled) => {
                if self.config.teacher_email_enabled == enabled {
                    return;
                }
                self.config.teacher_email_enabled = enabled;
            }
            DialogInput::UpdateTeacherEmail(new_name) => {
                if self.config.teacher_email == new_name {
                    return;
                }
                self.config.teacher_email = new_name;
            }
            DialogInput::UpdateTeacherTelEnabled(enabled) => {
                if self.config.teacher_tel_enabled == enabled {
                    return;
                }
                self.config.teacher_tel_enabled = enabled;
            }
            DialogInput::UpdateTeacherTel(new_name) => {
                if self.config.teacher_tel == new_name {
                    return;
                }
                self.config.teacher_tel = new_name;
            }
            DialogInput::UpdateDisplayWeekDates(display) => {
                if self.config.display_week_dates == display {
                    return;
                }
                self.config.display_week_dates = display;
            }
            DialogInput::UpdateDisplayAnnotations(display) => {
                if self.config.display_annotations == display {
                    return;
                }
                self.config.display_annotations = display;
            }
            DialogInput::UpdateNoInterrogationColor(color) => {
                if self.config.no_interrogation_color == color {
                    return;
                }
                self.config.no_interrogation_color = color;
            }
            DialogInput::UpdateAnnotationColorEnabled(enabled) => {
                if self.config.annotation_color_enabled == enabled {
                    return;
                }
                self.config.annotation_color_enabled = enabled;
            }
            DialogInput::UpdateAnnotationColor(color) => {
                if self.config.annotation_color == color {
                    return;
                }
                self.config.annotation_color = color;
            }
        }
    }
}
