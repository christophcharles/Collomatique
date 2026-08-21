use adw::prelude::{ActionRowExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use collomatique_state_colloscopes::balancing::BalancingOptions;
use collomatique_state_colloscopes::soft_param::SoftParam;

pub struct Dialog {
    hidden: bool,
    move_front: bool,
    should_redraw: bool,
    subject_name: Option<String>,

    has_teacher_rotation: bool,
    strict_teacher_rotation: bool,

    has_slot_rotation: bool,
    strict_slot_rotation: bool,

    has_avoid_twice_in_a_row: bool,
    strict_avoid_twice_in_a_row: bool,

    has_year_teacher_rotation: bool,

    has_period_teacher_rotation: bool,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(BalancingOptions, Option<String>),
    Cancel,
    Accept,

    UpdateHasTeacherRotation(bool),
    UpdateStrictTeacherRotation(bool),

    UpdateHasSlotRotation(bool),
    UpdateStrictSlotRotation(bool),

    UpdateHasAvoidTwiceInARow(bool),
    UpdateStrictAvoidTwiceInARow(bool),

    UpdateHasYearTeacherRotation(bool),

    UpdateHasPeriodTeacherRotation(bool),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(BalancingOptions),
    /// The dialog just closed: whoever owns the window underneath should bring
    /// it back to the front, because Windows will not do it on its own.
    PresentParent,
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
        root_window = adw::Window {
            set_modal: true,
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Paramètres d'équilibrage"),
            set_default_size: (500, 650),
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
                                set_description: Some("Un élève ne doit pas toujours tomber sur le même colleur. Ses colles sont réparties entre les colleurs de la matière, proportionnellement au nombre de créneaux de chacun. Si l'équilibrage est activé sans contrainte stricte, le logiciel s'en approche au mieux."),
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Activer",
                                    set_subtitle: "Si désactivé, cet équilibrage n'est ni imposé ni même recherché. Le calcul du colloscope est alors plus rapide.",
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
                                    set_title: "Contrainte stricte",
                                    set_subtitle: "Si activé, la répartition devient obligatoire sur toute suite de semaines. Le colloscope peut alors devenir impossible à générer.",
                                    #[watch]
                                    set_visible: model.has_teacher_rotation,
                                    #[track(self.should_redraw)]
                                    set_active: model.strict_teacher_rotation,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateStrictTeacherRotation(value));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Rotation des créneaux",
                                set_description: Some("Un élève ne doit pas toujours être collé au même horaire. Ses colles sont réparties entre les créneaux de la matière, proportionnellement à la fréquence de chacun. Si l'équilibrage est activé sans contrainte stricte, le logiciel s'en approche au mieux."),
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Activer",
                                    set_subtitle: "Si désactivé, cet équilibrage n'est ni imposé ni même recherché. Le calcul du colloscope est alors plus rapide.",
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
                                    set_title: "Contrainte stricte",
                                    set_subtitle: "Si activé, la répartition devient obligatoire sur toute suite de semaines. Le colloscope peut alors devenir impossible à générer.",
                                    #[watch]
                                    set_visible: model.has_slot_rotation,
                                    #[track(self.should_redraw)]
                                    set_active: model.strict_slot_rotation,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateStrictSlotRotation(value));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Éviter deux fois de suite le même colleur",
                                set_description: Some("Deux colles qui se suivent ne devraient pas être assurées par le même colleur. Si la règle est activée sans contrainte stricte, une répétition reste possible mais elle est évitée autant que possible."),
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Activer",
                                    set_subtitle: "Si désactivé, cette règle n'est ni imposée ni même recherchée. Le calcul du colloscope est alors plus rapide.",
                                    #[track(self.should_redraw)]
                                    set_active: model.has_avoid_twice_in_a_row,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateHasAvoidTwiceInARow(value));
                                    },
                                },
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Contrainte stricte",
                                    set_subtitle: "Si activé, la répétition est interdite. Attention : c'est impossible si la matière n'a qu'un seul colleur.",
                                    #[watch]
                                    set_visible: model.has_avoid_twice_in_a_row,
                                    #[track(self.should_redraw)]
                                    set_active: model.strict_avoid_twice_in_a_row,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateStrictAvoidTwiceInARow(value));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Rotation annuelle des colleurs",
                                set_description: Some("Sur l'année entière, un élève ne peut pas dépasser la part de colles qui revient à chaque colleur. Seul le total de l'année est contrôlé, pas la régularité."),
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Activer",
                                    set_subtitle: "Contrainte stricte supplémentaire. Utile surtout si la rotation des colleurs n'est pas stricte.",
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
                                set_description: Some("Même règle, mais appliquée à l'intérieur de chaque période. Un élève ne peut donc pas voir surtout un colleur au premier trimestre et surtout un autre au second."),
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Activer",
                                    set_subtitle: "Contrainte stricte supplémentaire, plus exigeante que la rotation annuelle.",
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
            move_front: false,
            should_redraw: false,
            subject_name: None,
            has_teacher_rotation: false,
            strict_teacher_rotation: false,
            has_slot_rotation: false,
            strict_slot_rotation: false,
            has_avoid_twice_in_a_row: false,
            strict_avoid_twice_in_a_row: false,
            has_year_teacher_rotation: false,
            has_period_teacher_rotation: false,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        self.move_front = false;
        match msg {
            DialogInput::Show(options, subject_name) => {
                self.hidden = false;
                self.move_front = true;
                self.should_redraw = true;
                self.subject_name = subject_name;
                self.update_state_from_options(options);
            }
            DialogInput::Cancel => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                }
            }
            DialogInput::Accept => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                    sender
                        .output(DialogOutput::Accepted(self.build_options()))
                        .unwrap();
                }
            }
            DialogInput::UpdateHasTeacherRotation(value) => {
                if self.has_teacher_rotation == value {
                    return;
                }
                self.has_teacher_rotation = value;
            }
            DialogInput::UpdateStrictTeacherRotation(value) => {
                if self.strict_teacher_rotation == value {
                    return;
                }
                self.strict_teacher_rotation = value;
            }
            DialogInput::UpdateHasSlotRotation(value) => {
                if self.has_slot_rotation == value {
                    return;
                }
                self.has_slot_rotation = value;
            }
            DialogInput::UpdateStrictSlotRotation(value) => {
                if self.strict_slot_rotation == value {
                    return;
                }
                self.strict_slot_rotation = value;
            }
            DialogInput::UpdateHasAvoidTwiceInARow(value) => {
                if self.has_avoid_twice_in_a_row == value {
                    return;
                }
                self.has_avoid_twice_in_a_row = value;
            }
            DialogInput::UpdateStrictAvoidTwiceInARow(value) => {
                if self.strict_avoid_twice_in_a_row == value {
                    return;
                }
                self.strict_avoid_twice_in_a_row = value;
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
        if self.move_front {
            widgets.root_window.present();
        }
        if self.should_redraw {
            let adj = widgets.scrolled_window.vadjustment();
            adj.set_value(0.);
        }
    }
}

impl Dialog {
    fn update_state_from_options(&mut self, options: BalancingOptions) {
        (self.has_teacher_rotation, self.strict_teacher_rotation) =
            Self::split_goal(options.teacher_rotation);
        (self.has_slot_rotation, self.strict_slot_rotation) =
            Self::split_goal(options.slot_rotation);
        (
            self.has_avoid_twice_in_a_row,
            self.strict_avoid_twice_in_a_row,
        ) = Self::split_goal(options.avoid_twice_in_a_row);

        self.has_year_teacher_rotation = options.year_teacher_rotation;
        self.has_period_teacher_rotation = options.period_teacher_rotation;
    }

    fn build_options(&self) -> BalancingOptions {
        BalancingOptions {
            teacher_rotation: Self::join_goal(
                self.has_teacher_rotation,
                self.strict_teacher_rotation,
            ),
            slot_rotation: Self::join_goal(self.has_slot_rotation, self.strict_slot_rotation),
            avoid_twice_in_a_row: Self::join_goal(
                self.has_avoid_twice_in_a_row,
                self.strict_avoid_twice_in_a_row,
            ),
            year_teacher_rotation: self.has_year_teacher_rotation,
            period_teacher_rotation: self.has_period_teacher_rotation,
        }
    }

    /// A three-state goal as the two switches show it: whether it is active,
    /// and whether it is strict. An inactive goal has no strictness, so the
    /// strict switch goes back to off.
    fn split_goal(goal: Option<SoftParam<()>>) -> (bool, bool) {
        match goal {
            Some(param) => (true, !param.soft),
            None => (false, false),
        }
    }

    /// The reverse of [`Self::split_goal`]: the strict switch is meaningless
    /// while the goal is off, and encodes `None`.
    fn join_goal(has: bool, strict: bool) -> Option<SoftParam<()>> {
        has.then_some(SoftParam {
            soft: !strict,
            value: (),
        })
    }
}
