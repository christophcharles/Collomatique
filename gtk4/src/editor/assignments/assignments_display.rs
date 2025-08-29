use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::FactorySender;

#[derive(Debug, Clone)]
pub struct PeriodEntryData {
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

use crate::tools::dynamic_column_view::{DynamicColumnView, LabelColumn};

#[derive(Debug)]
pub struct PeriodEntry {
    index: DynamicIndex,
    data: PeriodEntryData,
    column_view: DynamicColumnView<StudentItem, gtk::SingleSelection>,
}

#[derive(Debug, Clone)]
pub enum PeriodEntryInput {
    UpdateData(PeriodEntryData),
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
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Box {
            set_hexpand: true,
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,
            gtk::Label {
                set_halign: gtk::Align::Start,
                #[watch]
                set_label: &self.generate_title_text(),
                set_use_markup: true,
            },
            #[local_ref]
            column_view_widget -> gtk::ColumnView {},
        },
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let mut column_view = DynamicColumnView::new();
        column_view.append_column(SurnameColumn {});
        column_view.append_column(FirstnameColumn {});

        let mut model = Self {
            index: index.clone(),
            data,
            column_view,
        };

        model.update_view_wrapper();

        model
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let column_view_widget = &self.column_view.view;
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            PeriodEntryInput::UpdateData(new_data) => {
                let should_rebuild_columns =
                    self.data.filtered_subjects != new_data.filtered_subjects;
                self.data = new_data;
                if should_rebuild_columns {
                    self.rebuild_columns();
                }
                self.update_view_wrapper();
            }
        }
    }
}

impl PeriodEntry {
    fn rebuild_columns(&mut self) {
        self.column_view.clear_columns();
        self.column_view.append_column(SurnameColumn {});
        self.column_view.append_column(FirstnameColumn {});
    }

    fn update_view_wrapper(&mut self) {
        self.column_view.clear();
        self.column_view
            .extend_from_iter(self.data.filtered_students.iter().map(|(_id, student)| {
                StudentItem {
                    surname: student.desc.surname.clone(),
                    firstname: student.desc.firstname.clone(),
                }
            }));
    }
}

#[derive(Debug, PartialEq, Eq)]
struct StudentItem {
    surname: String,
    firstname: String,
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
