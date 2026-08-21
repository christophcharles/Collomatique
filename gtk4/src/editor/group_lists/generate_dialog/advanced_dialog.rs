use adw::prelude::{ActionRowExt, ExpanderRowExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use collomatique_constraints_groups::ObjectiveWeights;
use collomatique_state_colloscopes::NonEmptyRangeInclusive;
use std::num::NonZeroU32;

/// The group size the min/max spinners start on when the reference size has
/// never been fixed by hand. Only a seed for the switch's first use: as long
/// as the expander is off, the canonical size is elected from the document
/// and these two numbers mean nothing.
const SEED_CANONICAL_MIN: u32 = 2;
const SEED_CANONICAL_MAX: u32 = 3;

pub struct Dialog {
    hidden: bool,
    move_front: bool,
    should_redraw: bool,
    /// Weight of the "share as few pairs as possible" objective term.
    w_pairs: f64,
    /// Weight of the "stay close to the reference grouping" objective term.
    w_template: f64,
    /// Whether the reference group size is imposed rather than elected.
    canonical_enabled: bool,
    /// The imposed size, kept even while `canonical_enabled` is false so that
    /// switching the expander off and on again does not lose it.
    canonical_min: u32,
    canonical_max: u32,
}

impl Dialog {
    /// The canonical-size override this window stands for: `None` while the
    /// expander is off, which is what asks for the automatic election.
    fn canonical_range(&self) -> Option<NonEmptyRangeInclusive<NonZeroU32>> {
        if !self.canonical_enabled {
            return None;
        }
        Some(
            NonEmptyRangeInclusive::new(
                NonZeroU32::new(self.canonical_min).expect("the spinner's lower bound is 1")
                    ..=NonZeroU32::new(self.canonical_max).expect("the spinner's lower bound is 1"),
            )
            .expect("the spinners clamp min <= max"),
        )
    }
}

#[derive(Debug)]
pub enum DialogInput {
    /// Open with the generate dialog's current weights and canonical size.
    Show(ObjectiveWeights, Option<NonEmptyRangeInclusive<NonZeroU32>>),
    Cancel,
    Accept,
    UpdatePairsWeight(f64),
    UpdateTemplateWeight(f64),
    SetCanonicalEnabled(bool),
    UpdateCanonicalMin(u32),
    UpdateCanonicalMax(u32),
}

#[derive(Debug)]
pub enum DialogOutput {
    Cancelled,
    /// The assembled weights and canonical-size override.
    Accepted(ObjectiveWeights, Option<NonEmptyRangeInclusive<NonZeroU32>>),
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        #[root]
        root_window = adw::Window {
            set_modal: true,
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Paramètres avancés"),
            set_default_size: (500, 450),
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
                                set_title: "Pondération de l'objectif",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Poids des paires partagées",
                                    set_subtitle: "Pénalise chaque paire d'élèves partageant un groupe : favorise la réutilisation des mêmes groupes d'une liste à l'autre",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 0.,
                                        set_upper: f64::MAX,
                                        set_step_increment: 1.,
                                        set_page_increment: 10.,
                                    },
                                    set_digits: 1,
                                    set_wrap: false,
                                    set_numeric: true,
                                    #[track(self.should_redraw)]
                                    set_value: model.w_pairs,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value();
                                        sender.input(DialogInput::UpdatePairsWeight(value));
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Poids des écarts au groupe de référence",
                                    set_subtitle: "Pénalise chaque groupe de référence éclaté entre plusieurs groupes d'une liste : favorise la réutilisation du découpage de référence",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 0.,
                                        set_upper: f64::MAX,
                                        set_step_increment: 0.05,
                                        set_page_increment: 0.25,
                                    },
                                    set_digits: 2,
                                    set_wrap: false,
                                    set_numeric: true,
                                    #[track(self.should_redraw)]
                                    set_value: model.w_template,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value();
                                        sender.input(DialogInput::UpdateTemplateWeight(value));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Groupe de référence",
                                set_description: Some("Le découpage de référence regroupe tous les élèves à une même taille, et les listes générées cherchent à lui ressembler"),
                                set_margin_all: 5,
                                set_hexpand: true,
                                #[name(canonical_expander)]
                                adw::ExpanderRow {
                                    set_hexpand: true,
                                    set_title: "Taille canonique des groupes",
                                    set_subtitle: "Sans cela, la taille est élue automatiquement : celle qui concerne le plus d'élèves",
                                    set_show_enable_switch: true,
                                    #[track(model.should_redraw)]
                                    set_enable_expansion: model.canonical_enabled,
                                    connect_enable_expansion_notify[sender] => move |widget| {
                                        let value = widget.enables_expansion();
                                        sender.input(DialogInput::SetCanonicalEnabled(value));
                                    },
                                    add_row = &adw::SpinRow {
                                        set_hexpand: true,
                                        set_title: "Minimum",
                                        #[wrap(Some)]
                                        set_adjustment = &gtk::Adjustment {
                                            set_lower: 1.,
                                            #[watch]
                                            set_upper: model.canonical_max as f64,
                                            set_step_increment: 1.,
                                            set_page_increment: 5.,
                                        },
                                        set_wrap: false,
                                        set_snap_to_ticks: true,
                                        set_numeric: true,
                                        #[track(model.should_redraw)]
                                        set_value: model.canonical_min as f64,
                                        connect_value_notify[sender] => move |widget| {
                                            let value = widget.value() as u32;
                                            sender.input(DialogInput::UpdateCanonicalMin(value));
                                        },
                                    },
                                    add_row = &adw::SpinRow {
                                        set_hexpand: true,
                                        set_title: "Maximum",
                                        #[wrap(Some)]
                                        set_adjustment = &gtk::Adjustment {
                                            #[watch]
                                            set_lower: model.canonical_min as f64,
                                            set_upper: u32::MAX as f64,
                                            set_step_increment: 1.,
                                            set_page_increment: 5.,
                                        },
                                        set_wrap: false,
                                        set_snap_to_ticks: true,
                                        set_numeric: true,
                                        #[track(model.should_redraw)]
                                        set_value: model.canonical_max as f64,
                                        connect_value_notify[sender] => move |widget| {
                                            let value = widget.value() as u32;
                                            sender.input(DialogInput::UpdateCanonicalMax(value));
                                        },
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
        let defaults = ObjectiveWeights::default();
        let model = Dialog {
            hidden: true,
            move_front: false,
            should_redraw: false,
            w_pairs: defaults.w_pairs,
            w_template: defaults.w_template,
            canonical_enabled: false,
            canonical_min: SEED_CANONICAL_MIN,
            canonical_max: SEED_CANONICAL_MAX,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        self.move_front = false;
        match msg {
            DialogInput::Show(weights, canonical_range) => {
                self.hidden = false;
                self.move_front = true;
                self.should_redraw = true;
                self.w_pairs = weights.w_pairs;
                self.w_template = weights.w_template;
                self.canonical_enabled = canonical_range.is_some();
                if let Some(range) = canonical_range {
                    self.canonical_min = range.start().get();
                    self.canonical_max = range.end().get();
                }
            }
            DialogInput::Cancel => {
                self.hidden = true;
                sender.output(DialogOutput::Cancelled).unwrap();
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(
                        ObjectiveWeights {
                            w_pairs: self.w_pairs,
                            w_template: self.w_template,
                        },
                        self.canonical_range(),
                    ))
                    .unwrap();
            }
            DialogInput::UpdatePairsWeight(value) => {
                if self.w_pairs == value {
                    return;
                }
                self.w_pairs = value;
            }
            DialogInput::UpdateTemplateWeight(value) => {
                if self.w_template == value {
                    return;
                }
                self.w_template = value;
            }
            DialogInput::SetCanonicalEnabled(value) => {
                if self.canonical_enabled == value {
                    return;
                }
                self.canonical_enabled = value;
            }
            DialogInput::UpdateCanonicalMin(value) => {
                if self.canonical_min == value {
                    return;
                }
                self.canonical_min = value;
            }
            DialogInput::UpdateCanonicalMax(value) => {
                if self.canonical_max == value {
                    return;
                }
                self.canonical_max = value;
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
    }
}
