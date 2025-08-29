use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::RelmWidgetExt;
use relm4::{adw, gtk};
use relm4::{Component, ComponentParts, ComponentSender};
use std::path::PathBuf;

#[derive(Debug)]
pub enum LoadingInput {
    Load(PathBuf),
    StopLoading,
}

#[derive(Debug)]
pub enum LoadingOutput {
    Loaded(PathBuf, collomatique_state_colloscopes::Data),
    Failed(PathBuf, String),
}

#[derive(Debug)]
pub enum LoadingCmdOutput {
    Loaded(PathBuf, collomatique_state_colloscopes::Data),
    Failed(PathBuf, String),
}

pub struct LoadingPanel {
    path: Option<PathBuf>,
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
impl Component for LoadingPanel {
    type Input = LoadingInput;
    type Output = LoadingOutput;
    type Init = ();
    type CommandOutput = LoadingCmdOutput;

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
                "Fermer" => super::CloseAction
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
        let model = LoadingPanel { path: None };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            LoadingInput::Load(path) => {
                if Some(&path) == self.path.as_ref() {
                    return;
                }
                self.path = Some(path.clone());
                sender.oneshot_command(async move {
                    match collomatique_storage::load_data_from_file(&path).await {
                        Ok((data, _caveats)) => LoadingCmdOutput::Loaded(path, data),
                        Err(e) => LoadingCmdOutput::Failed(path, e.to_string()),
                    }
                });
            }
            LoadingInput::StopLoading => {
                self.path = None;
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            LoadingCmdOutput::Loaded(path, data) => {
                if Some(&path) != self.path.as_ref() {
                    return;
                }
                self.path = None;
                sender.output(LoadingOutput::Loaded(path, data)).unwrap();
            }
            LoadingCmdOutput::Failed(path, error) => {
                if Some(&path) != self.path.as_ref() {
                    return;
                }
                self.path = None;
                sender.output(LoadingOutput::Failed(path, error)).unwrap();
            }
        }
    }
}
