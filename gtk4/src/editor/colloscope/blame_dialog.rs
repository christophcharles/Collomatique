use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::FactorySender;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

pub struct Dialog {
    hidden: bool,
    move_front: bool,
    warnings: ComputationState,
    messages: FactoryVecDeque<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputationState {
    Debouncing,
    ComputingConstraints,
    RecomputingWarnings,
    ResultAvailable(Result<Vec<String>, String>),
}

impl ComputationState {
    fn as_ref(&self) -> Option<&Result<Vec<String>, String>> {
        match self {
            ComputationState::ResultAvailable(res) => Some(res),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum DialogInput {
    Show,
    Close,
    Update(ComputationState),
}

impl Dialog {
    fn is_debouncing(&self) -> bool {
        match &self.warnings {
            ComputationState::Debouncing => true,
            _ => false,
        }
    }

    fn is_constructing_constraints(&self) -> bool {
        match &self.warnings {
            ComputationState::ComputingConstraints => true,
            _ => false,
        }
    }

    fn is_rebuilding_warnings(&self) -> bool {
        match &self.warnings {
            ComputationState::RecomputingWarnings => true,
            _ => false,
        }
    }
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
                            set_visible: model.is_debouncing(),
                            gtk::Box {
                                set_hexpand: true,
                            },
                            adw::Spinner {
                                set_size_request: (30,30),
                            },
                            gtk::Label {
                                set_label: "En attente des données...",
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
                            #[watch]
                            set_visible: model.is_constructing_constraints(),
                            gtk::Box {
                                set_hexpand: true,
                            },
                            adw::Spinner {
                                set_size_request: (30,30),
                            },
                            gtk::Label {
                                set_label: "Contraintes en cours de construction...",
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
                            #[watch]
                            set_visible: model.is_rebuilding_warnings(),
                            gtk::Box {
                                set_hexpand: true,
                            },
                            adw::Spinner {
                                set_size_request: (30,30),
                            },
                            gtk::Label {
                                set_label: "Vérification du colloscope en cours...",
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
                            set_visible: matches!(&model.warnings, ComputationState::ResultAvailable(Ok(w)) if w.is_empty()),
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
                            set_visible: matches!(&model.warnings, ComputationState::ResultAvailable(Err(_))),
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
                            set_visible: matches!(&model.warnings, ComputationState::ResultAvailable(Ok(w)) if !w.is_empty()),
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
            warnings: ComputationState::ComputingConstraints,
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
        if let ComputationState::ResultAvailable(Ok(warnings)) = &self.warnings {
            messages.extend(warnings.iter().map(|x| EntryData::Warning(x.clone())));
        }
        // On Err, messages stays empty (error shown via label)
        super::super::tools::factories::update_vec_deque(
            &mut self.messages,
            messages.into_iter(),
            EntryInput::Update,
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
        Self { data }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            EntryInput::Update(data) => {
                self.data = data;
            }
        }
    }
}
