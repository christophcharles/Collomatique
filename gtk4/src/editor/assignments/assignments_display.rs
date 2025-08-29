use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::typed_view::column::{LabelColumn, TypedColumnView};
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

#[derive(Debug)]
pub struct PeriodEntry {
    index: DynamicIndex,
    data: PeriodEntryData,
    view_wrapper: TypedColumnView<StudentItem, gtk::SingleSelection>,
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
            view_wrapper_widget -> gtk::ColumnView {},
        },
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let mut view_wrapper = TypedColumnView::<StudentItem, gtk::SingleSelection>::new();
        view_wrapper.append_column::<SurnameColumn>();
        view_wrapper.append_column::<FirstnameColumn>();

        let mut model = Self {
            index: index.clone(),
            data,
            view_wrapper,
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
        let view_wrapper_widget = &self.view_wrapper.view;
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            PeriodEntryInput::UpdateData(new_data) => {
                self.data = new_data;
                self.update_view_wrapper();
            }
        }
    }
}

impl PeriodEntry {
    fn update_view_wrapper(&mut self) {
        self.view_wrapper.clear();
        self.view_wrapper
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

struct SurnameColumn;

impl LabelColumn for SurnameColumn {
    type Item = StudentItem;
    type Value = String;

    const COLUMN_NAME: &'static str = "Nom";
    const ENABLE_SORT: bool = true;
    const ENABLE_RESIZE: bool = true;

    fn get_cell_value(item: &Self::Item) -> Self::Value {
        item.surname.clone()
    }

    fn format_cell_value(value: &Self::Value) -> String {
        format!("{} ", value)
    }
}

struct FirstnameColumn;

impl LabelColumn for FirstnameColumn {
    type Item = StudentItem;
    type Value = String;

    const COLUMN_NAME: &'static str = "Prénom";
    const ENABLE_SORT: bool = true;
    const ENABLE_RESIZE: bool = true;

    fn get_cell_value(item: &Self::Item) -> Self::Value {
        item.firstname.clone()
    }

    fn format_cell_value(value: &Self::Value) -> String {
        format!("{} ", value)
    }
}
