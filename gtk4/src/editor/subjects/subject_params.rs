use adw::prelude::{
    ActionRowExt, ComboRowExt, EditableExt, PreferencesGroupExt, PreferencesRowExt,
};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

use std::num::NonZeroU32;

pub struct Dialog {
    hidden: bool,
    params: collomatique_state_colloscopes::SubjectParameters,
    global_first_week: Option<collomatique_time::NaiveMondayDate>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        Option<collomatique_time::NaiveMondayDate>,
        collomatique_state_colloscopes::SubjectParameters,
    ),
    Cancel,
    Accept,

    UpdateName(String),
    UpdateDuration(collomatique_time::NonZeroDurationInMinutes),
    UpdateDurationTakenIntoAccount(bool),
    UpdateStudentsPerGroupMinimum(NonZeroU32),
    UpdateStudentsPerGroupMaximum(NonZeroU32),
    UpdateGroupsPerInterrogationMinimum(NonZeroU32),
    UpdateGroupsPerInterrogationMaximum(NonZeroU32),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(collomatique_state_colloscopes::SubjectParameters),
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
            set_title: Some("Configuration de la matière"),
            set_size_request: (500, 600),
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
                            set_title: "Paramètres généraux",
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[name(name_entry)]
                            adw::EntryRow {
                                set_hexpand: true,
                                set_title: "Nom de la matière",
                                #[track(model.params.name.as_str() != name_entry.text().as_str())]
                                set_text: &model.params.name,
                                connect_text_notify[sender] => move |widget| {
                                    let text : String = widget.text().into();
                                    sender.input(DialogInput::UpdateName(text));
                                },
                            },
                            #[name(duration_entry)]
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Durée d'une colle (en minutes)",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    set_lower: 1.,
                                    set_upper: u32::MAX as f64,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                #[track(model.params.duration.get().get() as f64 != duration_entry.value())]
                                set_value: model.params.duration.get().get() as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let duration_u32 = widget.value() as u32;
                                    let duration = collomatique_time::NonZeroDurationInMinutes::new(duration_u32).unwrap();
                                    sender.input(DialogInput::UpdateDuration(duration));
                                },
                            },
                            #[name(duration_taken_into_account_entry)]
                            adw::SwitchRow {
                                set_hexpand: true,
                                set_title: "Durée compatibilisée",
                                set_subtitle: "Pour équilibrer le nombre d'heures par semaine",
                                #[track(model.params.take_duration_into_account != duration_taken_into_account_entry.is_active())]
                                set_active: model.params.take_duration_into_account,
                                connect_active_notify[sender] => move |widget| {
                                    let duration_taken_into_account = widget.is_active();
                                    sender.input(DialogInput::UpdateDurationTakenIntoAccount(duration_taken_into_account));
                                },
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "Élèves par groupe",
                            set_description: Some("Nombre d'élèves minimum et maximum dans les groupes"),
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[name(students_per_group_min_entry)]
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Minimum",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    set_lower: 1.,
                                    #[watch]
                                    set_upper: model.params.students_per_group.end().get() as f64,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                #[track(model.params.students_per_group.start().get() as f64 != students_per_group_min_entry.value())]
                                set_value: model.params.students_per_group.start().get() as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let students_per_group_min_u32 = widget.value() as u32;
                                    let students_per_group_min = NonZeroU32::new(students_per_group_min_u32).unwrap();
                                    sender.input(DialogInput::UpdateStudentsPerGroupMinimum(students_per_group_min));
                                },
                            },
                            #[name(students_per_group_max_entry)]
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Maximum",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    #[watch]
                                    set_lower: model.params.students_per_group.start().get() as f64,
                                    set_upper: u32::MAX as f64,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                #[track(model.params.students_per_group.end().get() as f64 != students_per_group_max_entry.value())]
                                set_value: model.params.students_per_group.end().get() as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let students_per_group_max_u32 = widget.value() as u32;
                                    let students_per_group_max = NonZeroU32::new(students_per_group_max_u32).unwrap();
                                    sender.input(DialogInput::UpdateStudentsPerGroupMaximum(students_per_group_max));
                                },
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "Groupes par colle",
                            set_description: Some("Nombre de groupes à coller simultanément"),
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[name(groups_per_interrogation_min_entry)]
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Minimum",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    set_lower: 1.,
                                    #[watch]
                                    set_upper: model.params.groups_per_interrogation.end().get() as f64,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                #[track(model.params.groups_per_interrogation.start().get() as f64 != groups_per_interrogation_min_entry.value())]
                                set_value: model.params.groups_per_interrogation.start().get() as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let groups_per_interrogation_min_u32 = widget.value() as u32;
                                    let groups_per_interrogation_min = NonZeroU32::new(groups_per_interrogation_min_u32).unwrap();
                                    sender.input(DialogInput::UpdateGroupsPerInterrogationMinimum(groups_per_interrogation_min));
                                },
                            },
                            #[name(groups_per_interrogation_max_entry)]
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Maximum",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    #[watch]
                                    set_lower: model.params.groups_per_interrogation.start().get() as f64,
                                    set_upper: u32::MAX as f64,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                #[track(model.params.groups_per_interrogation.end().get() as f64 != groups_per_interrogation_max_entry.value())]
                                set_value: model.params.groups_per_interrogation.end().get() as f64,
                                connect_value_notify[sender] => move |widget| {
                                    let groups_per_interrogation_max_u32 = widget.value() as u32;
                                    let groups_per_interrogation_max = NonZeroU32::new(groups_per_interrogation_max_u32).unwrap();
                                    sender.input(DialogInput::UpdateGroupsPerInterrogationMaximum(groups_per_interrogation_max));
                                },
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "Périodicité",
                            set_description: Some("Périodicité des colles de la matière"),
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::ComboRow {
                                set_title: "Type de périodicité",
                                set_model: Some(&gtk::StringList::new(&["Programme glissant", "Par blocs de semaines", "Colles à l'année", "Par blocs (arbitraires)"])),
                            },
                        },
                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Périodicité (en semaines)",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    set_lower: 1.,
                                    set_upper: 255.,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                set_value: 2.,
                                // connect_value_notify[sender] => move |widget| {},
                            },
                        },
                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Taille des blocs (en semaines)",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    set_lower: 1.,
                                    set_upper: 255.,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                set_value: 2.,
                                // connect_value_notify[sender] => move |widget| {},
                            },
                        },
                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Colles dans l'année",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    set_lower: 0.,
                                    set_upper: 255.,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                set_value: 2.,
                                // connect_value_notify[sender] => move |widget| {},
                            },
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Séparation minimale (en semaines)",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    set_lower: 0.,
                                    set_upper: 255.,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                                set_value: 0.,
                                // connect_value_notify[sender] => move |widget| {},
                            },
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            adw::PreferencesGroup {
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::ActionRow {
                                    set_hexpand: true,
                                    set_title: "Bloc 1 du 01/09/2025 au 14/09/2025 (semaines 1 à 2)",
                                    add_suffix = &gtk::Button {
                                        add_css_class: "flat",
                                        set_icon_name: "edit-delete",
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Semaines vides qui précèdent",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 0.,
                                        set_upper: 255.,
                                        set_step_increment: 1.,
                                        set_page_increment: 5.,
                                    },
                                    set_wrap: false,
                                    set_snap_to_ticks: true,
                                    set_numeric: true,
                                    set_value: 0.,
                                    // connect_value_notify[sender] => move |widget| {},
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Durée du bloc (en semaines)",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 1.,
                                        set_upper: 255.,
                                        set_step_increment: 1.,
                                        set_page_increment: 5.,
                                    },
                                    set_wrap: false,
                                    set_snap_to_ticks: true,
                                    set_numeric: true,
                                    set_value: 2.,
                                    // connect_value_notify[sender] => move |widget| {},
                                },
                            },
                            adw::PreferencesGroup {
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::ActionRow {
                                    set_hexpand: true,
                                    set_title: "Bloc 2 du 21/09/2025 au 27/09/2025 (semaine 4)",
                                    add_suffix = &gtk::Button {
                                        add_css_class: "flat",
                                        set_icon_name: "edit-delete",
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Semaines vides qui précèdent",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 0.,
                                        set_upper: 255.,
                                        set_step_increment: 1.,
                                        set_page_increment: 5.,
                                    },
                                    set_wrap: false,
                                    set_snap_to_ticks: true,
                                    set_numeric: true,
                                    set_value: 1.,
                                    // connect_value_notify[sender] => move |widget| {},
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Durée du bloc (en semaines)",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 1.,
                                        set_upper: 255.,
                                        set_step_increment: 1.,
                                        set_page_increment: 5.,
                                    },
                                    set_wrap: false,
                                    set_snap_to_ticks: true,
                                    set_numeric: true,
                                    set_value: 1.,
                                    // connect_value_notify[sender] => move |widget| {},
                                },
                            },
                        },
                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::ButtonRow {
                                set_hexpand: true,
                                set_title: "Ajouter un bloc",
                                set_start_icon_name: Some("edit-add"),
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
            params: collomatique_state_colloscopes::SubjectParameters::default(),
            global_first_week: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Show(global_first_week, params) => {
                self.hidden = false;
                self.params = params;
                self.global_first_week = global_first_week;
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.params.clone()))
                    .unwrap();
            }
            DialogInput::UpdateName(new_name) => {
                if self.params.name == new_name {
                    return;
                }
                self.params.name = new_name;
            }
            DialogInput::UpdateDuration(new_duration) => {
                if self.params.duration == new_duration {
                    return;
                }
                self.params.duration = new_duration;
            }
            DialogInput::UpdateDurationTakenIntoAccount(duration_taken_into_account) => {
                if self.params.take_duration_into_account == duration_taken_into_account {
                    return;
                }
                self.params.take_duration_into_account = duration_taken_into_account;
            }
            DialogInput::UpdateStudentsPerGroupMinimum(new_min) => {
                if *self.params.students_per_group.start() == new_min {
                    return;
                }
                let old_max = self.params.students_per_group.end().clone();
                assert!(new_min <= old_max);
                self.params.students_per_group = new_min..=old_max;
            }
            DialogInput::UpdateStudentsPerGroupMaximum(new_max) => {
                if *self.params.students_per_group.end() == new_max {
                    return;
                }
                let old_min = self.params.students_per_group.start().clone();
                assert!(old_min <= new_max);
                self.params.students_per_group = old_min..=new_max;
            }
            DialogInput::UpdateGroupsPerInterrogationMinimum(new_min) => {
                if *self.params.groups_per_interrogation.start() == new_min {
                    return;
                }
                let old_max = self.params.groups_per_interrogation.end().clone();
                assert!(new_min <= old_max);
                self.params.groups_per_interrogation = new_min..=old_max;
            }
            DialogInput::UpdateGroupsPerInterrogationMaximum(new_max) => {
                if *self.params.groups_per_interrogation.end() == new_max {
                    return;
                }
                let old_min = self.params.groups_per_interrogation.start().clone();
                assert!(old_min <= new_max);
                self.params.groups_per_interrogation = old_min..=new_max;
            }
        }
    }
}
