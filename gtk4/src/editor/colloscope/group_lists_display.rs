use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::FactorySender;
use relm4::RelmWidgetExt;

#[derive(Debug)]
pub struct EntryData {
    pub id: collomatique_state_colloscopes::GroupListId,
    pub group_list: collomatique_state_colloscopes::group_lists::GroupList,
    pub collo_group_list: collomatique_state_colloscopes::colloscopes::ColloscopeGroupList,
    pub total_student_count: usize,
}

pub struct Entry {
    data: EntryData,
    remaining_student_count: usize,
}

#[derive(Debug)]
pub enum EntryInput {
    UpdateData(EntryData),

    EditClicked,
}

#[derive(Debug)]
pub enum EntryOutput {
    EditGroupList(collomatique_state_colloscopes::GroupListId),
}

impl Entry {
    fn generate_list_name(&self) -> String {
        self.data.group_list.params.name.clone()
    }

    fn generate_remaining_student_text(&self) -> String {
        match self.remaining_student_count {
            0 => "liste complète".into(),
            1 => "1 élève sans groupe".into(),
            _ => format!("{} élèves sans groupe", self.remaining_student_count),
        }
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for Entry {
    type Init = EntryData;
    type Input = EntryInput;
    type Output = EntryOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        gtk::Box {
            set_hexpand: true,
            set_margin_all: 5,
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 5,
            gtk::Button {
                set_icon_name: "edit-symbolic",
                add_css_class: "flat",
                connect_clicked => EntryInput::EditClicked,
                set_tooltip_text: Some("Modifier les paramètres"),
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_xalign: 0.,
                set_margin_start: 5,
                set_margin_end: 5,
                #[watch]
                set_label: &self.generate_list_name(),
                set_size_request: (150, -1),
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_margin_start: 5,
                #[watch]
                set_label: &self.generate_remaining_student_text(),
                set_attributes: Some(&gtk::pango::AttrList::from_string("style italic, scale 0.8").unwrap()),
            },
            gtk::Box {
                set_hexpand: true,
            },
        }
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let mut model = Self {
            data,
            remaining_student_count: 0,
        };

        model.update_remaining_student_count();

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
        match msg {
            EntryInput::UpdateData(new_data) => {
                self.data = new_data;
                self.update_remaining_student_count();
            }
            EntryInput::EditClicked => {
                sender
                    .output(EntryOutput::EditGroupList(self.data.id.clone()))
                    .unwrap();
            }
        }
    }
}

impl Entry {
    fn update_remaining_student_count(&mut self) {
        self.remaining_student_count = self.data.total_student_count
            - self.data.group_list.params.excluded_students.len()
            - self.data.collo_group_list.groups_for_students.len();
    }
}
