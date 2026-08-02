use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::prelude::FactoryVecDeque;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};
use relm4::{adw, gtk};

use collomatique_ops::GroupListsUpdateOp;

mod associations_display;
mod edit_dialog;
mod group_lists_display;

#[derive(Debug)]
pub enum GroupListsInput {
    Update(
        collomatique_state_colloscopes::periods::Periods,
        collomatique_state_colloscopes::weeks::Weeks,
        collomatique_state_colloscopes::subjects::Subjects,
        collomatique_state_colloscopes::students::Students,
        collomatique_state_colloscopes::group_lists::GroupLists,
    ),

    EditGroupList(collomatique_state_colloscopes::GroupListId),
    DeleteGroupList(collomatique_state_colloscopes::GroupListId),
    AddGroupList,
    GroupListSelected(collomatique_state_colloscopes::group_lists::GroupList),
}

#[derive(Debug)]
enum GroupListSelectionReason {
    New,
    Edit(collomatique_state_colloscopes::GroupListId),
}

pub struct GroupLists {
    periods: collomatique_state_colloscopes::periods::Periods,
    weeks: collomatique_state_colloscopes::weeks::Weeks,
    subjects: collomatique_state_colloscopes::subjects::Subjects,
    students: collomatique_state_colloscopes::students::Students,
    group_lists: collomatique_state_colloscopes::group_lists::GroupLists,

    group_list_entries: FactoryVecDeque<group_lists_display::Entry>,
    period_entries: FactoryVecDeque<associations_display::PeriodEntry>,
    edit_dialog: Controller<edit_dialog::Dialog>,

    selection_reason: GroupListSelectionReason,
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
                    gtk::Box {
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Horizontal,
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: "Listes de groupes",
                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                        },
                        gtk::Box {
                            set_hexpand: true,
                            set_orientation: gtk::Orientation::Horizontal,
                        },
                        gtk::Button {
                            add_css_class: "frame",
                            add_css_class: "accent",
                            set_sensitive: false,
                            set_margin_all: 5,
                            adw::ButtonContent {
                                set_icon_name: "system-run-symbolic",
                                set_label: "Générer des listes automatiquement",
                            },
                        },
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
                #[local_ref]
                period_box -> gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 30,
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
                group_lists_display::EntryOutput::DeleteGroupList(id) => {
                    GroupListsInput::DeleteGroupList(id)
                }
            });

        let period_entries = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.output_sender(), |msg| match msg {
                associations_display::PeriodEntryOutput::UpdateGroupListForSubjectOnPeriod(
                    period_id,
                    subject_id,
                    group_list_id,
                ) => GroupListsUpdateOp::AssignGroupListToSubject(
                    period_id,
                    subject_id,
                    group_list_id,
                ),
                associations_display::PeriodEntryOutput::CopyPreviousPeriod(period_id) => {
                    GroupListsUpdateOp::DuplicatePreviousPeriod(period_id)
                }
            });

        let edit_dialog = edit_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                edit_dialog::DialogOutput::Accepted(group_list) => {
                    GroupListsInput::GroupListSelected(group_list)
                }
            });

        let model = GroupLists {
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            weeks: collomatique_state_colloscopes::weeks::Weeks::default(),
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            students: collomatique_state_colloscopes::students::Students::default(),
            group_lists: collomatique_state_colloscopes::group_lists::GroupLists::default(),
            group_list_entries,
            period_entries,
            edit_dialog,
            selection_reason: GroupListSelectionReason::New,
        };

        let list_box = model.group_list_entries.widget();
        let period_box = model.period_entries.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            GroupListsInput::Update(periods, weeks, subjects, students, group_lists) => {
                self.periods = periods;
                self.weeks = weeks;
                self.subjects = subjects;
                self.students = students;
                self.group_lists = group_lists;

                self.update_group_list_entries();
                self.update_period_entries();
            }
            GroupListsInput::AddGroupList => {
                self.selection_reason = GroupListSelectionReason::New;

                let mut group_list_params =
                    collomatique_state_colloscopes::group_lists::GroupListParameters::default();
                let max_group_count = (self.students.student_map.len() as u32)
                    / (group_list_params.students_per_group.start().get());
                let group_count = max_group_count.max(1) as usize;
                group_list_params.group_names = vec![None; group_count];

                // A brand new list opens on the prefilled mode: the point of the
                // merged dialog is to fill the groups while setting them up.
                let group_list_filling =
                    collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled {
                        groups: vec![
                            collomatique_state_colloscopes::group_lists::PrefilledGroup::default();
                            group_count
                        ],
                    };

                let group_list = collomatique_state_colloscopes::group_lists::GroupList::new(
                    group_list_params,
                    group_list_filling,
                )
                .expect("as many empty prefilled groups as group names");

                self.show_edit_dialog(group_list);
            }
            GroupListsInput::EditGroupList(group_list_id) => {
                let group_list = self
                    .group_lists
                    .group_list_map
                    .get(&group_list_id)
                    .expect("Group list ID should be valid")
                    .clone();
                self.selection_reason = GroupListSelectionReason::Edit(group_list_id);

                self.show_edit_dialog(group_list);
            }
            GroupListsInput::DeleteGroupList(id) => {
                sender
                    .output(GroupListsUpdateOp::DeleteGroupList(id))
                    .unwrap();
            }
            GroupListsInput::GroupListSelected(group_list) => match self.selection_reason {
                GroupListSelectionReason::New => {
                    sender
                        .output(GroupListsUpdateOp::AddNewGroupList(group_list))
                        .unwrap();
                }
                GroupListSelectionReason::Edit(group_list_id) => {
                    sender
                        .output(GroupListsUpdateOp::UpdateGroupList(
                            group_list_id,
                            group_list,
                        ))
                        .unwrap();
                }
            },
        }
    }
}

impl GroupLists {
    fn show_edit_dialog(&self, group_list: collomatique_state_colloscopes::group_lists::GroupList) {
        // Pass all students - exclusion is handled inside the dialog
        let filtered_students = self
            .students
            .student_map
            .iter()
            .map(|(id, student)| (id, student.clone()))
            .collect();

        self.edit_dialog
            .sender()
            .send(edit_dialog::DialogInput::Show(
                group_list,
                filtered_students,
            ))
            .unwrap();
    }

    fn update_group_list_entries(&mut self) {
        let mut group_lists_vec: Vec<_> = self
            .group_lists
            .group_list_map
            .iter()
            .map(|(id, group_list)| group_lists_display::EntryData {
                id,
                group_list: group_list.clone(),
            })
            .collect();

        group_lists_vec.sort_by_key(|data| (data.group_list.params().name.clone(), data.id));

        crate::tools::factories::update_vec_deque(
            &mut self.group_list_entries,
            group_lists_vec.into_iter(),
            group_lists_display::EntryInput::UpdateData,
        );
    }

    fn update_period_entries(&mut self) {
        let periods_vec: Vec<_> = self
            .periods
            .period_ids()
            .map(|id| {
                let id = &id;
                let period =
                    collomatique_ops::rendering::render_period(&self.periods, &self.weeks, *id)
                        .expect("the period comes from the document being displayed");
                associations_display::PeriodEntryData {
                    period_id: *id,
                    period_text: format!("Associations pour la période {}", period),
                    subjects: self
                        .subjects
                        .ordered_subject_list
                        .iter()
                        .filter_map(|(subject_id, subject)| {
                            if subject.excluded_periods.contains(id) {
                                return None;
                            }
                            subject.parameters.interrogation_parameters.as_ref()?;

                            Some((subject_id, subject.clone()))
                        })
                        .collect(),
                    group_list_associations: self
                        .group_lists
                        .subjects_associations
                        .iter()
                        .filter_map(|((period, subject), group_list)| {
                            (period == *id).then_some((subject, *group_list))
                        })
                        .collect(),
                    group_lists: self
                        .group_lists
                        .group_list_map
                        .iter()
                        .map(|(id, gl)| (id, gl.clone()))
                        .collect(),
                }
            })
            .collect();
        crate::tools::factories::update_vec_deque(
            &mut self.period_entries,
            periods_vec.into_iter(),
            associations_display::PeriodEntryInput::UpdateData,
        );
    }
}
