use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::RelmWidgetExt;
use relm4::{adw, gtk};
use relm4::{Component, ComponentParts, ComponentSender, SimpleComponent, WorkerController};
use std::path::PathBuf;

mod file_loader;

#[derive(Debug)]
pub enum LoadingInput {
    Load(PathBuf),

    Loaded(PathBuf, collomatique_state_colloscopes::Data),
    Failed(PathBuf, String),
}

#[derive(Debug)]
pub enum LoadingOutput {
    Loaded(PathBuf, collomatique_state_colloscopes::Data),
    Failed(PathBuf, String),
}

pub struct LoadingPanel {
    path: Option<PathBuf>,
    worker: Option<WorkerController<file_loader::FileLoader>>,
}

impl LoadingPanel {
    fn generate_loading_text(&self) -> String {
        match &self.path {
            Some(path) => format!("Chargement de {}", path.to_string_lossy()),
            None => String::new(),
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for LoadingPanel {
    type Input = LoadingInput;
    type Output = LoadingOutput;
    type Init = ();

    view! {
        #[root]
        adw::ToolbarView {
            add_top_bar = &adw::HeaderBar {
                pack_end = &gtk::MenuButton {
                    set_icon_name: "open-menu-symbolic",
                    set_menu_model: Some(&main_menu),
                },
            },
            #[wrap(Some)]
            set_content = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 5,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
                set_hexpand: true,
                set_vexpand: true,

                adw::Spinner {
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_size_request: (200, 200),
                },
                gtk::Label {
                    #[watch]
                    set_text: &model.generate_loading_text(),
                }
            },
        },
    }

    menu! {
        main_menu: {
            section! {
                "Nouveau" => super::NewAction,
                "Ouvrir" => super::OpenAction,
            },
            section! {
                "À propos" => super::AboutAction
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = LoadingPanel {
            path: None,
            worker: None,
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            LoadingInput::Load(path) => {
                self.path = Some(path.clone());
                let worker = file_loader::FileLoader::builder()
                    .detach_worker(())
                    .forward(sender.input_sender(), |x| match x {
                        file_loader::FileLoadingOutput::Failed(path, error) => {
                            LoadingInput::Failed(path, error)
                        }
                        file_loader::FileLoadingOutput::Loaded(path, data) => {
                            LoadingInput::Loaded(path, data)
                        }
                    });
                worker
                    .sender()
                    .send(file_loader::FileLoadingInput::Load(path))
                    .unwrap();
                self.worker = Some(worker);
            }
            LoadingInput::Failed(path, error) => {
                sender.output(LoadingOutput::Failed(path, error)).unwrap();
            }
            LoadingInput::Loaded(path, data) => {
                sender.output(LoadingOutput::Loaded(path, data)).unwrap();
            }
        }
    }
}
