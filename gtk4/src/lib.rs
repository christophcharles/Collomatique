use gtk::prelude::{BoxExt, GtkWindowExt, WidgetExt};
use relm4::component::{
    AsyncComponent, AsyncComponentParts, AsyncComponentSender, AsyncController,
    SimpleAsyncComponent,
};
use relm4::prelude::AsyncComponentController;
use relm4::{adw, gtk};
use relm4::{Component, ComponentController, Controller};
use std::path::PathBuf;

mod editor;
mod welcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInit {
    pub new: bool,
    pub file_name: Option<PathBuf>,
}

struct AppControllers {
    welcome: Controller<welcome::WelcomePanel>,
    editor: AsyncController<editor::EditorPanel>,
}

enum AppState {
    WelcomeScreen,
    EditorScreen,
}

impl AppState {
    fn is_welcome_screen(&self) -> bool {
        match self {
            AppState::WelcomeScreen => true,
            _ => false,
        }
    }

    fn is_editor_screen(&self) -> bool {
        match self {
            AppState::EditorScreen => true,
            _ => false,
        }
    }
}

pub struct AppModel {
    controllers: AppControllers,
    state: AppState,
}

#[derive(Debug)]
pub enum AppInput {
    Ignore,
    OpenNewColloscope,
    OpenExistingColloscope,
}

#[relm4::component(async, pub)]
impl SimpleAsyncComponent for AppModel {
    type Input = AppInput;
    type Output = ();
    type Init = AppInit;

    view! {
        #[root]
        root_window = adw::ApplicationWindow {
            set_default_width: 800,
            set_default_height: 600,
            gtk::Box {
                set_hexpand: true,
                set_vexpand: true,

                gtk::Box {
                    set_hexpand: true,
                    set_vexpand: true,
                    #[watch]
                    set_visible: model.state.is_welcome_screen(),

                    append: model.controllers.welcome.widget(),
                },
                gtk::Box {
                    set_hexpand: true,
                    set_vexpand: true,
                    #[watch]
                    set_visible: model.state.is_editor_screen(),

                    append: model.controllers.editor.widget(),
                },
            }
        }
    }

    async fn init(
        params: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let editor_payload = if params.new {
            editor::EditorInput::NewFile(params.file_name.clone())
        } else {
            match &params.file_name {
                Some(file_name) => editor::EditorInput::ExistingFile(file_name.clone()),
                None => editor::EditorInput::NewFile(None),
            }
        };

        let editor = editor::EditorPanel::builder()
            .launch(editor_payload)
            .forward(sender.input_sender(), |_msg| AppInput::Ignore);

        let welcome =
            welcome::WelcomePanel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    welcome::WelcomeMessage::OpenNewColloscope => AppInput::OpenNewColloscope,
                    welcome::WelcomeMessage::OpenExistingColloscope => {
                        AppInput::OpenExistingColloscope
                    }
                });

        let controllers = AppControllers { welcome, editor };

        let state = if params.new || params.file_name.is_some() {
            AppState::EditorScreen
        } else {
            AppState::WelcomeScreen
        };

        let model = AppModel { controllers, state };

        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, message: Self::Input, _sender: AsyncComponentSender<Self>) {
        match message {
            AppInput::Ignore => {
                // This message exists only to be ignored (as its name suggests)
            }
            AppInput::OpenNewColloscope => {
                self.state = AppState::EditorScreen;
                self.controllers
                    .editor
                    .sender()
                    .send(editor::EditorInput::NewFile(None))
                    .unwrap();
            }
            AppInput::OpenExistingColloscope => {
                // Ignore for now
            }
        }
    }
}
