use adw::prelude::{PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use collomatique_state_colloscopes::balancing::BalancingOptions;
use collomatique_state_colloscopes::soft_param::SoftParam;

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    subject_name: Option<String>,

    has_teacher_rotation: bool,
    soft_teacher_rotation: bool,

    has_slot_rotation: bool,
    soft_slot_rotation: bool,

    has_avoid_twice_in_a_row: bool,

    has_year_teacher_rotation: bool,

    has_period_teacher_rotation: bool,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(BalancingOptions, Option<String>),
    Cancel,
    Accept,

    UpdateHasTeacherRotation(bool),
    UpdateSoftTeacherRotation(bool),

    UpdateHasSlotRotation(bool),
    UpdateSoftSlotRotation(bool),

    UpdateHasAvoidTwiceInARow(bool),

    UpdateHasYearTeacherRotation(bool),

    UpdateHasPeriodTeacherRotation(bool),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(BalancingOptions),
}

impl Dialog {
    fn generate_params_name(&self) -> String {
        match &self.subject_name {
            Some(name) => format!("Matière concernée : {}", name),
            None => "Paramètres globaux".into(),
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
            set_title: Some("Paramètres d'équilibrage"),
            set_default_size: (500, 300),
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
                set_content = &gtk::Box {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_margin_all: 5,
                    set_spacing: 10,
                    set_orientation: gtk::Orientation::Vertical,
                    #[name(scrolled_window)]
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                        gtk::Box {
                            set_hexpand: true,
                            set_margin_all: 5,
                            set_spacing: 10,
                            set_orientation: gtk::Orientation::Vertical,
                            adw::PreferencesGroup {
                                set_title: "Rotation des colleurs",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Activer la rotation des colleurs",
                                    #[track(self.should_redraw)]
                                    set_active: model.has_teacher_rotation,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateHasTeacherRotation(value));
                                    },
                                },
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Contrainte douce",
                                    #[watch]
                                    set_visible: model.has_teacher_rotation,
                                    #[track(self.should_redraw)]
                                    set_active: model.soft_teacher_rotation,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateSoftTeacherRotation(value));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Rotation des créneaux",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Activer la rotation des créneaux",
                                    #[track(self.should_redraw)]
                                    set_active: model.has_slot_rotation,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateHasSlotRotation(value));
                                    },
                                },
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Contrainte douce",
                                    #[watch]
                                    set_visible: model.has_slot_rotation,
                                    #[track(self.should_redraw)]
                                    set_active: model.soft_slot_rotation,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateSoftSlotRotation(value));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Éviter deux fois de suite le même colleur",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Activer",
                                    #[track(self.should_redraw)]
                                    set_active: model.has_avoid_twice_in_a_row,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateHasAvoidTwiceInARow(value));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Rotation annuelle des colleurs",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Activer la rotation annuelle des colleurs",
                                    #[track(self.should_redraw)]
                                    set_active: model.has_year_teacher_rotation,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateHasYearTeacherRotation(value));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Rotation des colleurs par période",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Activer la rotation des colleurs par période",
                                    #[track(self.should_redraw)]
                                    set_active: model.has_period_teacher_rotation,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateHasPeriodTeacherRotation(value));
                                    },
                                },
                            },
                        },
                    },
                    gtk::Label {
                        set_margin_all: 5,
                        #[watch]
                        set_label: &model.generate_params_name(),
                        set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
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
            subject_name: None,
            has_teacher_rotation: false,
            soft_teacher_rotation: false,
            has_slot_rotation: false,
            soft_slot_rotation: false,
            has_avoid_twice_in_a_row: false,
            has_year_teacher_rotation: false,
            has_period_teacher_rotation: false,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(options, subject_name) => {
                self.hidden = false;
                self.should_redraw = true;
                self.subject_name = subject_name;
                self.update_state_from_options(options);
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.build_options()))
                    .unwrap();
            }
            DialogInput::UpdateHasTeacherRotation(value) => {
                if self.has_teacher_rotation == value {
                    return;
                }
                self.has_teacher_rotation = value;
            }
            DialogInput::UpdateSoftTeacherRotation(value) => {
                if self.soft_teacher_rotation == value {
                    return;
                }
                self.soft_teacher_rotation = value;
            }
            DialogInput::UpdateHasSlotRotation(value) => {
                if self.has_slot_rotation == value {
                    return;
                }
                self.has_slot_rotation = value;
            }
            DialogInput::UpdateSoftSlotRotation(value) => {
                if self.soft_slot_rotation == value {
                    return;
                }
                self.soft_slot_rotation = value;
            }
            DialogInput::UpdateHasAvoidTwiceInARow(value) => {
                if self.has_avoid_twice_in_a_row == value {
                    return;
                }
                self.has_avoid_twice_in_a_row = value;
            }
            DialogInput::UpdateHasYearTeacherRotation(value) => {
                if self.has_year_teacher_rotation == value {
                    return;
                }
                self.has_year_teacher_rotation = value;
            }
            DialogInput::UpdateHasPeriodTeacherRotation(value) => {
                if self.has_period_teacher_rotation == value {
                    return;
                }
                self.has_period_teacher_rotation = value;
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.should_redraw {
            let adj = widgets.scrolled_window.vadjustment();
            adj.set_value(0.);
        }
    }
}

impl Dialog {
    fn update_state_from_options(&mut self, options: BalancingOptions) {
        if let Some(tr) = options.teacher_rotation {
            self.has_teacher_rotation = true;
            self.soft_teacher_rotation = tr.soft;
        } else {
            self.has_teacher_rotation = false;
            self.soft_teacher_rotation = false;
        }

        if let Some(sr) = options.slot_rotation {
            self.has_slot_rotation = true;
            self.soft_slot_rotation = sr.soft;
        } else {
            self.has_slot_rotation = false;
            self.soft_slot_rotation = false;
        }

        self.has_avoid_twice_in_a_row = options.avoid_twice_in_a_row;
        self.has_year_teacher_rotation = options.year_teacher_rotation;
        self.has_period_teacher_rotation = options.period_teacher_rotation;
    }

    fn build_options(&self) -> BalancingOptions {
        BalancingOptions {
            teacher_rotation: Self::soft_unit_value(
                self.has_teacher_rotation,
                self.soft_teacher_rotation,
            ),
            slot_rotation: Self::soft_unit_value(self.has_slot_rotation, self.soft_slot_rotation),
            avoid_twice_in_a_row: self.has_avoid_twice_in_a_row,
            year_teacher_rotation: self.has_year_teacher_rotation,
            period_teacher_rotation: self.has_period_teacher_rotation,
        }
    }

    fn soft_unit_value(has: bool, soft: bool) -> Option<SoftParam<()>> {
        if has {
            Some(SoftParam { soft, value: () })
        } else {
            None
        }
    }
}
