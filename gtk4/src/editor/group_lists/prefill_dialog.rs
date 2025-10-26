use adw::prelude::{AdjustmentExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

use std::collections::BTreeMap;

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    filtered_students: BTreeMap<
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::students::Student<collomatique_state_colloscopes::PeriodId>,
    >,

    ordered_students: Vec<(collomatique_state_colloscopes::StudentId, String, String)>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        collomatique_state_colloscopes::group_lists::GroupList<
            collomatique_state_colloscopes::StudentId,
        >,
        BTreeMap<
            collomatique_state_colloscopes::StudentId,
            collomatique_state_colloscopes::students::Student<
                collomatique_state_colloscopes::PeriodId,
            >,
        >,
    ),
    Cancel,
    Accept,
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(
        collomatique_state_colloscopes::group_lists::GroupListPrefilledGroups<
            collomatique_state_colloscopes::StudentId,
        >,
    ),
}

impl Dialog {}

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
            set_title: Some("Préremplissage de la liste de groupes"),
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
                        set_sensitive: false,
                    },
                },
                #[name(scrolled_window)]
                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                    gtk::Box {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_margin_all: 5,
                        set_spacing: 10,
                        set_orientation: gtk::Orientation::Vertical,
                        adw::PreferencesGroup {
                            set_title: "Paramètres du préremplissage",
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::SpinRow {
                                set_hexpand: true,
                                set_title: "Nombre de groupes",
                                #[wrap(Some)]
                                set_adjustment = &gtk::Adjustment {
                                    set_lower: 1.,
                                    set_upper: 20.,
                                    set_step_increment: 1.,
                                    set_page_increment: 5.,
                                },
                                set_wrap: false,
                                set_snap_to_ticks: true,
                                set_numeric: true,
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "Groupe 1",
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::EntryRow {
                                set_hexpand: true,
                                set_title: "Nom du groupe",
                            },
                            adw::SwitchRow {
                                set_hexpand: true,
                                set_use_markup: false,
                                set_title: "Groupe scellé",
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "",
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::ButtonRow {
                                set_hexpand: true,
                                set_title: "Ajouter un élève",
                                set_start_icon_name: Some("edit-add"),
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "Groupe 2",
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::EntryRow {
                                set_hexpand: true,
                                set_title: "Nom du groupe",
                            },
                            adw::SwitchRow {
                                set_hexpand: true,
                                set_use_markup: false,
                                set_title: "Groupe scellé",
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "",
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::ButtonRow {
                                set_hexpand: true,
                                set_title: "Ajouter un élève",
                                set_start_icon_name: Some("edit-add"),
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "Groupe 3",
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::EntryRow {
                                set_hexpand: true,
                                set_title: "Nom du groupe",
                            },
                            adw::SwitchRow {
                                set_hexpand: true,
                                set_use_markup: false,
                                set_title: "Groupe scellé",
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "",
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::ButtonRow {
                                set_hexpand: true,
                                set_title: "Ajouter un élève",
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
            filtered_students: BTreeMap::new(),
            ordered_students: vec![],
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(group_list_data, filtered_students) => {
                self.hidden = false;
                self.should_redraw = true;
                self.filtered_students = filtered_students;
                self.update_ordered_students();
                self.update_from_data(group_list_data);
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.generate_data()))
                    .unwrap();
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
    fn update_ordered_students(&mut self) {
        self.ordered_students = self
            .filtered_students
            .iter()
            .map(|(student_id, student)| {
                (
                    student_id.clone(),
                    student.desc.firstname.clone(),
                    student.desc.surname.clone(),
                )
            })
            .collect();

        self.ordered_students
            .sort_by_key(|(id, firstname, surname)| {
                (surname.clone(), firstname.clone(), id.clone())
            });
    }

    fn update_from_data(
        &mut self,
        _data: collomatique_state_colloscopes::group_lists::GroupList<
            collomatique_state_colloscopes::StudentId,
        >,
    ) {
    }

    fn generate_data(
        &self,
    ) -> collomatique_state_colloscopes::group_lists::GroupListPrefilledGroups<
        collomatique_state_colloscopes::StudentId,
    > {
        todo!()
    }
}
