use gtk::prelude::{ApplicationExt, GtkWindowExt, WidgetExt};
use relm4::actions::{AccelsPlus, RelmAction, RelmActionGroup};
use relm4::prelude::ComponentController;
use relm4::{adw, gtk};
use relm4::{Component, ComponentParts, ComponentSender, Controller, SimpleComponent};
use std::path::PathBuf;

#[allow(dead_code)]
mod dialogs;

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

    file_error: Controller<dialogs::file_error::Dialog>,
    open_dialog: Controller<dialogs::open_save::Dialog>,
    warn_dirty: Controller<dialogs::warning_changed::Dialog>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

pub struct AppActions {
    save_action: RelmAction<SaveAction>,
    undo_action: RelmAction<UndoAction>,
    redo_action: RelmAction<RedoAction>,
}

pub struct AppModel {
    controllers: AppControllers,
    actions: AppActions,
    state: GlobalState,
    next_warn_msg: Option<AppInput>,
}

impl AppModel {
    fn send_but_check_dirty(&mut self, sender: ComponentSender<Self>, msg: AppInput) {
        if self.controllers.editor.model().is_dirty() {
            self.next_warn_msg = Some(msg);
            sender.input(AppInput::WarnDirty);
        } else {
            sender.input(msg);
        }
    }
}

#[derive(Debug)]
pub enum AppInput {
    Ignore,
    WarnDirty,
    OkDirty,
    RequestNewColloscope,
    NewColloscope(Option<PathBuf>),
    LoadColloscope(PathBuf),
    ColloscopeLoaded(PathBuf, collomatique_state_colloscopes::Data),
    ColloscopeLoadingFailed(PathBuf, String),
    RequestOpenExistingColloscopeWithDialog,
    OpenExistingColloscopeWithDialog,
    RequestQuit,
    Quit,
    RequestCloseFile,
    CloseFile,
    RequestSave,
    RequestSaveAs,
    RequestUndo,
    RequestRedo,
    UpdateActions,
}

relm4::new_action_group!(AppActionGroup, "app");

relm4::new_stateless_action!(NewAction, AppActionGroup, "new");
relm4::new_stateless_action!(OpenAction, AppActionGroup, "open");
relm4::new_stateless_action!(SaveAction, AppActionGroup, "save");
relm4::new_stateless_action!(SaveAsAction, AppActionGroup, "save_as");
relm4::new_stateless_action!(UndoAction, AppActionGroup, "undo");
relm4::new_stateless_action!(RedoAction, AppActionGroup, "redo");
relm4::new_stateless_action!(CloseAction, AppActionGroup, "close");
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

            connect_close_request[sender] => move |_| {
                sender.input(AppInput::RequestQuit);
                gtk::glib::Propagation::Stop
            }
        }
    }

    fn init(
        params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let editor = editor::EditorPanel::builder().launch(()).forward(
            sender.input_sender(),
            |msg| match msg {
                editor::EditorOutput::UpdateActions => AppInput::UpdateActions,
            },
        );

        let loading =
            loading::LoadingPanel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    loading::LoadingOutput::Loaded(path, data) => {
                        AppInput::ColloscopeLoaded(path, data)
                    }
                    loading::LoadingOutput::Failed(path, error) => {
                        AppInput::ColloscopeLoadingFailed(path, error)
                    }
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

        let file_error = dialogs::file_error::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |_| AppInput::Ignore);

        let open_dialog = dialogs::open_save::Dialog::builder()
            .transient_for_native(&root)
            .launch(dialogs::open_save::Type::Open)
            .forward(sender.input_sender(), |msg| match msg {
                dialogs::open_save::DialogOutput::Cancel => AppInput::Ignore,
                dialogs::open_save::DialogOutput::FileSelected(path) => {
                    AppInput::LoadColloscope(path)
                }
            });

        let warn_dirty = dialogs::warning_changed::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                dialogs::warning_changed::DialogOutput::Accept => AppInput::OkDirty,
            });

        let controllers = AppControllers {
            welcome,
            loading,
            editor,
            file_error,
            open_dialog,
            warn_dirty,
        };

        let state = GlobalState::WelcomeScreen;

        let app = relm4::main_application();
        app.set_accelerators_for_action::<NewAction>(&["<primary>N"]);
        app.set_accelerators_for_action::<OpenAction>(&["<primary>O"]);
        app.set_accelerators_for_action::<SaveAction>(&["<primary>S"]);
        app.set_accelerators_for_action::<UndoAction>(&["<primary>Z"]);
        app.set_accelerators_for_action::<RedoAction>(&["<shift><primary>Z"]);
        app.set_accelerators_for_action::<CloseAction>(&["<primary>W"]);

        let new_action: RelmAction<NewAction> = {
            let sender = sender.clone();
            RelmAction::new_stateless(move |_| {
                sender.input(AppInput::RequestNewColloscope);
            })
        };
        let open_action: RelmAction<OpenAction> = {
            let sender = sender.clone();
            RelmAction::new_stateless(move |_| {
                sender.input(AppInput::RequestOpenExistingColloscopeWithDialog);
            })
        };
        let save_action: RelmAction<SaveAction> = {
            let sender = sender.clone();
            RelmAction::new_stateless(move |_| {
                sender.input(AppInput::RequestSave);
            })
        };
        let save_as_action: RelmAction<SaveAsAction> = {
            let sender = sender.clone();
            RelmAction::new_stateless(move |_| {
                sender.input(AppInput::RequestSaveAs);
            })
        };
        let undo_action: RelmAction<UndoAction> = {
            let sender = sender.clone();
            RelmAction::new_stateless(move |_| {
                sender.input(AppInput::RequestUndo);
            })
        };
        let redo_action: RelmAction<RedoAction> = {
            let sender = sender.clone();
            RelmAction::new_stateless(move |_| {
                sender.input(AppInput::RequestRedo);
            })
        };
        let close_action: RelmAction<CloseAction> = {
            let sender = sender.clone();
            RelmAction::new_stateless(move |_| {
                sender.input(AppInput::RequestCloseFile);
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
        group.add_action(save_action.clone());
        group.add_action(save_as_action);
        group.add_action(undo_action.clone());
        group.add_action(redo_action.clone());
        group.add_action(close_action);
        group.add_action(about_action);
        group.register_for_main_application();

        save_action.set_enabled(false);
        undo_action.set_enabled(false);
        redo_action.set_enabled(false);

        let actions = AppActions {
            save_action,
            undo_action,
            redo_action,
        };

        let model = AppModel {
            controllers,
            state,
            next_warn_msg: None,
            actions,
        };
        let widgets = view_output!();

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

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            AppInput::Ignore => {
                // This message exists only to be ignored (as its name suggests)
            }
            AppInput::RequestNewColloscope => {
                self.send_but_check_dirty(sender, AppInput::NewColloscope(None));
            }
            AppInput::NewColloscope(path) => {
                self.controllers
                    .loading
                    .sender()
                    .send(loading::LoadingInput::StopLoading)
                    .unwrap();
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
                self.controllers
                    .loading
                    .sender()
                    .send(loading::LoadingInput::StopLoading)
                    .unwrap();
                self.controllers
                    .editor
                    .sender()
                    .send(editor::EditorInput::NewFile {
                        file_name: None,
                        data: collomatique_state_colloscopes::Data::new(),
                        dirty: false,
                    })
                    .unwrap();
                self.state = GlobalState::LoadingScreen;
                self.controllers
                    .loading
                    .sender()
                    .send(loading::LoadingInput::Load(path))
                    .unwrap();
            }
            AppInput::RequestOpenExistingColloscopeWithDialog => {
                self.send_but_check_dirty(sender, AppInput::OpenExistingColloscopeWithDialog);
            }
            AppInput::OpenExistingColloscopeWithDialog => {
                self.controllers
                    .open_dialog
                    .sender()
                    .send(dialogs::open_save::DialogInput::Show)
                    .unwrap();
            }
            AppInput::ColloscopeLoaded(path, data) => {
                if self.state != GlobalState::LoadingScreen {
                    return;
                }
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
            AppInput::ColloscopeLoadingFailed(path, error) => {
                self.controllers
                    .file_error
                    .sender()
                    .send(dialogs::file_error::DialogInput::Show(
                        dialogs::file_error::Type::Open,
                        path,
                        error,
                    ))
                    .unwrap();
                self.state = GlobalState::WelcomeScreen;
            }
            AppInput::WarnDirty => {
                self.controllers
                    .warn_dirty
                    .sender()
                    .send(dialogs::warning_changed::DialogInput::Show)
                    .unwrap();
            }
            AppInput::OkDirty => {
                let msg_opt = self.next_warn_msg.take();
                match msg_opt {
                    Some(msg) => sender.input(msg),
                    None => {}
                }
            }
            AppInput::RequestQuit => {
                self.send_but_check_dirty(sender, AppInput::Quit);
            }
            AppInput::Quit => {
                relm4::main_application().quit();
            }
            AppInput::RequestCloseFile => {
                if self.state == GlobalState::WelcomeScreen {
                    return;
                }
                self.send_but_check_dirty(sender, AppInput::CloseFile);
            }
            AppInput::CloseFile => {
                self.state = GlobalState::WelcomeScreen;
                self.controllers
                    .editor
                    .sender()
                    .send(editor::EditorInput::NewFile {
                        file_name: None,
                        data: collomatique_state_colloscopes::Data::new(),
                        dirty: false,
                    })
                    .unwrap();
                self.controllers
                    .loading
                    .sender()
                    .send(loading::LoadingInput::StopLoading)
                    .unwrap();
            }
            AppInput::RequestSave => {
                if self.state != GlobalState::EditorScreen {
                    return;
                }
                self.controllers
                    .editor
                    .sender()
                    .send(editor::EditorInput::SaveClicked)
                    .unwrap();
            }
            AppInput::RequestSaveAs => {
                if self.state != GlobalState::EditorScreen {
                    return;
                }
                self.controllers
                    .editor
                    .sender()
                    .send(editor::EditorInput::SaveAsClicked)
                    .unwrap();
            }
            AppInput::RequestUndo => {
                if self.state != GlobalState::EditorScreen {
                    return;
                }
                self.controllers
                    .editor
                    .sender()
                    .send(editor::EditorInput::UndoClicked)
                    .unwrap();
            }
            AppInput::RequestRedo => {
                if self.state != GlobalState::EditorScreen {
                    return;
                }
                self.controllers
                    .editor
                    .sender()
                    .send(editor::EditorInput::RedoClicked)
                    .unwrap();
            }
            AppInput::UpdateActions => {
                self.actions
                    .save_action
                    .set_enabled(self.controllers.editor.model().is_dirty());
                self.actions
                    .undo_action
                    .set_enabled(self.controllers.editor.model().can_undo());
                self.actions
                    .redo_action
                    .set_enabled(self.controllers.editor.model().can_redo());
            }
        }
    }
}
