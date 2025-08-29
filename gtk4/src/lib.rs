use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::component::{AsyncComponent, AsyncComponentParts, AsyncComponentSender};
use relm4::RelmWidgetExt;
use relm4::{adw, gtk};
use std::path::PathBuf;

pub struct AppInit {
    pub new: bool,
    pub file_name: Option<PathBuf>,
}

struct FileDesc {
    file_name: Option<PathBuf>,
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

impl AppModel {
    fn generate_title(&self) -> String {
        match &self.current_file {
            Some(file_desc) => match &file_desc.file_name {
                Some(path) => {
                    format!("Collomatique - {}", path.to_string_lossy())
                }
                None => "Collomatique - Fichier sans nom".into(),
            },
            None => "Collomatique".into(),
        }
    }
}

#[relm4::component(async, pub)]
impl AsyncComponent for AppModel {
    type Input = AppInput;
    type Output = ();
    type Init = AppInit;
    type CommandOutput = ();

    view! {
        #[root]
        root_window = adw::ApplicationWindow {
            set_default_width: 800,
            set_default_height: 600,
            #[watch]
            set_title: Some(&model.generate_title()),
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

    async fn init(
        params: Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = AppModel {
            current_file: if params.new || params.file_name.is_some() {
                Some(FileDesc {
                    file_name: params.file_name,
                })
            } else {
                None
            },
        };
        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            AppInput::OpenNewColloscope => {
                self.current_file = Some(FileDesc { file_name: None });
            }
            AppInput::OpenExistingColloscope => {
                // Ignore for now
            }
        }
    }
}
