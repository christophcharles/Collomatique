use adw::prelude::{ActionRowExt, ComboRowExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
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
                            adw::EntryRow {
                                set_hexpand: true,
                                set_title: "Nom de la matière",
                            },
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Durée d'une colle (en minutes)",
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
                                set_value: 60.,
                                // connect_value_notify[sender] => move |widget| {},
                            },
                            adw::SwitchRow {
                                set_hexpand: true,
                                set_title: "Durée compatibilisée",
                                set_subtitle: "Pour équilibrer le nombre d'heures par semaine",
                                set_active: true,
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "Élèves par groupe",
                            set_description: Some("Nombre d'élèves minimum et maximum dans les groupes"),
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Minimum",
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
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Maximum",
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
                                set_value: 3.,
                                // connect_value_notify[sender] => move |widget| {},
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "Groupes par colle",
                            set_description: Some("Nombre de groupes à coller simultanément"),
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Minimum",
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
                                set_title: "Maximum",
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
            should_redraw: false,
            params: collomatique_state_colloscopes::SubjectParameters::default(),
            global_first_week: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(global_first_week, params) => {
                self.hidden = false;
                self.should_redraw = true;
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
        }
    }
}
