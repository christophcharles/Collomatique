use adw::prelude::PreferencesRowExt;
use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};

use collomatique_ops::GroupListsUpdateOp;

#[derive(Debug)]
pub enum GroupListsInput {
    Update(
        collomatique_state_colloscopes::periods::Periods<collomatique_state_colloscopes::PeriodId>,
        collomatique_state_colloscopes::subjects::Subjects<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::PeriodId,
        >,
        collomatique_state_colloscopes::students::Students<
            collomatique_state_colloscopes::StudentId,
            collomatique_state_colloscopes::PeriodId,
        >,
        collomatique_state_colloscopes::group_lists::GroupLists<
            collomatique_state_colloscopes::GroupListId,
            collomatique_state_colloscopes::PeriodId,
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::StudentId,
        >,
    ),
}

pub struct GroupLists {
    periods:
        collomatique_state_colloscopes::periods::Periods<collomatique_state_colloscopes::PeriodId>,
    subjects: collomatique_state_colloscopes::subjects::Subjects<
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    >,
    students: collomatique_state_colloscopes::students::Students<
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::PeriodId,
    >,
    group_lists: collomatique_state_colloscopes::group_lists::GroupLists<
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::StudentId,
    >,
}

#[relm4::component(pub)]
impl Component for GroupLists {
    type Input = GroupListsInput;
    type Output = GroupListsUpdateOp;
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
                set_spacing: 30,
                gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 10,
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_label: "Listes de groupes",
                        set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                    },
                    gtk::ListBox {
                        set_hexpand: true,
                        add_css_class: "boxed-list",
                        set_selection_mode: gtk::SelectionMode::None,
                        append = &gtk::Box {
                            set_hexpand: true,
                            set_margin_all: 5,
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 5,
                            gtk::Button {
                                set_icon_name: "edit-symbolic",
                                add_css_class: "flat",
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Vertical,
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_xalign: 0.,
                                set_margin_start: 5,
                                set_margin_end: 5,
                                set_label: "Liste 1",
                                set_size_request: (150, -1),
                            },
                            gtk::Button {
                                set_icon_name: "view-list-bullet-symbolic",
                                add_css_class: "flat",
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Vertical,
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_xalign: 0.,
                                set_margin_start: 5,
                                set_margin_end: 5,
                                set_label: "<b>Élèves par groupe :</b> 2 à 3",
                                set_use_markup: true,
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "spacer",
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_xalign: 0.,
                                set_margin_start: 5,
                                set_margin_end: 5,
                                set_label: "<b>Nombre de groupes :</b> 7 à 8",
                                set_use_markup: true,
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                            gtk::Separator {
                                set_orientation: gtk::Orientation::Vertical,
                            },
                            gtk::Button {
                                set_icon_name: "edit-delete",
                                add_css_class: "flat",
                            },
                        },
                    },
                    gtk::Button {
                        set_margin_top: 10,
                        adw::ButtonContent {
                            set_icon_name: "edit-add",
                            set_label: "Ajouter une liste de groupes",
                        },
                    }
                },
                gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 30,
                    gtk::Box {
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 10,
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: "Associations pour la période 1",
                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                        },
                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::ComboRow {
                                set_title: "Français",
                            },
                            adw::ComboRow {
                                set_title: "Maths",
                            },
                        },
                    },
                    gtk::Box {
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 10,
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: "Associations pour la période 2",
                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                        },
                        adw::PreferencesGroup {
                            set_margin_all: 5,
                            set_hexpand: true,
                            adw::ComboRow {
                                set_title: "Français",
                            },
                            adw::ComboRow {
                                set_title: "Maths",
                            },
                        },
                    }
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = GroupLists {
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            students: collomatique_state_colloscopes::students::Students::default(),
            group_lists: collomatique_state_colloscopes::group_lists::GroupLists::default(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            GroupListsInput::Update(periods, subjects, students, group_lists) => {
                self.periods = periods;
                self.subjects = subjects;
                self.students = students;
                self.group_lists = group_lists;
            }
        }
    }
}
