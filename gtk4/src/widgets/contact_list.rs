use gtk::prelude::{ButtonExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::FactorySender;
use relm4::{gtk, Component};
use relm4::{ComponentParts, ComponentSender};

pub trait Identifier:
    Clone + std::fmt::Debug + PartialEq + Eq + PartialOrd + Ord + Send + Sync + 'static
{
}

impl<T: Clone + std::fmt::Debug + PartialEq + Eq + PartialOrd + Ord + Send + Sync + 'static>
    Identifier for T
{
}

#[derive(Debug, Clone)]
pub struct ContactInfo<Id: Identifier> {
    pub id: Id,
    pub contact: collomatique_state_colloscopes::PersonWithContact,
    pub extra: String,
}

#[derive(Debug)]
pub enum WidgetInput<Id: Identifier> {
    UpdateList(Vec<ContactInfo<Id>>),
}

#[derive(Debug)]
pub enum WidgetOutput<Id: Identifier> {
    EditContact(Id),
    DeleteContact(Id),
}

pub struct Widget<Id: Identifier> {
    current_list: Vec<ContactInfo<Id>>,
    entries: FactoryVecDeque<Entry<Id>>,
}

#[relm4::component(pub)]
impl<Id: Identifier> Component for Widget<Id> {
    type Input = WidgetInput<Id>;
    type Output = WidgetOutput<Id>;
    type Init = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            #[watch]
            set_visible: !model.current_list.is_empty(),
            #[local_ref]
            entries_widget -> gtk::ListBox {
                set_hexpand: true,
                add_css_class: "boxed-list",
                set_selection_mode: gtk::SelectionMode::None,
            },
        }

    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let entries = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.output_sender(), |msg| match msg {
                EntryOutput::EditContact(id) => WidgetOutput::EditContact(id),
                EntryOutput::DeleteContact(id) => WidgetOutput::DeleteContact(id),
            });
        let model = Widget {
            current_list: vec![],
            entries,
        };

        let entries_widget = model.entries.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            WidgetInput::UpdateList(new_list) => {
                self.current_list = new_list;
                crate::tools::factories::update_vec_deque(
                    &mut self.entries,
                    self.current_list.iter().cloned(),
                    |data| EntryInput::UpdateData(data),
                );
            }
        }
    }
}

pub struct Entry<Id: Identifier> {
    data: ContactInfo<Id>,
}

#[derive(Debug)]
pub enum EntryInput<Id: Identifier> {
    UpdateData(ContactInfo<Id>),

    EditClicked,
    DeleteClicked,
}

#[derive(Debug)]
pub enum EntryOutput<Id: Identifier> {
    EditContact(Id),
    DeleteContact(Id),
}

impl<Id: Identifier> Entry<Id> {
    fn generate_name_text(&self) -> String {
        format!(
            "{} {}",
            self.data.contact.firstname, self.data.contact.surname,
        )
    }

    fn generate_telephone_text(&self) -> String {
        match &self.data.contact.tel {
            Some(t) => t.clone().into_inner(),
            None => "<i>Non renseigné</i>".into(),
        }
    }

    fn generate_email_text(&self) -> String {
        match &self.data.contact.email {
            Some(e) => e.clone().into_inner(),
            None => "<i>Non renseigné</i>".into(),
        }
    }
}

#[relm4::factory(pub)]
impl<Id: Identifier> FactoryComponent for Entry<Id> {
    type Init = ContactInfo<Id>;
    type Input = EntryInput<Id>;
    type Output = EntryOutput<Id>;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        gtk::Box {
            set_hexpand: true,
            set_orientation: gtk::Orientation::Horizontal,
            gtk::Button {
                set_icon_name: "edit-symbolic",
                add_css_class: "flat",
                connect_clicked => EntryInput::EditClicked,
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
                set_label: &self.generate_name_text(),
                set_size_request: (200, -1),
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
            },
            gtk::Image {
                set_halign: gtk::Align::Start,
                set_margin_start: 5,
                set_margin_end: 5,
                set_icon_name: Some("contact-symbolic"),
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_margin_end: 5,
                #[watch]
                set_label: &self.generate_telephone_text(),
                #[watch]
                set_use_markup: self.data.contact.tel.is_none(),
                set_size_request: (120, -1),
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
            },
            gtk::Image {
                set_halign: gtk::Align::Start,
                set_margin_start: 5,
                set_margin_end: 5,
                set_icon_name: Some("emblem-mail-symbolic"),
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_xalign: 0.,
                set_margin_end: 5,
                #[watch]
                set_label: &self.generate_email_text(),
                #[watch]
                set_use_markup: self.data.contact.email.is_none(),
            },
            gtk::Box {
                set_hexpand: true,
            },
            gtk::Label {
                set_halign: gtk::Align::End,
                set_margin_end: 5,
                #[watch]
                set_label: &self.data.extra,
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
            },
            gtk::Button {
                set_icon_name: "edit-delete",
                add_css_class: "flat",
                connect_clicked => EntryInput::DeleteClicked,
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
                    .output(EntryOutput::EditContact(self.data.id.clone()))
                    .unwrap();
            }
            EntryInput::DeleteClicked => {
                sender
                    .output(EntryOutput::DeleteContact(self.data.id.clone()))
                    .unwrap();
            }
        }
    }
}
