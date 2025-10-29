use adw::prelude::{PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

use std::num::NonZeroU32;
pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    student_name: Option<String>,

    has_max_interrogations_per_day: bool,
    selected_max_interrogations_per_day: u32,
    soft_max_interrogations_per_day: bool,

    has_max_interrogations_per_week: bool,
    selected_max_interrogations_per_week: u32,
    soft_max_interrogations_per_week: bool,

    has_min_interrogations_per_week: bool,
    selected_min_interrogations_per_week: u32,
    soft_min_interrogations_per_week: bool,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        collomatique_state_colloscopes::settings::Limits,
        Option<String>,
    ),
    Cancel,
    Accept,

    UpdateHasMaxInterrogationsPerDay(bool),
    UpdateSelectedMaxInterrogationsPerDay(u32),
    UpdateSoftMaxInterrogationsPerDay(bool),

    UpdateHasMaxInterrogationsPerWeek(bool),
    UpdateSelectedMaxInterrogationsPerWeek(u32),
    UpdateSoftMaxInterrogationsPerWeek(bool),

    UpdateHasMinInterrogationsPerWeek(bool),
    UpdateSelectedMinInterrogationsPerWeek(u32),
    UpdateSoftMinInterrogationsPerWeek(bool),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(collomatique_state_colloscopes::settings::Limits),
}

impl Dialog {
    fn generate_params_name(&self) -> String {
        match &self.student_name {
            Some(name) => format!("Élève concerné : {}", name),
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
            set_title: Some("Paramètres supplémentaires"),
            set_default_size: (500, 500),
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
                                set_title: "Colles par semaine",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Imposer un nombre maximum de colles par semaine",
                                    #[track(self.should_redraw)]
                                    set_active: model.has_max_interrogations_per_week,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateHasMaxInterrogationsPerWeek(value));
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Nombre maximum de colles par semaine",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        #[watch]
                                        set_lower: if model.has_min_interrogations_per_week {
                                            model.selected_min_interrogations_per_week as f64
                                        } else {
                                            0.
                                        },
                                        set_upper: u32::MAX as f64,
                                        set_step_increment: 1.,
                                        set_page_increment: 5.,
                                    },
                                    set_wrap: false,
                                    set_snap_to_ticks: true,
                                    set_numeric: true,
                                    #[watch]
                                    set_visible: model.has_max_interrogations_per_week,
                                    #[track(self.should_redraw)]
                                    set_value: model.selected_max_interrogations_per_week as f64,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value() as u32;
                                        sender.input(DialogInput::UpdateSelectedMaxInterrogationsPerWeek(value));
                                    },
                                },
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Contrainte douce",
                                    #[watch]
                                    set_visible: model.has_max_interrogations_per_week,
                                    #[track(self.should_redraw)]
                                    set_active: model.soft_max_interrogations_per_week,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateSoftMaxInterrogationsPerWeek(value));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Imposer un nombre minimum de colles par semaine",
                                    #[track(self.should_redraw)]
                                    set_active: model.has_min_interrogations_per_week,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateHasMinInterrogationsPerWeek(value));
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Nombre minimum de colles par semaine",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 0.,
                                        #[watch]
                                        set_upper: if model.has_max_interrogations_per_week {
                                            model.selected_max_interrogations_per_week as f64
                                        } else {
                                            u32::MAX as f64
                                        },
                                        set_step_increment: 1.,
                                        set_page_increment: 5.,
                                    },
                                    set_wrap: false,
                                    set_snap_to_ticks: true,
                                    set_numeric: true,
                                    #[watch]
                                    set_visible: model.has_min_interrogations_per_week,
                                    #[track(self.should_redraw)]
                                    set_value: model.selected_min_interrogations_per_week as f64,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value() as u32;
                                        sender.input(DialogInput::UpdateSelectedMinInterrogationsPerWeek(value));
                                    },
                                },
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Contrainte douce",
                                    #[watch]
                                    set_visible: model.has_min_interrogations_per_week,
                                    #[track(self.should_redraw)]
                                    set_active: model.soft_min_interrogations_per_week,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateSoftMinInterrogationsPerWeek(value));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Colles par jour",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Imposer un nombre maximum de colles par jour",
                                    #[track(self.should_redraw)]
                                    set_active: model.has_max_interrogations_per_day,
                                    connect_active_notify[sender] => move |widget| {
                                        let has_max_interrogations_per_day = widget.is_active();
                                        sender.input(DialogInput::UpdateHasMaxInterrogationsPerDay(has_max_interrogations_per_day));
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Nombre maximum de colles par jour",
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
                                    #[watch]
                                    set_visible: model.has_max_interrogations_per_day,
                                    #[track(self.should_redraw)]
                                    set_value: model.selected_max_interrogations_per_day as f64,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value() as u32;
                                        sender.input(DialogInput::UpdateSelectedMaxInterrogationsPerDay(value));
                                    },
                                },
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Contrainte douce",
                                    #[watch]
                                    set_visible: model.has_max_interrogations_per_day,
                                    #[track(self.should_redraw)]
                                    set_active: model.soft_max_interrogations_per_day,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateSoftMaxInterrogationsPerDay(value));
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
            student_name: None,
            has_max_interrogations_per_day: false,
            selected_max_interrogations_per_day: 1,
            soft_max_interrogations_per_day: false,
            has_max_interrogations_per_week: false,
            selected_max_interrogations_per_week: 2,
            soft_max_interrogations_per_week: false,
            has_min_interrogations_per_week: false,
            selected_min_interrogations_per_week: 1,
            soft_min_interrogations_per_week: false,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(limits, student_name) => {
                self.hidden = false;
                self.should_redraw = true;
                self.student_name = student_name;
                self.update_state_from_limits(limits);
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.build_limits()))
                    .unwrap();
            }
            DialogInput::UpdateHasMaxInterrogationsPerDay(value) => {
                if self.has_max_interrogations_per_day == value {
                    return;
                }
                self.has_max_interrogations_per_day = value;
            }
            DialogInput::UpdateSelectedMaxInterrogationsPerDay(value) => {
                if self.selected_max_interrogations_per_day == value {
                    return;
                }
                self.selected_max_interrogations_per_day = value;
            }
            DialogInput::UpdateSoftMaxInterrogationsPerDay(value) => {
                if self.soft_max_interrogations_per_day == value {
                    return;
                }
                self.soft_max_interrogations_per_day = value;
            }
            DialogInput::UpdateHasMaxInterrogationsPerWeek(value) => {
                if self.has_max_interrogations_per_week == value {
                    return;
                }
                self.has_max_interrogations_per_week = value;
            }
            DialogInput::UpdateSelectedMaxInterrogationsPerWeek(value) => {
                if self.selected_max_interrogations_per_week == value {
                    return;
                }
                self.selected_max_interrogations_per_week = value;
                if self.selected_max_interrogations_per_week
                    < self.selected_min_interrogations_per_week
                {
                    self.selected_min_interrogations_per_week =
                        self.selected_max_interrogations_per_week;
                    self.should_redraw = true;
                }
            }
            DialogInput::UpdateSoftMaxInterrogationsPerWeek(value) => {
                if self.soft_max_interrogations_per_week == value {
                    return;
                }
                self.soft_max_interrogations_per_week = value;
            }
            DialogInput::UpdateHasMinInterrogationsPerWeek(value) => {
                if self.has_min_interrogations_per_week == value {
                    return;
                }
                self.has_min_interrogations_per_week = value;
            }
            DialogInput::UpdateSelectedMinInterrogationsPerWeek(value) => {
                if self.selected_min_interrogations_per_week == value {
                    return;
                }
                self.selected_min_interrogations_per_week = value;
                if self.selected_max_interrogations_per_week
                    < self.selected_min_interrogations_per_week
                {
                    self.selected_max_interrogations_per_week =
                        self.selected_min_interrogations_per_week;
                    self.should_redraw = true;
                }
            }
            DialogInput::UpdateSoftMinInterrogationsPerWeek(value) => {
                if self.soft_min_interrogations_per_week == value {
                    return;
                }
                self.soft_min_interrogations_per_week = value;
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
    fn update_state_from_limits(
        &mut self,
        limits: collomatique_state_colloscopes::settings::Limits,
    ) {
        if let Some(per_day) = limits.max_interrogations_per_day {
            self.has_max_interrogations_per_day = true;
            self.selected_max_interrogations_per_day = per_day.value.get();
            self.soft_max_interrogations_per_day = per_day.soft;
        } else {
            self.has_max_interrogations_per_day = false;
            self.selected_max_interrogations_per_day = 1;
            self.soft_max_interrogations_per_day = false;
        }

        if let Some(max_per_week) = limits.interrogations_per_week_max {
            self.has_max_interrogations_per_week = true;
            self.selected_max_interrogations_per_week = max_per_week.value;
            self.soft_max_interrogations_per_week = max_per_week.soft;
        } else {
            self.has_max_interrogations_per_week = false;
            self.selected_max_interrogations_per_week = 2;
            self.soft_max_interrogations_per_week = false;
        }

        if let Some(min_per_week) = limits.interrogations_per_week_min {
            self.has_min_interrogations_per_week = true;
            self.selected_min_interrogations_per_week = min_per_week.value;
            self.soft_min_interrogations_per_week = min_per_week.soft;
        } else {
            self.has_min_interrogations_per_week = false;
            self.selected_min_interrogations_per_week = 1;
            self.soft_min_interrogations_per_week = false;
        }
    }

    fn build_limits(&self) -> collomatique_state_colloscopes::settings::Limits {
        collomatique_state_colloscopes::settings::Limits {
            interrogations_per_week_min: Self::soft_value(
                self.has_min_interrogations_per_week,
                self.selected_min_interrogations_per_week,
                self.soft_min_interrogations_per_week,
            ),
            interrogations_per_week_max: Self::soft_value(
                self.has_max_interrogations_per_week,
                self.selected_max_interrogations_per_week,
                self.soft_max_interrogations_per_week,
            ),
            max_interrogations_per_day: Self::soft_value(
                self.has_max_interrogations_per_day,
                NonZeroU32::new(self.selected_max_interrogations_per_day).unwrap(),
                self.soft_max_interrogations_per_day,
            ),
        }
    }
}

impl Dialog {
    fn soft_value<T>(
        has: bool,
        value: T,
        soft: bool,
    ) -> Option<collomatique_state_colloscopes::settings::SoftParam<T>> {
        if has {
            Some(collomatique_state_colloscopes::settings::SoftParam { soft, value })
        } else {
            None
        }
    }
}
