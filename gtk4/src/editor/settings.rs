use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::gtk;
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};

use collomatique_ops::SettingsUpdateOp;

#[derive(Debug)]
pub enum SettingsInput {
    Update(
        collomatique_state_colloscopes::students::Students<
            collomatique_state_colloscopes::StudentId,
            collomatique_state_colloscopes::PeriodId,
        >,
        collomatique_state_colloscopes::settings::Settings<
            collomatique_state_colloscopes::StudentId,
        >,
    ),
}

pub struct Settings {
    students: collomatique_state_colloscopes::students::Students<
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::PeriodId,
    >,
    settings: collomatique_state_colloscopes::settings::Settings<
        collomatique_state_colloscopes::StudentId,
    >,
}

#[relm4::component(pub)]
impl Component for Settings {
    type Input = SettingsInput;
    type Output = SettingsUpdateOp;
    type Init = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hexpand: true,
            set_margin_all: 5,
            set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Automatic),
            gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 10,
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    set_label: "Paramètres globaux",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                },
                gtk::ListBox {
                    set_hexpand: true,
                    add_css_class: "boxed-list",
                    set_selection_mode: gtk::SelectionMode::None,
                    append = &gtk::Box {
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_margin_all: 5,
                        set_spacing: 5,
                        gtk::Button {
                            set_icon_name: "edit-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Modifier les paramètres globaux"),
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.,
                            set_margin_start: 5,
                            set_margin_end: 5,
                            set_label: "Paramètres globaux",
                            set_size_request: (200, -1),
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.,
                            set_margin_start: 5,
                            set_margin_end: 5,
                            #[watch]
                            set_label: &limits_to_string(&model.settings.global),
                            set_attributes: Some(&gtk::pango::AttrList::from_string("style italic, scale 0.8").unwrap()),
                        },
                    },
                },
                gtk::Label {
                    set_margin_top: 30,
                    set_halign: gtk::Align::Start,
                    set_label: "Paramètres par élève",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                },
                gtk::Label {
                    set_margin_top: 10,
                    #[watch]
                    set_visible: model.students.student_map.is_empty(),
                    set_halign: gtk::Align::Start,
                    set_label: "<i>Aucun élève à afficher</i>",
                    set_use_markup: true,
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Settings {
            students: collomatique_state_colloscopes::students::Students::default(),
            settings: collomatique_state_colloscopes::settings::Settings::default(),
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            SettingsInput::Update(students, settings) => {
                self.students = students;
                self.settings = settings;
            }
        }
    }
}

fn soft_less_than(soft: bool) -> String {
    if soft {
        String::from("⪅")
    } else {
        String::from("⩽")
    }
}

fn limits_to_string(limits: &collomatique_state_colloscopes::settings::Limits) -> String {
    let mut parts = vec![];

    if limits.interrogations_per_week_max.is_some() || limits.interrogations_per_week_min.is_some()
    {
        let mut text = String::new();

        if let Some(min_per_week) = &limits.interrogations_per_week_min {
            text += &format!(
                "{} {} ",
                min_per_week.value,
                soft_less_than(min_per_week.soft),
            );
        }

        text += "colles par semaine";

        if let Some(max_per_week) = &limits.interrogations_per_week_max {
            text += &format!(
                " {} {}",
                soft_less_than(max_per_week.soft),
                max_per_week.value,
            );
        }

        parts.push(text);
    }

    if let Some(max_per_day) = &limits.max_interrogations_per_day {
        parts.push(format!(
            "colles par jour {} {}",
            soft_less_than(max_per_day.soft),
            max_per_day.value,
        ));
    }

    parts.join("    ―    ")
}
