use adw::prelude::{PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use collomatique_constraints_colloscopes::SolveConfig;

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    /// Whether the cross-fixed-period constraints are softened (objectified) rather than kept hard.
    objectify_enabled: bool,
    /// Penalty weight applied when softening; kept across toggles so re-enabling restores it.
    objectify_weight: f64,
    /// L1 "keep the current value" anchor penalty weight.
    l1_anchor_weight: f64,
}

#[derive(Debug)]
pub enum DialogInput {
    /// Open with the current `objectify_cross_fixed_period` and `l1_anchor_weight`.
    Show(Option<f64>, f64),
    Cancel,
    Accept,
    UpdateObjectifyEnabled(bool),
    UpdateObjectifyWeight(f64),
    UpdateL1AnchorWeight(f64),
}

#[derive(Debug)]
pub enum DialogOutput {
    Cancelled,
    /// The assembled `objectify_cross_fixed_period` and `l1_anchor_weight`.
    Accepted(Option<f64>, f64),
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
            set_title: Some("Paramètres avancés"),
            set_default_size: (500, 350),
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
                                set_title: "Gestion des contraintes inter-périodes",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Assouplir les contraintes qui traversent les périodes figées",
                                    #[track(self.should_redraw)]
                                    set_active: model.objectify_enabled,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateObjectifyEnabled(value));
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Poids",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 0.,
                                        set_upper: f64::MAX,
                                        set_step_increment: 100.,
                                        set_page_increment: 500.,
                                    },
                                    set_digits: 1,
                                    set_wrap: false,
                                    set_numeric: true,
                                    #[watch]
                                    set_visible: model.objectify_enabled,
                                    #[track(self.should_redraw)]
                                    set_value: model.objectify_weight,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value();
                                        sender.input(DialogInput::UpdateObjectifyWeight(value));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Prise en compte du colloscope actuel",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Poids L1",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 0.,
                                        set_upper: f64::MAX,
                                        set_step_increment: 100.,
                                        set_page_increment: 500.,
                                    },
                                    set_digits: 1,
                                    set_wrap: false,
                                    set_numeric: true,
                                    #[track(self.should_redraw)]
                                    set_value: model.l1_anchor_weight,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value();
                                        sender.input(DialogInput::UpdateL1AnchorWeight(value));
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
        let defaults = SolveConfig::default();
        let model = Dialog {
            hidden: true,
            should_redraw: false,
            objectify_enabled: defaults.objectify_cross_fixed_period.is_some(),
            objectify_weight: defaults.objectify_cross_fixed_period.unwrap_or(0.),
            l1_anchor_weight: defaults.l1_anchor_weight,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(objectify, l1_anchor_weight) => {
                self.hidden = false;
                self.should_redraw = true;
                match objectify {
                    Some(weight) => {
                        self.objectify_enabled = true;
                        self.objectify_weight = weight;
                    }
                    // Keep the last weight so re-enabling the switch restores it.
                    None => {
                        self.objectify_enabled = false;
                    }
                }
                self.l1_anchor_weight = l1_anchor_weight;
            }
            DialogInput::Cancel => {
                self.hidden = true;
                sender.output(DialogOutput::Cancelled).unwrap();
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(
                        self.objectify_enabled.then_some(self.objectify_weight),
                        self.l1_anchor_weight,
                    ))
                    .unwrap();
            }
            DialogInput::UpdateObjectifyEnabled(value) => {
                if self.objectify_enabled == value {
                    return;
                }
                self.objectify_enabled = value;
            }
            DialogInput::UpdateObjectifyWeight(value) => {
                if self.objectify_weight == value {
                    return;
                }
                self.objectify_weight = value;
            }
            DialogInput::UpdateL1AnchorWeight(value) => {
                if self.l1_anchor_weight == value {
                    return;
                }
                self.l1_anchor_weight = value;
            }
        }
    }
}
