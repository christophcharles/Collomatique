use collo_ml::eval::Origin;
use collomatique_binding_colloscopes::views::ObjectId;
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::FactorySender;
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    move_front: bool,
    warnings: Option<Result<Vec<Origin<ObjectId>>, String>>,
    messages: FactoryVecDeque<Entry>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show,
    Close,
    Update(Option<Result<Vec<Origin<ObjectId>>, String>>),
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = ();

    view! {
        #[root]
        root_window = adw::Window {
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Erreurs dans le colloscope"),
            set_size_request: (500, 400),

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_end = &gtk::Button {
                        set_label: "Fermer",
                        add_css_class: "suggested-action",
                        connect_clicked => DialogInput::Close,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_margin_all: 5,
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        gtk::Box {
                            set_hexpand: true,
                            set_vexpand: true,
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 10,
                            #[watch]
                            set_visible: model.warnings.is_none(),
                            gtk::Box {
                                set_hexpand: true,
                            },
                            adw::Spinner {
                                set_size_request: (30,30),
                            },
                            gtk::Label {
                                set_label: "Constraintes en cours de reconstruction...",
                                set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                        },
                        gtk::Box {
                            set_hexpand: true,
                            set_vexpand: true,
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 10,
                            gtk::Box {
                                set_hexpand: true,
                            },
                            #[watch]
                            set_visible: matches!(&model.warnings, Some(Ok(w)) if w.is_empty()),
                            gtk::Image {
                                set_icon_size: gtk::IconSize::Large,
                                set_icon_name: Some("emblem-ok-symbolic"),
                            },
                            gtk::Label {
                                set_label: "Toutes les contraintes sont satisfaites",
                                set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                        },
                        gtk::Box {
                            set_hexpand: true,
                            set_vexpand: true,
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 10,
                            add_css_class: "error",
                            gtk::Box {
                                set_hexpand: true,
                            },
                            #[watch]
                            set_visible: matches!(&model.warnings, Some(Err(_))),
                            gtk::Image {
                                set_icon_size: gtk::IconSize::Large,
                                set_icon_name: Some("dialog-error-symbolic"),
                            },
                            gtk::Label {
                                #[watch]
                                set_label: &model.warnings.as_ref()
                                    .and_then(|r| r.as_ref().err())
                                    .map(|e| e.to_string())
                                    .unwrap_or_default(),
                                set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                        },
                        #[local_ref]
                        messages_listbox -> gtk::ListBox {
                            set_hexpand: true,
                            set_vexpand: true,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::Single,
                            #[watch]
                            set_visible: matches!(&model.warnings, Some(Ok(w)) if !w.is_empty()),
                        }
                    },
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let messages = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .detach();

        let model = Dialog {
            hidden: true,
            move_front: false,
            warnings: None,
            messages,
        };

        let messages_listbox = model.messages.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        self.move_front = false;
        match msg {
            DialogInput::Show => {
                self.hidden = false;
                self.move_front = true;
            }
            DialogInput::Close => {
                self.hidden = true;
            }
            DialogInput::Update(warnings) => {
                self.warnings = warnings;
                self.update_messages();
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
    }
}

impl Dialog {
    fn update_messages(&mut self) {
        let mut messages = vec![];
        if let Some(Ok(warnings)) = &self.warnings {
            messages.extend(warnings.iter().map(|x| EntryData::Warning(x.to_string())));
        }
        // On Err, messages stays empty (error shown via label)
        super::super::tools::factories::update_vec_deque(
            &mut self.messages,
            messages.into_iter(),
            |x| EntryInput::Update(x),
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EntryData {
    Warning(String),
}

#[derive(Debug)]
struct Entry {
    data: EntryData,
}

#[derive(Debug)]
enum EntryInput {
    Update(EntryData),
}

impl Entry {
    fn generate_icon_name(&self) -> String {
        match &self.data {
            EntryData::Warning(_) => "dialog-warning-symbolic".into(),
        }
    }

    fn generate_label(&self) -> String {
        match &self.data {
            EntryData::Warning(s) => s.clone(),
        }
    }
}

#[relm4::factory]
impl FactoryComponent for Entry {
    type Init = EntryData;
    type Input = EntryInput;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        root_widget = gtk::Box {
            set_margin_all: 5,
            set_orientation: gtk::Orientation::Horizontal,
            #[watch]
            add_css_class: match &self.data {
                EntryData::Warning(_) => "warning",
            },
            gtk::Image {
                set_margin_end: 5,
                #[watch]
                set_icon_name: Some(&self.generate_icon_name()),
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                #[watch]
                set_label: &self.generate_label(),
            },
        },
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let model = Self { data };

        model
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            EntryInput::Update(data) => {
                self.data = data;
            }
        }
    }
}
