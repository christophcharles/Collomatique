use adw::prelude::{
    ActionRowExt, ComboRowExt, EditableExt, PreferencesGroupExt, PreferencesRowExt,
};
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, WidgetExt};
use relm4::FactorySender;
use relm4::factory::FactoryView;
use relm4::gtk::prelude::OrientableExt;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};
use std::collections::{BTreeMap, BTreeSet};

use collomatique_state_colloscopes::export_config;

// === Extra Color Factory ===

#[derive(Debug, Clone)]
enum ExtraColorEntryData {
    Switch {
        annotation: String,
        enabled: bool,
    },
    Color {
        annotation: String,
        color: export_config::Color,
        visible: bool,
    },
}

#[derive(Debug)]
struct ExtraColorEntry {
    data: ExtraColorEntryData,
    index: DynamicIndex,
    should_redraw: bool,
}

impl ExtraColorEntry {
    fn is_switch(&self) -> bool {
        matches!(self.data, ExtraColorEntryData::Switch { .. })
    }

    fn title(&self) -> String {
        match &self.data {
            ExtraColorEntryData::Switch { annotation, .. } => {
                format!("Colorer l'annotation \"{annotation}\"")
            }
            ExtraColorEntryData::Color { annotation, .. } => {
                format!("Couleur pour \"{annotation}\"")
            }
        }
    }

    fn is_row_visible(&self) -> bool {
        match &self.data {
            ExtraColorEntryData::Switch { .. } => true,
            ExtraColorEntryData::Color { visible, .. } => *visible,
        }
    }

    fn is_enabled(&self) -> bool {
        match &self.data {
            ExtraColorEntryData::Switch { enabled, .. } => *enabled,
            ExtraColorEntryData::Color { .. } => false,
        }
    }

    fn get_gtk_color(&self) -> gtk::gdk::RGBA {
        match &self.data {
            ExtraColorEntryData::Color { color, .. } => Dialog::compute_gtk_color(color),
            ExtraColorEntryData::Switch { .. } => gtk::gdk::RGBA::BLACK,
        }
    }
}

#[derive(Debug, Clone)]
enum ExtraColorInput {
    UpdateData(ExtraColorEntryData),
    UpdateStatus(bool),
    UpdateColor(export_config::Color),
}

#[derive(Debug)]
enum ExtraColorOutput {
    UpdateEnabled(usize, bool),
    UpdateColor(usize, export_config::Color),
}

#[relm4::factory]
impl FactoryComponent for ExtraColorEntry {
    type Init = ExtraColorEntryData;
    type Input = ExtraColorInput;
    type Output = ExtraColorOutput;
    type CommandOutput = ();
    type ParentWidget = adw::PreferencesGroup;

    view! {
        #[root]
        adw::ActionRow {
            set_use_markup: false,
            #[watch]
            set_title: &self.title(),
            #[watch]
            set_visible: self.is_row_visible(),

            add_suffix = &gtk::Switch {
                set_valign: gtk::Align::Center,
                #[watch]
                set_visible: self.is_switch(),
                #[track(self.should_redraw)]
                set_active: self.is_enabled(),
                connect_active_notify[sender] => move |widget| {
                    sender.input(ExtraColorInput::UpdateStatus(widget.is_active()));
                },
            },

            add_suffix = &gtk::ColorDialogButton {
                set_margin_all: 5,
                #[watch]
                set_visible: !self.is_switch(),
                #[track(self.should_redraw)]
                set_rgba: &self.get_gtk_color(),
                set_dialog = &gtk::ColorDialog {
                    set_title: "Choisir la couleur",
                    set_with_alpha: false,
                },
                connect_rgba_notify[sender] => move |widget| {
                    let rgba = widget.rgba();
                    sender.input(ExtraColorInput::UpdateColor(
                        Dialog::compute_internal_color(&rgba)
                    ));
                },
            },
        }
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            data,
            index: index.clone(),
            should_redraw: false,
        }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();
        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        self.should_redraw = false;
        match msg {
            ExtraColorInput::UpdateData(new_data) => {
                self.data = new_data;
                self.should_redraw = true;
            }
            ExtraColorInput::UpdateStatus(new_status) => {
                if let ExtraColorEntryData::Switch {
                    ref mut enabled, ..
                } = self.data
                {
                    if *enabled == new_status {
                        return;
                    }
                    *enabled = new_status;
                    sender
                        .output(ExtraColorOutput::UpdateEnabled(
                            self.index.current_index(),
                            new_status,
                        ))
                        .unwrap();
                }
            }
            ExtraColorInput::UpdateColor(new_color) => {
                if let ExtraColorEntryData::Color { ref mut color, .. } = self.data {
                    if *color == new_color {
                        return;
                    }
                    *color = new_color.clone();
                    sender
                        .output(ExtraColorOutput::UpdateColor(
                            self.index.current_index(),
                            new_color,
                        ))
                        .unwrap();
                }
            }
        }
    }
}

// === Dialog ===

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    config: export_config::ColloscopeConfig,
    extra_colors_state: BTreeMap<String, (bool, export_config::Color)>,
    extra_color_entries: FactoryVecDeque<ExtraColorEntry>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(export_config::ColloscopeConfig, BTreeSet<String>),
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
    UpdateExtraColorEnabled(usize, bool),
    UpdateExtraColorColor(usize, export_config::Color),
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
            set_default_size: (500, 700),
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

                        #[local_ref]
                        extra_color_entries_widget -> adw::PreferencesGroup {
                            set_title: "Couleurs des annotations",
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[watch]
                            set_visible: !model.extra_colors_state.is_empty(),
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
        let extra_color_entries = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |msg| match msg {
                ExtraColorOutput::UpdateEnabled(index, enabled) => {
                    DialogInput::UpdateExtraColorEnabled(index, enabled)
                }
                ExtraColorOutput::UpdateColor(index, color) => {
                    DialogInput::UpdateExtraColorColor(index, color)
                }
            });

        let model = Dialog {
            hidden: true,
            should_redraw: false,
            config: export_config::ColloscopeConfig::default(),
            extra_colors_state: BTreeMap::new(),
            extra_color_entries,
        };

        let extra_color_entries_widget = model.extra_color_entries.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(config, annotations) => {
                self.config = config;
                self.hidden = false;
                self.should_redraw = true;

                // Build extra_colors_state by merging annotations and config.extra_colors
                let mut state = BTreeMap::new();
                for annotation in &annotations {
                    state.insert(
                        annotation.clone(),
                        (
                            false,
                            export_config::Color {
                                red: 255,
                                green: 0,
                                blue: 0,
                            },
                        ),
                    );
                }
                for (annotation, color) in &self.config.extra_colors {
                    state
                        .entry(annotation.clone())
                        .and_modify(|e| {
                            e.0 = true;
                            e.1 = color.clone();
                        })
                        .or_insert((true, color.clone()));
                }
                self.extra_colors_state = state;

                // Rebuild factory
                let entries: Vec<ExtraColorEntryData> = self
                    .extra_colors_state
                    .iter()
                    .flat_map(|(annotation, (enabled, color))| {
                        [
                            ExtraColorEntryData::Switch {
                                annotation: annotation.clone(),
                                enabled: *enabled,
                            },
                            ExtraColorEntryData::Color {
                                annotation: annotation.clone(),
                                color: color.clone(),
                                visible: *enabled,
                            },
                        ]
                    })
                    .collect();
                crate::tools::factories::update_vec_deque(
                    &mut self.extra_color_entries,
                    entries.into_iter(),
                    ExtraColorInput::UpdateData,
                );
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                // Rebuild extra_colors from state (only enabled entries)
                self.config.extra_colors = self
                    .extra_colors_state
                    .iter()
                    .filter(|(_, (enabled, _))| *enabled)
                    .map(|(annotation, (_, color))| (annotation.clone(), color.clone()))
                    .collect();
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
            DialogInput::UpdateExtraColorEnabled(index, enabled) => {
                let annotation_index = index / 2;
                let annotation = match self.extra_colors_state.keys().nth(annotation_index) {
                    Some(a) => a.clone(),
                    None => return,
                };
                let state = match self.extra_colors_state.get_mut(&annotation) {
                    Some(s) => s,
                    None => return,
                };
                if state.0 == enabled {
                    return;
                }
                state.0 = enabled;
                let color = state.1.clone();
                // Update the color row's visibility
                let color_index = index + 1;
                let guard = self.extra_color_entries.guard();
                guard.send(
                    color_index,
                    ExtraColorInput::UpdateData(ExtraColorEntryData::Color {
                        annotation,
                        color,
                        visible: enabled,
                    }),
                );
                guard.drop();
            }
            DialogInput::UpdateExtraColorColor(index, color) => {
                let annotation_index = index / 2;
                let annotation = match self.extra_colors_state.keys().nth(annotation_index) {
                    Some(a) => a.clone(),
                    None => return,
                };
                let state = match self.extra_colors_state.get_mut(&annotation) {
                    Some(s) => s,
                    None => return,
                };
                if state.1 == color {
                    return;
                }
                state.1 = color;
            }
        }
    }
}
