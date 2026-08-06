use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::typed_view::list::{RelmListItem, TypedListView};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use collomatique_constraints_colloscopes::SeverityLevel;

pub struct Dialog {
    hidden: bool,
    move_front: bool,
    warnings: ComputationState,
    messages: TypedListView<BlameEntry, gtk::SingleSelection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputationState {
    Debouncing,
    ComputingConstraints,
    RecomputingWarnings,
    ResultAvailable(Result<Vec<(SeverityLevel, String)>, String>),
}

impl ComputationState {
    fn as_ref(&self) -> Option<&Result<Vec<(SeverityLevel, String)>, String>> {
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
                                set_pixel_size: 30,
                                set_icon_name: Some("object-select-symbolic"),
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
                                set_pixel_size: 30,
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
                        messages_listview -> gtk::ListView {
                            set_hexpand: true,
                            set_vexpand: true,
                            add_css_class: "boxed-list",
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
        let messages: TypedListView<BlameEntry, gtk::SingleSelection> = TypedListView::new();

        let model = Dialog {
            hidden: true,
            move_front: false,
            warnings: ComputationState::ComputingConstraints,
            messages,
        };

        let messages_listview = &model.messages.view;

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
        self.messages.clear();
        if let ComputationState::ResultAvailable(Ok(warnings)) = &self.warnings {
            self.messages
                .extend_from_iter(warnings.iter().map(|(s, m)| BlameEntry {
                    severity: *s,
                    message: m.clone(),
                }));
        }
    }
}

struct BlameEntry {
    severity: SeverityLevel,
    message: String,
}

struct BlameEntryWidgets {
    icon: gtk::Image,
    label: gtk::Label,
}

impl BlameEntry {
    fn icon_name(&self) -> &'static str {
        match self.severity {
            SeverityLevel::Infeasibility => "computer-fail-symbolic",
            SeverityLevel::Structural | SeverityLevel::Quality => "dialog-error-symbolic",
            SeverityLevel::Progressive => "dialog-warning-symbolic",
            SeverityLevel::Preference => "dialog-information-symbolic",
        }
    }

    fn css_class(&self) -> &'static str {
        match self.severity {
            SeverityLevel::Infeasibility | SeverityLevel::Structural | SeverityLevel::Quality => {
                "error"
            }
            SeverityLevel::Progressive | SeverityLevel::Preference => "warning",
        }
    }
}

impl RelmListItem for BlameEntry {
    type Root = gtk::Box;
    type Widgets = BlameEntryWidgets;

    fn setup(_list_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .margin_start(5)
            .margin_end(5)
            .margin_top(5)
            .margin_bottom(5)
            .build();

        let icon = gtk::Image::builder().margin_end(5).build();
        let label = gtk::Label::builder().halign(gtk::Align::Start).build();

        root.append(&icon);
        root.append(&label);

        (root, BlameEntryWidgets { icon, label })
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, root: &mut Self::Root) {
        root.remove_css_class("error");
        root.remove_css_class("warning");
        root.add_css_class(self.css_class());
        widgets.icon.set_icon_name(Some(self.icon_name()));
        widgets.label.set_label(&self.message);
    }
}
