use adw::prelude::PreferencesRowExt;
use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::prelude::FactoryVecDeque;
use relm4::{adw, gtk};
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};

use collomatique_ops::GroupListsUpdateOp;

mod associations_display;
mod group_lists_display;

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

    EditGroupList(collomatique_state_colloscopes::GroupListId),
    PrefillGroupList(collomatique_state_colloscopes::GroupListId),
    DeleteGroupList(collomatique_state_colloscopes::GroupListId),
    AddGroupList,
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

    group_list_entries: FactoryVecDeque<group_lists_display::Entry>,
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
                    #[local_ref]
                    list_box -> gtk::ListBox {
                        set_hexpand: true,
                        add_css_class: "boxed-list",
                        set_selection_mode: gtk::SelectionMode::None,
                        #[watch]
                        set_visible: !model.group_lists.group_list_map.is_empty(),
                    },
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_label: "<i>Aucune liste à afficher</i>",
                        set_use_markup: true,
                        #[watch]
                        set_visible: model.group_lists.group_list_map.is_empty(),
                    },
                    gtk::Button {
                        set_margin_top: 10,
                        adw::ButtonContent {
                            set_icon_name: "edit-add",
                            set_label: "Ajouter une liste de groupes",
                        },
                        connect_clicked => GroupListsInput::AddGroupList,
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
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let group_list_entries = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |msg| match msg {
                group_lists_display::EntryOutput::EditGroupList(id) => {
                    GroupListsInput::EditGroupList(id)
                }
                group_lists_display::EntryOutput::PrefillGroupList(id) => {
                    GroupListsInput::PrefillGroupList(id)
                }
                group_lists_display::EntryOutput::DeleteGroupList(id) => {
                    GroupListsInput::DeleteGroupList(id)
                }
            });

        let model = GroupLists {
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            students: collomatique_state_colloscopes::students::Students::default(),
            group_lists: collomatique_state_colloscopes::group_lists::GroupLists::default(),
            group_list_entries,
        };

        let list_box = model.group_list_entries.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            GroupListsInput::Update(periods, subjects, students, group_lists) => {
                self.periods = periods;
                self.subjects = subjects;
                self.students = students;
                self.group_lists = group_lists;

                self.update_group_list_entries();
            }
            GroupListsInput::AddGroupList => {}
            GroupListsInput::EditGroupList(_id) => {}
            GroupListsInput::PrefillGroupList(_id) => {}
            GroupListsInput::DeleteGroupList(id) => {
                sender
                    .output(GroupListsUpdateOp::DeleteGroupList(id))
                    .unwrap();
            }
        }
    }
}

impl GroupLists {
    fn update_group_list_entries(&mut self) {
        let mut group_lists_vec: Vec<_> = self
            .group_lists
            .group_list_map
            .iter()
            .map(|(id, group_list)| group_lists_display::EntryData {
                id: id.clone(),
                group_list: group_list.clone(),
            })
            .collect();

        group_lists_vec.sort_by_key(|data| (data.group_list.params.name.clone(), data.id.clone()));

        crate::tools::factories::update_vec_deque(
            &mut self.group_list_entries,
            group_lists_vec.into_iter(),
            |data| group_lists_display::EntryInput::UpdateData(data),
        );
    }
}
