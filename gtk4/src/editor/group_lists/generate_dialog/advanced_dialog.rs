use adw::prelude::{ActionRowExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use collomatique_constraints_groups::ObjectiveWeights;

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    /// Weight of the "use as few groups as possible" objective term.
    w_groups: f64,
    /// Weight of the "share as few pairs as possible" objective term.
    w_pairs: f64,
}

#[derive(Debug)]
pub enum DialogInput {
    /// Open with the generate dialog's current weights.
    Show(ObjectiveWeights),
    Cancel,
    Accept,
    UpdateGroupsWeight(f64),
    UpdatePairsWeight(f64),
}

#[derive(Debug)]
pub enum DialogOutput {
    Cancelled,
    /// The assembled weights.
    Accepted(ObjectiveWeights),
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
            set_default_size: (500, 250),
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
                                    set_title: "Poids du nombre de groupes",
                                    set_subtitle: "Pénalise chaque groupe non vide : un poids élevé minimise d'abord le nombre de groupes",
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
                                    set_value: model.w_groups,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value();
                                        sender.input(DialogInput::UpdateGroupsWeight(value));
                                    },
                                },
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
            should_redraw: false,
            w_groups: defaults.w_groups,
            w_pairs: defaults.w_pairs,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(weights) => {
                self.hidden = false;
                self.should_redraw = true;
                self.w_groups = weights.w_groups;
                self.w_pairs = weights.w_pairs;
            }
            DialogInput::Cancel => {
                self.hidden = true;
                sender.output(DialogOutput::Cancelled).unwrap();
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(ObjectiveWeights {
                        w_groups: self.w_groups,
                        w_pairs: self.w_pairs,
                    }))
                    .unwrap();
            }
            DialogInput::UpdateGroupsWeight(value) => {
                if self.w_groups == value {
                    return;
                }
                self.w_groups = value;
            }
            DialogInput::UpdatePairsWeight(value) => {
                if self.w_pairs == value {
                    return;
                }
                self.w_pairs = value;
            }
        }
    }
}
