use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::prelude::FactoryVecDeque;
use relm4::{adw, gtk};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};

use collomatique_ops::GroupListsUpdateOp;

mod associations_display;
mod group_lists_display;
mod params_dialog;
mod prefill_dialog;

#[derive(Debug)]
pub enum GroupListsInput {
    Update(
        collomatique_state_colloscopes::periods::Periods,
        collomatique_state_colloscopes::subjects::Subjects,
        collomatique_state_colloscopes::students::Students,
        collomatique_state_colloscopes::group_lists::GroupLists,
    ),

    EditGroupList(collomatique_state_colloscopes::GroupListId),
    PrefillGroupList(collomatique_state_colloscopes::GroupListId),
    DeleteGroupList(collomatique_state_colloscopes::GroupListId),
    AddGroupList,
    GroupListParamsSelected(collomatique_state_colloscopes::group_lists::GroupListParameters),
    GroupListPrefillSelected(
        Option<collomatique_state_colloscopes::group_lists::GroupListPrefilledGroups>,
    ),
}

#[derive(Debug)]
enum GroupListParamsSelectionReason {
    New,
    Edit(collomatique_state_colloscopes::GroupListId),
}

pub struct GroupLists {
    periods: collomatique_state_colloscopes::periods::Periods,
    subjects: collomatique_state_colloscopes::subjects::Subjects,
    students: collomatique_state_colloscopes::students::Students,
    group_lists: collomatique_state_colloscopes::group_lists::GroupLists,

    group_list_entries: FactoryVecDeque<group_lists_display::Entry>,
    period_entries: FactoryVecDeque<associations_display::PeriodEntry>,
    params_dialog: Controller<params_dialog::Dialog>,
    prefill_dialog: Controller<prefill_dialog::Dialog>,

    params_selection_reason: GroupListParamsSelectionReason,
    prefill_group_list_id: Option<collomatique_state_colloscopes::GroupListId>,
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
                group_lists_display::EntryOutput::PrefillGroupList(id) => {
                    GroupListsInput::PrefillGroupList(id)
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

        let params_dialog = params_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                params_dialog::DialogOutput::Accepted(params) => {
                    GroupListsInput::GroupListParamsSelected(params)
                }
            });

        let prefill_dialog = prefill_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                prefill_dialog::DialogOutput::Accepted(prefill) => {
                    GroupListsInput::GroupListPrefillSelected(prefill)
                }
            });

        let model = GroupLists {
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            students: collomatique_state_colloscopes::students::Students::default(),
            group_lists: collomatique_state_colloscopes::group_lists::GroupLists::default(),
            group_list_entries,
            period_entries,
            params_dialog,
            params_selection_reason: GroupListParamsSelectionReason::New,
            prefill_dialog,
            prefill_group_list_id: None,
        };

        let list_box = model.group_list_entries.widget();
        let period_box = model.period_entries.widget();
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
                self.update_period_entries();
            }
            GroupListsInput::AddGroupList => {
                self.params_selection_reason = GroupListParamsSelectionReason::New;

                let mut group_list_params =
                    collomatique_state_colloscopes::group_lists::GroupListParameters::default();
                let max_group_count = (self.students.student_map.len() as u32)
                    / (group_list_params.students_per_group.start().get());
                group_list_params.group_names = vec![None; max_group_count.max(1) as usize];

                self.params_dialog
                    .sender()
                    .send(params_dialog::DialogInput::Show(
                        group_list_params,
                        self.students.clone(),
                    ))
                    .unwrap();
            }
            GroupListsInput::EditGroupList(group_list_id) => {
                let group_list_params = self
                    .group_lists
                    .group_list_map
                    .get(&group_list_id)
                    .expect("Group list ID should be valid")
                    .params
                    .clone();
                self.params_selection_reason = GroupListParamsSelectionReason::Edit(group_list_id);
                self.params_dialog
                    .sender()
                    .send(params_dialog::DialogInput::Show(
                        group_list_params,
                        self.students.clone(),
                    ))
                    .unwrap();
            }
            GroupListsInput::PrefillGroupList(group_list_id) => {
                let group_list = self
                    .group_lists
                    .group_list_map
                    .get(&group_list_id)
                    .expect("Group list ID should be valid")
                    .clone();
                let filtered_students = self
                    .students
                    .student_map
                    .iter()
                    .filter_map(|(student_id, student)| {
                        if group_list.params.excluded_students.contains(student_id) {
                            return None;
                        }
                        Some((student_id.clone(), student.clone()))
                    })
                    .collect();
                self.prefill_group_list_id = Some(group_list_id);
                self.prefill_dialog
                    .sender()
                    .send(prefill_dialog::DialogInput::Show(
                        group_list,
                        filtered_students,
                    ))
                    .unwrap();
            }
            GroupListsInput::DeleteGroupList(id) => {
                sender
                    .output(GroupListsUpdateOp::DeleteGroupList(id))
                    .unwrap();
            }
            GroupListsInput::GroupListParamsSelected(params) => {
                match self.params_selection_reason {
                    GroupListParamsSelectionReason::New => {
                        sender
                            .output(GroupListsUpdateOp::AddNewGroupList(params))
                            .unwrap();
                    }
                    GroupListParamsSelectionReason::Edit(group_list_id) => {
                        sender
                            .output(GroupListsUpdateOp::UpdateGroupList(group_list_id, params))
                            .unwrap();
                    }
                }
            }
            GroupListsInput::GroupListPrefillSelected(prefill) => {
                let group_list_id = self
                    .prefill_group_list_id
                    .take()
                    .expect("There should be a currently edited group list ID");
                sender
                    .output(GroupListsUpdateOp::PrefillGroupList(group_list_id, prefill))
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

    fn update_period_entries(&mut self) {
        let periods_vec: Vec<_> = self
            .periods
            .ordered_period_list
            .iter()
            .enumerate()
            .scan(0usize, |acc, (num, (id, desc))| {
                let out = associations_display::PeriodEntryData {
                    period_id: id.clone(),
                    period_text: super::generate_week_succession_title(
                        "Associations pour la période",
                        &self.periods.first_week,
                        num,
                        *acc,
                        desc.len(),
                    ),
                    subjects: self
                        .subjects
                        .ordered_subject_list
                        .iter()
                        .filter_map(|(subject_id, subject)| {
                            if subject.excluded_periods.contains(id) {
                                return None;
                            }
                            if subject.parameters.interrogation_parameters.is_none() {
                                return None;
                            }

                            Some((subject_id.clone(), subject.clone()))
                        })
                        .collect(),
                    group_list_associations: self
                        .group_lists
                        .subjects_associations
                        .get(id)
                        .expect("Period ID should be valid")
                        .clone(),
                    group_lists: self.group_lists.group_list_map.clone(),
                };

                *acc += desc.len();

                Some(out)
            })
            .collect();
        crate::tools::factories::update_vec_deque(
            &mut self.period_entries,
            periods_vec.into_iter(),
            |data| associations_display::PeriodEntryInput::UpdateData(data),
        );
    }
}
