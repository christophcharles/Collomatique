use gtk::prelude::{BoxExt, ButtonExt, CheckButtonExt, ObjectExt, OrientableExt, WidgetExt};
use libadwaita::glib::SignalHandlerId;
use relm4::factory::FactoryView;
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::FactorySender;
use relm4::{Component, ComponentController, Controller};

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct PeriodEntryData {
    pub period_id: collomatique_state_colloscopes::PeriodId,
    pub global_first_week: Option<collomatique_time::NaiveMondayDate>,
    pub first_week_num: usize,
    pub week_count: usize,
    pub filtered_subjects: Vec<(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::subjects::Subject<collomatique_state_colloscopes::PeriodId>,
    )>,
    pub filtered_students: Vec<(
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::students::Student<collomatique_state_colloscopes::PeriodId>,
    )>,
    pub period_assignments: collomatique_state_colloscopes::assignments::PeriodAssignments<
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::StudentId,
    >,
}

use crate::tools::dynamic_column_view::{DynamicColumnView, LabelColumn, RelmColumn};

pub struct PeriodEntry {
    index: DynamicIndex,
    data: PeriodEntryData,
    subjects_dropdown: Controller<crate::widgets::droplist::Widget>,
    current_subject: Option<collomatique_state_colloscopes::SubjectId>,
    column_view: DynamicColumnView<StudentItem, gtk::SingleSelection>,
    shown: bool,
}

#[derive(Debug, Clone)]
pub enum PeriodEntryInput {
    UpdateData(PeriodEntryData),
    UpdateStatus(
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::SubjectId,
        bool,
    ),
    CopyPreviousPeriod,
    SubjectDropdownChanged(Option<usize>),
    AssignAll,
    UnassignAll,
    ToggleShown,
}

#[derive(Debug, Clone)]
pub enum PeriodEntryOutput {
    UpdateStatus(
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::SubjectId,
        bool,
    ),
    CopyPreviousPeriod(collomatique_state_colloscopes::PeriodId),
    UpdateStatusAll(
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::SubjectId,
        bool,
    ),
}

impl PeriodEntry {
    fn generate_title_text(&self) -> String {
        format!(
            "<b><big>{}</big></b>",
            super::super::generate_period_title(
                &self.data.global_first_week,
                self.index.current_index(),
                self.data.first_week_num,
                self.data.week_count
            )
        )
    }
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
                gtk::Button {
                    #[watch]
                    set_icon_name: if self.shown {
                        "go-up"
                    } else {
                        "go-down"
                    },
                    add_css_class: "flat",
                    #[watch]
                    set_tooltip_text: Some(if self.shown {
                        "Masquer la période"
                    } else {
                        "Afficher la période"
                    }),
                    connect_clicked => PeriodEntryInput::ToggleShown,
                },
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    #[watch]
                    set_label: &self.generate_title_text(),
                    set_use_markup: true,
                },
                gtk::Button {
                    set_icon_name: "edit-copy-symbolic",
                    add_css_class: "flat",
                    #[watch]
                    set_visible: self.data.first_week_num != 0,
                    set_tooltip_text: Some("Dupliquer les inscriptions de la période précédente"),
                    connect_clicked => PeriodEntryInput::CopyPreviousPeriod,
                },
                gtk::Box {
                    set_hexpand: true,
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,
                    #[watch]
                    set_visible: !self.data.filtered_students.is_empty() && !self.data.filtered_subjects.is_empty() && self.shown,
                    append = &gtk::Box {
                        set_hexpand: true,
                    },
                    append = &gtk::Button {
                        connect_clicked => PeriodEntryInput::AssignAll,
                        gtk::Label {
                            set_label: "Inscrire",
                            set_attributes: Some(&gtk::pango::AttrList::from_string("scale 0.8").unwrap()),
                        }
                    },
                    append = &gtk::Label {
                        set_label: "<small>/</small>",
                        set_use_markup: true,

                    },
                    append = &gtk::Button {
                        connect_clicked => PeriodEntryInput::UnassignAll,
                        gtk::Label {
                            set_label: "désinscrire",
                            set_attributes: Some(&gtk::pango::AttrList::from_string("scale 0.8").unwrap()),
                        }
                    },
                    append = &gtk::Label {
                        set_label: "<small>tous les élèves en</small>",
                        set_use_markup: true,
                    },
                    append: self.subjects_dropdown.widget(),
                },
            },
            gtk::Label {
                #[watch]
                set_visible: self.data.filtered_students.is_empty() && self.shown,
                set_halign: gtk::Align::Start,
                set_label: "<i>Pas d'élèves inscrits sur la période</i>",
                set_use_markup: true,
            },
            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,
                set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Automatic),
                #[watch]
                set_visible: !self.data.filtered_students.is_empty() && self.shown,
                #[local_ref]
                column_view_widget -> gtk::ColumnView {
                    add_css_class: "frame",
                },
            },
        },
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let subjects_dropdown = crate::widgets::droplist::Widget::builder()
            .launch(crate::widgets::droplist::WidgetParams {
                initial_list: vec![],
                initial_selected: None,
                enable_search: false,
                width_request: 100,
            })
            .forward(sender.input_sender(), |msg| match msg {
                crate::widgets::droplist::WidgetOutput::SelectionChanged(num) => {
                    PeriodEntryInput::SubjectDropdownChanged(num)
                }
            });

        let column_view = DynamicColumnView::new();

        let mut model = Self {
            index: index.clone(),
            data,
            column_view,
            subjects_dropdown,
            current_subject: None,
            shown: index.current_index() == 0,
        };

        model.rebuild_columns();
        model.update_subjects_dropdown();
        model.update_view_wrapper(sender);

        model
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let column_view_widget = &self.column_view.view;
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            PeriodEntryInput::UpdateData(new_data) => {
                let should_rebuild_columns =
                    self.data.filtered_subjects != new_data.filtered_subjects;
                self.data = new_data;
                if should_rebuild_columns {
                    self.rebuild_columns();
                    self.update_subjects_dropdown();
                }
                self.update_view_wrapper(sender);
            }
            PeriodEntryInput::UpdateStatus(student_id, subject_id, new_status) => {
                let current_status = self
                    .data
                    .period_assignments
                    .subject_map
                    .get(&subject_id)
                    .expect("Subject id should be valid at this point")
                    .contains(&student_id);

                if current_status == new_status {
                    return;
                }

                sender
                    .output(PeriodEntryOutput::UpdateStatus(
                        self.data.period_id,
                        student_id,
                        subject_id,
                        new_status,
                    ))
                    .unwrap();
            }
            PeriodEntryInput::CopyPreviousPeriod => {
                sender
                    .output(PeriodEntryOutput::CopyPreviousPeriod(self.data.period_id))
                    .unwrap();
            }
            PeriodEntryInput::SubjectDropdownChanged(num) => {
                self.current_subject = num.map(|x| self.data.filtered_subjects[x].0.clone());
            }
            PeriodEntryInput::AssignAll => {
                let Some(subject_id) = self.current_subject else {
                    return;
                };

                sender
                    .output(PeriodEntryOutput::UpdateStatusAll(
                        self.data.period_id,
                        subject_id,
                        true,
                    ))
                    .unwrap();
            }
            PeriodEntryInput::UnassignAll => {
                let Some(subject_id) = self.current_subject else {
                    return;
                };

                sender
                    .output(PeriodEntryOutput::UpdateStatusAll(
                        self.data.period_id,
                        subject_id,
                        false,
                    ))
                    .unwrap();
            }
            PeriodEntryInput::ToggleShown => {
                self.shown = !self.shown;
            }
        }
    }
}

impl PeriodEntry {
    fn rebuild_columns(&mut self) {
        self.column_view.clear_columns();
        self.column_view.append_column(SurnameColumn {});
        self.column_view.append_column(FirstnameColumn {});
        for (subject_id, subject) in &self.data.filtered_subjects {
            self.column_view.append_column(SubjectColumn {
                subject_id: *subject_id,
                subject_name: subject.parameters.name.clone(),
            });
        }
    }

    fn find_subject_pos(&self, id: collomatique_state_colloscopes::SubjectId) -> Option<usize> {
        for (i, (subject_id, _subject)) in self.data.filtered_subjects.iter().enumerate() {
            if *subject_id == id {
                return Some(i);
            }
        }
        None
    }

    fn update_subjects_dropdown(&mut self) {
        let mut list = vec![];

        for (_subject_id, subject) in &self.data.filtered_subjects {
            list.push(subject.parameters.name.clone());
        }

        let num = match self.current_subject {
            None => None,
            Some(id) => self.find_subject_pos(id),
        };

        self.subjects_dropdown
            .sender()
            .send(crate::widgets::droplist::WidgetInput::UpdateList(list, num))
            .unwrap();
    }

    fn update_view_wrapper(&mut self, sender: FactorySender<Self>) {
        self.column_view.splice(
            0,
            self.column_view.len(),
            self.data
                .filtered_students
                .iter()
                .map(|(student_id, student)| StudentItem {
                    student_id: *student_id,
                    surname: student.desc.surname.clone(),
                    firstname: student.desc.firstname.clone(),
                    assigned_subjects: self
                        .data
                        .period_assignments
                        .subject_map
                        .iter()
                        .filter_map(|(subject_id, assigned_students)| {
                            if assigned_students.contains(student_id) {
                                Some(*subject_id)
                            } else {
                                None
                            }
                        })
                        .collect(),
                    sender: sender.clone(),
                    handler_ids: BTreeMap::new(),
                }),
        );
    }
}

struct StudentItem {
    student_id: collomatique_state_colloscopes::StudentId,
    surname: String,
    firstname: String,
    assigned_subjects: BTreeSet<collomatique_state_colloscopes::SubjectId>,
    sender: FactorySender<PeriodEntry>,
    handler_ids: BTreeMap<collomatique_state_colloscopes::SubjectId, SignalHandlerId>,
}

#[derive(Debug, Clone)]
struct FirstnameColumn {}

impl LabelColumn for FirstnameColumn {
    type Item = StudentItem;
    type Value = String;

    fn column_name(&self) -> String {
        "Prénom".into()
    }
    fn sort_enabled(&self) -> bool {
        true
    }
    fn resize_enabled(&self) -> bool {
        true
    }

    fn get_cell_value(&self, item: &Self::Item) -> Self::Value {
        item.firstname.clone()
    }
}

#[derive(Debug, Clone)]
struct SurnameColumn {}

impl LabelColumn for SurnameColumn {
    type Item = StudentItem;
    type Value = String;

    fn column_name(&self) -> String {
        "Nom".into()
    }
    fn sort_enabled(&self) -> bool {
        true
    }
    fn resize_enabled(&self) -> bool {
        true
    }

    fn get_cell_value(&self, item: &Self::Item) -> Self::Value {
        item.surname.clone()
    }
}

#[derive(Debug, Clone)]
struct SubjectColumn {
    subject_id: collomatique_state_colloscopes::SubjectId,
    subject_name: String,
}

impl RelmColumn for SubjectColumn {
    type Root = gtk::CheckButton;
    type Widgets = ();
    type Item = StudentItem;

    fn column_name(&self) -> String {
        self.subject_name.clone()
    }

    fn setup(&self, _item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let root = gtk::CheckButton::new();
        root.set_halign(gtk::Align::Center);

        (root, ())
    }

    fn bind(&self, item: &mut Self::Item, _: &mut Self::Widgets, root: &mut Self::Root) {
        root.set_active(item.assigned_subjects.contains(&self.subject_id));
        let sender = item.sender.clone();
        let student_id = item.student_id;
        let subject_id = self.subject_id;
        item.handler_ids.insert(
            subject_id.clone(),
            root.connect_active_notify(move |widget| {
                let status = widget.is_active();
                sender.input(PeriodEntryInput::UpdateStatus(
                    student_id, subject_id, status,
                ));
            }),
        );
    }

    fn unbind(&self, item: &mut Self::Item, _widgets: &mut Self::Widgets, root: &mut Self::Root) {
        if let Some(id) = item.handler_ids.remove(&self.subject_id) {
            root.disconnect(id);
        }
    }
}
