use gtk::prelude::{GtkWindowExt, WidgetExt};
use relm4::actions::{AccelsPlus, RelmAction, RelmActionGroup};
use relm4::prelude::ComponentController;
use relm4::{adw, gtk};
use relm4::{Component, ComponentParts, ComponentSender, Controller, SimpleComponent};
use std::path::PathBuf;

mod editor;
mod loading;
mod welcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInit {
    pub new: bool,
    pub file_name: Option<PathBuf>,
}

struct AppControllers {
    welcome: Controller<welcome::WelcomePanel>,
    loading: Controller<loading::LoadingPanel>,
    editor: Controller<editor::EditorPanel>,
}

enum GlobalState {
    WelcomeScreen,
    LoadingScreen,
    EditorScreen,
}

impl GlobalState {
    fn get_screen_name(&self) -> &'static str {
        match self {
            GlobalState::WelcomeScreen => "welcome",
            GlobalState::LoadingScreen => "loading",
            GlobalState::EditorScreen => "editor",
        }
    }
}

pub struct AppModel {
    controllers: AppControllers,
    state: GlobalState,
}

#[derive(Debug)]
pub enum AppInput {
    Ignore,
    NewColloscope(Option<PathBuf>),
    LoadColloscope(PathBuf),
    ColloscopeLoaded(PathBuf, collomatique_state_colloscopes::Data),
    ColloscopeLoadingFailed(PathBuf),
    OpenExistingColloscopeWithDialog,
}

relm4::new_action_group!(AppActionGroup, "app");

relm4::new_stateless_action!(NewAction, AppActionGroup, "new");
relm4::new_stateless_action!(OpenAction, AppActionGroup, "open");
relm4::new_stateless_action!(AboutAction, AppActionGroup, "about");

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Input = AppInput;
    type Output = ();
    type Init = AppInit;

    view! {
        #[root]
        root_window = adw::ApplicationWindow {
            set_default_width: 1280,
            set_default_height: 720,
            set_title: Some("Collomatique"),
            gtk::Stack {
                set_hexpand: true,
                set_vexpand: true,
                add_named: (model.controllers.welcome.widget(), Some("welcome")),
                add_named: (model.controllers.loading.widget(), Some("loading")),
                add_named: (model.controllers.editor.widget(), Some("editor")),
                #[watch]
                set_visible_child_name: model.state.get_screen_name(),
                set_transition_type: gtk::StackTransitionType::Crossfade,
            },
        }
    }

    fn init(
        params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let editor = editor::EditorPanel::builder()
            .launch(())
            .forward(sender.input_sender(), |_msg| AppInput::Ignore);

        let loading =
            loading::LoadingPanel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    loading::LoadingOutput::Loaded(path, data) => {
                        AppInput::ColloscopeLoaded(path, data)
                    }
                    loading::LoadingOutput::Failed(path) => AppInput::ColloscopeLoadingFailed(path),
                });

        let welcome =
            welcome::WelcomePanel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    welcome::WelcomeMessage::OpenNewColloscope => AppInput::NewColloscope(None),
                    welcome::WelcomeMessage::OpenExistingColloscope => {
                        AppInput::OpenExistingColloscopeWithDialog
                    }
                });

        let controllers = AppControllers {
            welcome,
            loading,
            editor,
        };

        let state = GlobalState::WelcomeScreen;

        let model = AppModel { controllers, state };
        let widgets = view_output!();

        let app = relm4::main_application();
        app.set_accelerators_for_action::<NewAction>(&["<primary>N"]);
        app.set_accelerators_for_action::<OpenAction>(&["<primary>O"]);

        let new_action: RelmAction<NewAction> = {
            RelmAction::new_stateless(move |_| {
                //sender.input(Msg::Increment);
            })
        };
        let open_action: RelmAction<OpenAction> = {
            RelmAction::new_stateless(move |_| {
                //sender.input(Msg::Increment);
            })
        };
        let about_action: RelmAction<AboutAction> = {
            RelmAction::new_stateless(move |_| {
                //sender.input(Msg::Increment);
            })
        };

        let mut group = RelmActionGroup::<AppActionGroup>::new();
        group.add_action(new_action);
        group.add_action(open_action);
        group.add_action(about_action);
        group.register_for_main_application();

        sender.input(if params.new {
            AppInput::NewColloscope(params.file_name.clone())
        } else {
            match &params.file_name {
                Some(file_name) => AppInput::LoadColloscope(file_name.clone()),
                None => AppInput::Ignore,
            }
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppInput::Ignore => {
                // This message exists only to be ignored (as its name suggests)
            }
            AppInput::NewColloscope(path) => {
                self.state = GlobalState::EditorScreen;
                self.controllers
                    .editor
                    .sender()
                    .send(editor::EditorInput::NewFile {
                        file_name: path,
                        data: collomatique_state_colloscopes::Data::new(),
                        dirty: true,
                    })
                    .unwrap();
            }
            AppInput::LoadColloscope(path) => {
                self.state = GlobalState::LoadingScreen;
                self.controllers
                    .loading
                    .sender()
                    .send(loading::LoadingInput::Load(path))
                    .unwrap();
            }
            AppInput::OpenExistingColloscopeWithDialog => {
                // Ignore for now
            }
            AppInput::ColloscopeLoaded(path, data) => {
                self.state = GlobalState::EditorScreen;
                self.controllers
                    .editor
                    .sender()
                    .send(editor::EditorInput::NewFile {
                        file_name: Some(path),
                        data,
                        dirty: false,
                    })
                    .unwrap();
            }
            AppInput::ColloscopeLoadingFailed(_path) => {
                // Ignore for now
                // Only restore welcome screen
                self.state = GlobalState::WelcomeScreen;
            }
        }
    }
}
