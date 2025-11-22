use std::collections::BTreeMap;

use adw::prelude::{ComboRowExt, PreferencesRowExt};
use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::FactorySender;
use relm4::RelmWidgetExt;
use relm4::{adw, gtk};

#[derive(Debug)]
pub struct PeriodEntryData {
    pub period_id: collomatique_state_colloscopes::PeriodId,
    pub period_text: String,
    pub subjects: Vec<(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::Subject,
    )>,
    pub group_list_associations: BTreeMap<
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::GroupListId,
    >,
    pub group_lists: BTreeMap<
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::group_lists::GroupList,
    >,
}

pub struct PeriodEntry {
    index: DynamicIndex,
    data: PeriodEntryData,
    subject_entries: FactoryVecDeque<SubjectEntry>,
}

#[derive(Debug)]
pub enum PeriodEntryInput {
    UpdateData(PeriodEntryData),

    UpdateGroupListForSubject(
        collomatique_state_colloscopes::SubjectId,
        Option<collomatique_state_colloscopes::GroupListId>,
    ),
    CopyPreviousPeriod,
}

#[derive(Debug)]
pub enum PeriodEntryOutput {
    UpdateGroupListForSubjectOnPeriod(
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::SubjectId,
        Option<collomatique_state_colloscopes::GroupListId>,
    ),
    CopyPreviousPeriod(collomatique_state_colloscopes::PeriodId),
}

#[relm4::factory(pub)]
impl FactoryComponent for PeriodEntry {
    type Init = PeriodEntryData;
    type Input = PeriodEntryInput;
    type Output = PeriodEntryOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Box {
            set_hexpand: true,
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,
            gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 5,
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    #[watch]
                    set_label: &self.data.period_text,
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                },
                gtk::Button {
                    set_icon_name: "edit-copy-symbolic",
                    add_css_class: "flat",
                    #[watch]
                    set_visible: self.index.current_index() != 0,
                    set_tooltip_text: Some("Dupliquer les associations de la période précédente"),
                    connect_clicked => PeriodEntryInput::CopyPreviousPeriod,
                },
            },
            #[local_ref]
            subject_group -> adw::PreferencesGroup {
                set_margin_all: 5,
                set_hexpand: true,
            },
        },
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let subject_entries = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |msg| match msg {
                SubjectEntryOutput::UpdateGroupListForSubject(subject_id, group_list_id) => {
                    PeriodEntryInput::UpdateGroupListForSubject(subject_id, group_list_id)
                }
            });

        let mut model = Self {
            index: index.clone(),
            data,
            subject_entries,
        };

        model.update_subject_entries();

        model
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let subject_group = self.subject_entries.widget();
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            PeriodEntryInput::UpdateData(new_data) => {
                self.data = new_data;

                self.update_subject_entries();
            }
            PeriodEntryInput::UpdateGroupListForSubject(subject_id, group_list_id) => {
                sender
                    .output(PeriodEntryOutput::UpdateGroupListForSubjectOnPeriod(
                        self.data.period_id,
                        subject_id,
                        group_list_id,
                    ))
                    .unwrap();
            }
            PeriodEntryInput::CopyPreviousPeriod => {
                sender
                    .output(PeriodEntryOutput::CopyPreviousPeriod(self.data.period_id))
                    .unwrap();
            }
        }
    }
}

impl PeriodEntry {
    fn update_subject_entries(&mut self) {
        crate::tools::factories::update_vec_deque(
            &mut self.subject_entries,
            self.data
                .subjects
                .iter()
                .map(|(id, subject)| SubjectEntryData {
                    subject_id: id.clone(),
                    group_list_id: self.data.group_list_associations.get(id).cloned(),
                    subject: subject.clone(),
                    group_lists: self.data.group_lists.clone(),
                }),
            |data| SubjectEntryInput::UpdateData(data),
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SubjectEntryData {
    subject_id: collomatique_state_colloscopes::SubjectId,
    group_list_id: Option<collomatique_state_colloscopes::GroupListId>,
    subject: collomatique_state_colloscopes::subjects::Subject,
    group_lists: BTreeMap<
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::group_lists::GroupList,
    >,
}

struct SubjectEntry {
    data: SubjectEntryData,
    should_redraw: bool,
    group_list_selected: u32,
    ordered_group_lists: Vec<(collomatique_state_colloscopes::GroupListId, String)>,
}

#[derive(Debug)]
enum SubjectEntryInput {
    UpdateData(SubjectEntryData),

    UpdateSelectedGroupList(u32),
}

#[derive(Debug)]
enum SubjectEntryOutput {
    UpdateGroupListForSubject(
        collomatique_state_colloscopes::SubjectId,
        Option<collomatique_state_colloscopes::GroupListId>,
    ),
}

impl SubjectEntry {
    fn update_ordered_group_list(&mut self) {
        self.ordered_group_lists = self
            .data
            .group_lists
            .iter()
            .map(|(id, group_list)| (id.clone(), group_list.params.name.clone()))
            .collect();

        self.ordered_group_lists
            .sort_by_key(|(id, name)| (name.clone(), id.clone()));
    }

    fn generate_group_lists_model(&self) -> gtk::StringList {
        let group_list_names_list: Vec<_> = ["(Aucune liste)"]
            .into_iter()
            .chain(
                self.ordered_group_lists
                    .iter()
                    .map(|(_id, name)| name.as_str()),
            )
            .collect();
        gtk::StringList::new(&group_list_names_list[..])
    }

    fn group_list_id_to_selected(
        &self,
        group_list_id_opt: Option<collomatique_state_colloscopes::GroupListId>,
    ) -> u32 {
        let Some(group_list_id) = group_list_id_opt else {
            return 0;
        };
        for (i, (id, _)) in self.ordered_group_lists.iter().enumerate() {
            if *id == group_list_id {
                return (i as u32) + 1;
            }
        }
        panic!("Group list ID should be in list");
    }

    fn group_list_selected_to_id(
        &self,
        selected: u32,
    ) -> Option<collomatique_state_colloscopes::GroupListId> {
        if selected == 0 {
            return None;
        }
        Some(self.ordered_group_lists[(selected - 1) as usize].0)
    }
}

#[relm4::factory]
impl FactoryComponent for SubjectEntry {
    type Init = SubjectEntryData;
    type Input = SubjectEntryInput;
    type Output = SubjectEntryOutput;
    type CommandOutput = ();
    type ParentWidget = adw::PreferencesGroup;

    view! {
        #[root]
        adw::ComboRow {
            #[watch]
            set_title: &self.data.subject.parameters.name,
            #[track(self.should_redraw)]
            set_model: Some(&self.generate_group_lists_model()),
            #[track(self.should_redraw)]
            set_selected: self.group_list_selected,
            connect_selected_notify[sender] => move |widget| {
                let selected = widget.selected() as u32;
                sender.input(SubjectEntryInput::UpdateSelectedGroupList(selected));
            },
        },
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let mut model = Self {
            data,
            should_redraw: false,
            group_list_selected: 0,
            ordered_group_lists: vec![],
        };

        model.update_ordered_group_list();
        model.group_list_selected = model.group_list_id_to_selected(model.data.group_list_id);

        model
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        // Redrawing generates messages that we should ignore
        if self.should_redraw {
            self.should_redraw = false;
            return;
        }
        match msg {
            SubjectEntryInput::UpdateData(new_data) => {
                if self.data == new_data {
                    return;
                }

                self.should_redraw = true;
                self.data = new_data;

                self.update_ordered_group_list();
                self.group_list_selected = self.group_list_id_to_selected(self.data.group_list_id);
            }
            SubjectEntryInput::UpdateSelectedGroupList(selected_group_list) => {
                if self.group_list_selected == selected_group_list {
                    return;
                }
                self.group_list_selected = selected_group_list;
                self.data.group_list_id = self.group_list_selected_to_id(self.group_list_selected);
                sender
                    .output(SubjectEntryOutput::UpdateGroupListForSubject(
                        self.data.subject_id,
                        self.data.group_list_id,
                    ))
                    .unwrap();
            }
        }
    }
}
