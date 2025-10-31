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
}

pub struct Entry {
    data: EntryData,
}

#[derive(Debug)]
pub enum EntryInput {
    UpdateData(EntryData),

    EditClicked,
    PrefillClicked,
    DeleteClicked,
}

#[derive(Debug)]
pub enum EntryOutput {
    EditGroupList(collomatique_state_colloscopes::GroupListId),
    PrefillGroupList(collomatique_state_colloscopes::GroupListId),
    DeleteGroupList(collomatique_state_colloscopes::GroupListId),
}

impl Entry {
    fn generate_list_name(&self) -> String {
        self.data.group_list.params.name.clone()
    }

    fn generate_students_per_group_text(&self) -> String {
        let range = &self.data.group_list.params.students_per_group;
        if range.start() == range.end() {
            format!("<b>Élèves par groupe :</b> {}", range.start())
        } else {
            format!(
                "<b>Élèves par groupe :</b> {} à {}",
                range.start(),
                range.end()
            )
        }
    }

    fn generate_group_count_text(&self) -> String {
        let range = &self.data.group_list.params.group_count;
        if range.start() == range.end() {
            format!("<b>Nombre de groupes :</b> {}", range.start())
        } else {
            format!(
                "<b>Nombre de groupes :</b> {} à {}",
                range.start(),
                range.end()
            )
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
            gtk::Button {
                set_icon_name: "view-list-bullet-symbolic",
                add_css_class: "flat",
                connect_clicked => EntryInput::PrefillClicked,
                set_tooltip_text: Some("Préremplir la liste"),
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
                set_label: &self.generate_students_per_group_text(),
                set_use_markup: true,
                set_size_request: (200, -1),
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Horizontal,
                add_css_class: "spacer",
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_xalign: 0.,
                set_margin_start: 5,
                set_margin_end: 5,
                #[watch]
                set_label: &self.generate_group_count_text(),
                set_use_markup: true,
            },
            gtk::Box {
                set_hexpand: true,
            },
            gtk::Label {
                set_halign: gtk::Align::End,
                set_margin_end: 5,
                set_label: "Liste préremplie",
                set_attributes: Some(&gtk::pango::AttrList::from_string("style italic, scale 0.8").unwrap()),
                #[watch]
                set_visible: !self.data.group_list.prefilled_groups.is_empty() && !self.data.group_list.is_sealed(),
            },
            gtk::Label {
                set_halign: gtk::Align::End,
                set_margin_end: 5,
                set_label: "Liste scellée",
                set_attributes: Some(&gtk::pango::AttrList::from_string("style italic, scale 0.8").unwrap()),
                #[watch]
                set_visible: self.data.group_list.is_sealed(),
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
            },
            gtk::Button {
                set_icon_name: "edit-delete",
                add_css_class: "flat",
                connect_clicked => EntryInput::DeleteClicked,
                set_tooltip_text: Some("Supprimer la liste"),
            },
        }
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
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            EntryInput::UpdateData(new_data) => {
                self.data = new_data;
            }
            EntryInput::EditClicked => {
                sender
                    .output(EntryOutput::EditGroupList(self.data.id.clone()))
                    .unwrap();
            }
            EntryInput::PrefillClicked => {
                sender
                    .output(EntryOutput::PrefillGroupList(self.data.id.clone()))
                    .unwrap();
            }
            EntryInput::DeleteClicked => {
                sender
                    .output(EntryOutput::DeleteGroupList(self.data.id.clone()))
                    .unwrap();
            }
        }
    }
}
