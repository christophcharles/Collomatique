use gtk::prelude::{OrientableExt, WidgetExt};
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent, RelmWidgetExt};
use relm4::FactorySender;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryData {
    Success(String),
    Failed(String),
    Invalid(String),
}

#[derive(Debug)]
pub struct Entry {
    data: EntryData,
}

impl Entry {
    fn generate_icon_name(&self) -> String {
        match &self.data {
            EntryData::Success(_) => "emblem-success".into(),
            EntryData::Invalid(_) => "emblem-warning".into(),
            EntryData::Failed(_) => "emblem-error".into(),
        }
    }

    fn generate_label(&self) -> String {
        match &self.data {
            EntryData::Success(s) => s.clone(),
            EntryData::Invalid(s) => String::from("(invalide) ") + s,
            EntryData::Failed(s) => String::from("(échec) ") + s,
        }
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for Entry {
    type Init = EntryData;
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        root_widget = gtk::Box {
            set_margin_all: 5,
            set_orientation: gtk::Orientation::Horizontal,
            add_css_class: match &self.data {
                EntryData::Success(_) => "success",
                EntryData::Invalid(_) => "warning",
                EntryData::Failed(_) => "error",
            },
            gtk::Image {
                set_margin_end: 5,
                set_icon_name: Some(&self.generate_icon_name()),
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_label: &self.generate_label(),
            },
        },
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let model = Self { data };

        model
    }

    fn update(&mut self, _msg: Self::Input, _sender: FactorySender<Self>) {}
}
