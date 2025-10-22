use std::collections::BTreeMap;

use adw::prelude::PreferencesRowExt;
use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::FactorySender;
use relm4::RelmWidgetExt;
use relm4::{adw, gtk};

#[derive(Debug)]
pub struct PeriodEntryData {
    pub period_id: collomatique_state_colloscopes::PeriodId,
    pub period_text: String,
    pub subjects: Vec<(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::Subject<collomatique_state_colloscopes::PeriodId>,
    )>,
    pub group_list_associations: BTreeMap<
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::GroupListId,
    >,
}

pub struct PeriodEntry {
    data: PeriodEntryData,
}

#[derive(Debug)]
pub enum PeriodEntryInput {
    UpdateData(PeriodEntryData),
}

#[derive(Debug)]
pub enum PeriodEntryOutput {}

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
            gtk::Label {
                set_halign: gtk::Align::Start,
                #[watch]
                set_label: &self.data.period_text,
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
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let model = Self { data };

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
            PeriodEntryInput::UpdateData(new_data) => {
                self.data = new_data;
            }
        }
    }
}
