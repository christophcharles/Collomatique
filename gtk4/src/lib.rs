use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk, ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use std::path::PathBuf;

pub struct AppInit {
    pub new: bool,
    pub file_name: Option<PathBuf>,
}

struct FileDesc {
    _file_name: Option<PathBuf>,
}

pub struct AppModel {
    current_file: Option<FileDesc>,
}

#[derive(Debug)]
pub enum AppInput {
    OpenNewColloscope,
    OpenExistingColloscope,
}

pub struct AppWidgets {}

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Input = AppInput;
    type Output = ();
    type Init = AppInit;

    view! {
        #[root]
        root_window = adw::ApplicationWindow {
            set_default_width: 800,
            set_default_height: 600,
            set_title: Some("Collomatique"),
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                adw::HeaderBar::new(),
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 5,
                    set_spacing: 5,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_vexpand: true,

                    #[watch]
                    set_visible: model.current_file.is_none(),

                    gtk::Button::with_label("Commencer un nouveau colloscope") {
                        set_margin_all: 5,
                        add_css_class: "suggested-action",
                        connect_clicked => AppInput::OpenNewColloscope,
                    },
                    gtk::Button::with_label("Ouvrir un colloscope existant") {
                        set_margin_all: 5,
                        add_css_class: "suggested-action",
                        connect_clicked => AppInput::OpenExistingColloscope,
                    },
                }
            }
        }
    }

    fn init(
        params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AppModel {
            current_file: if params.new || params.file_name.is_some() {
                Some(FileDesc {
                    _file_name: params.file_name,
                })
            } else {
                None
            },
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppInput::OpenNewColloscope => {
                self.current_file = Some(FileDesc { _file_name: None });
            }
            AppInput::OpenExistingColloscope => {
                // Ignore for now
            }
        }
    }
}
