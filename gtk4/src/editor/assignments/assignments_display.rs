use glib::object::ObjectExt;
use glib::SignalHandlerId;
use gtk::prelude::{BoxExt, ButtonExt, CheckButtonExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::FactorySender;

use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct PeriodEntryData {
    pub period_id: collomatique_state_colloscopes::PeriodId,
    pub global_first_week: Option<collomatique_time::NaiveMondayDate>,
    pub first_week_num: usize,
    pub week_count: usize,
    pub filtered_subjects: Vec<(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::subjects::Subject,
    )>,
    pub filtered_students: Vec<(
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::students::Student,
    )>,
    pub period_assignments: collomatique_state_colloscopes::assignments::PeriodAssignments,
}

use crate::tools::dynamic_column_view::{DynamicColumnView, LabelColumn, RelmColumn};

#[derive(Debug)]
pub struct PeriodEntry {
    index: DynamicIndex,
    data: PeriodEntryData,
    column_view: DynamicColumnView<StudentItem, gtk::SingleSelection>,
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
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    #[watch]
                    set_label: &self.generate_title_text(),
                    set_use_markup: true,
                },
                gtk::Box {
                    set_hexpand: true,
                },
                gtk::Button {
                    set_icon_name: "edit-copy-symbolic",
                    add_css_class: "flat",
                    #[watch]
                    set_visible: self.data.first_week_num != 0,
                    set_tooltip_text: Some("Dupliquer les inscriptions de la période précédente"),
                    connect_clicked => PeriodEntryInput::CopyPreviousPeriod,
                },
            },
            gtk::Label {
                #[watch]
                set_visible: self.data.filtered_students.is_empty(),
                set_halign: gtk::Align::Start,
                set_label: "<i>Pas d'élèves inscrits sur la période</i>",
                set_use_markup: true,
            },
            #[local_ref]
            column_view_widget -> gtk::ColumnView {
                add_css_class: "frame",
                #[watch]
                set_visible: !self.data.filtered_students.is_empty(),
            },
        },
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let column_view = DynamicColumnView::new();

        let mut model = Self {
            index: index.clone(),
            data,
            column_view,
        };

        model.rebuild_columns();
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

    fn update_view_wrapper(&mut self, sender: FactorySender<Self>) {
        self.column_view.clear();
        self.column_view
            .extend_from_iter(
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
                        handler_id: None,
                    }),
            );
    }
}

#[derive(Debug)]
struct StudentItem {
    student_id: collomatique_state_colloscopes::StudentId,
    surname: String,
    firstname: String,
    assigned_subjects: BTreeSet<collomatique_state_colloscopes::SubjectId>,
    sender: FactorySender<PeriodEntry>,
    handler_id: Option<SignalHandlerId>,
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
        item.handler_id = Some(root.connect_active_notify(move |widget| {
            let status = widget.is_active();
            sender.input(PeriodEntryInput::UpdateStatus(
                student_id, subject_id, status,
            ));
        }));
    }

    fn unbind(&self, item: &mut Self::Item, _widgets: &mut Self::Widgets, root: &mut Self::Root) {
        if let Some(id) = item.handler_id.take() {
            root.disconnect(id);
        }
    }
}
