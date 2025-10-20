use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::FactorySender;

#[derive(Debug, Clone)]
pub struct EntryData {
    pub subject_params: collomatique_state_colloscopes::SubjectParameters,
    pub subject_id: collomatique_state_colloscopes::SubjectId,
}

#[derive(Debug)]
pub struct Entry {
    subject_params: collomatique_state_colloscopes::SubjectParameters,
    subject_id: collomatique_state_colloscopes::SubjectId,
}

#[derive(Debug, Clone)]
pub enum EntryInput {
    UpdateData(EntryData),
}

#[derive(Debug)]
pub enum EntryOutput {}

#[relm4::factory(pub)]
impl FactoryComponent for Entry {
    type Init = EntryData;
    type Input = EntryInput;
    type Output = EntryOutput;
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
                set_label: &self.subject_params.name,
                set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_label: "Stub",
            },
        },
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let model = Self {
            subject_params: data.subject_params,
            subject_id: data.subject_id,
        };

        model
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            EntryInput::UpdateData(new_data) => {
                self.subject_params = new_data.subject_params;
                self.subject_id = new_data.subject_id;
            }
        }
    }
}
