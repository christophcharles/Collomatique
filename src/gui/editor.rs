use collomatique::json::json;
use collomatique::frontend::state::Manager;
use thiserror::Error;

use iced::widget::{button, center, column, container, row, text, tooltip, Space};
use iced::{window, Element, Length, Subscription, Task, Theme};

use super::{tools, GuiMessage, GuiState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Panel {
    SubjectGroups,
    Subjects,
    Teachers,
    Students,
}

#[derive(Debug, Clone)]
struct AppStateBox {
    state: std::sync::Arc<
        std::sync::RwLock<
            Option<collomatique::frontend::state::AppState<collomatique::json::json::JsonStore>>,
        >,
    >,
}

impl AppStateBox {
    fn read_lock(
        &self,
    ) -> std::sync::RwLockReadGuard<
        '_,
        Option<collomatique::frontend::state::AppState<collomatique::json::json::JsonStore>>,
    > {
        self.state.read().unwrap()
    }

    fn extract(
        &mut self,
    ) -> Option<collomatique::frontend::state::AppState<collomatique::json::json::JsonStore>>
    {
        let mut lock = self.state.write().unwrap();

        lock.take()
    }

    fn new(
        value: collomatique::frontend::state::AppState<collomatique::json::json::JsonStore>,
    ) -> Self {
        AppStateBox {
            state: std::sync::Arc::new(std::sync::RwLock::new(Some(value))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    panel: Panel,
    path: Option<std::path::PathBuf>,
    app_state: AppStateBox,
    init_state: collomatique::json::json::JsonStore,
}

#[derive(Debug, Error, Clone)]
pub enum OpenCollomatiqueFileError {
    #[error("Erreur à l'ouverture json : {0}")]
    JsonError(std::sync::Arc<collomatique::json::json::FromJsonError>),
    #[error("Erreur d'entrée/sortie : {0}")]
    IOError(std::sync::Arc<std::io::Error>),
}

impl From<collomatique::json::json::OpenError> for OpenCollomatiqueFileError {
    fn from(value: collomatique::json::json::OpenError) -> Self {
        use collomatique::json::json::OpenError;
        match value {
            OpenError::IO(error) => OpenCollomatiqueFileError::IOError(std::sync::Arc::new(error)),
            OpenError::FromJsonError(error) => {
                OpenCollomatiqueFileError::JsonError(std::sync::Arc::new(error))
            }
        }
    }
}

impl From<collomatique::json::json::FromJsonError> for OpenCollomatiqueFileError {
    fn from(value: collomatique::json::json::FromJsonError) -> Self {
        OpenCollomatiqueFileError::JsonError(std::sync::Arc::new(value))
    }
}

impl From<std::io::Error> for OpenCollomatiqueFileError {
    fn from(value: std::io::Error) -> Self {
        OpenCollomatiqueFileError::IOError(std::sync::Arc::new(value))
    }
}

pub type OpenCollomatiqueFileResult<T> = std::result::Result<T, OpenCollomatiqueFileError>;

impl State {
    pub fn new_with_empty_file() -> OpenCollomatiqueFileResult<Self> {
        use collomatique::json::Logic;
        use collomatique::frontend::state::AppState;

        let logic = Logic::new(json::JsonStore::new());
        let app_state = AppState::new(logic);
        let init_state = app_state.get_logic().get_storage().clone();

        Ok(Self {
            panel: Panel::SubjectGroups,
            path: None,
            app_state: AppStateBox::new(app_state),
            init_state,
        })
    }

    pub async fn new_with_existing_file(
        file: std::path::PathBuf,
    ) -> OpenCollomatiqueFileResult<Self> {
        use collomatique::json::Logic;
        use collomatique::frontend::state::AppState;

        let content = tokio::fs::read_to_string(&file).await?;

        let logic = Logic::new(json::JsonStore::from_json(&content)?);
        let app_state = AppState::new(logic);
        let init_state = app_state.get_logic().get_storage().clone();

        Ok(Self {
            panel: Panel::SubjectGroups,
            path: Some(file),
            app_state: AppStateBox::new(app_state),
            init_state,
        })
    }
}

impl State {
    fn is_modified(&self) -> bool {
        let app_state_lock = self.app_state.read_lock();
        let Some(app_state) = &*app_state_lock else {
            return true;
        };

        *app_state.get_logic().get_storage() != self.init_state
    }

    fn is_saved(&self) -> bool {
        if self.is_modified() {
            return false;
        }

        self.path.is_some()
    }
}

type UndoErr = collomatique::frontend::state::UndoError<
    <collomatique::json::json::JsonStore as collomatique::json::Storage>::InternalError,
>;
type RedoErr = collomatique::frontend::state::RedoError<
    <collomatique::json::json::JsonStore as collomatique::json::Storage>::InternalError,
>;

#[derive(Error, Debug, Clone)]
pub enum SaveToFileError {
    #[error("Erreur d'entrée sortie : {0:?}")]
    IOError(std::sync::Arc<std::io::Error>),
}

impl From<std::io::Error> for SaveToFileError {
    fn from(value: std::io::Error) -> Self {
        SaveToFileError::IOError(std::sync::Arc::new(value))
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    PanelChanged(Panel),
    NewClicked,
    OpenClicked,
    SaveClicked,
    SaveAsClicked,
    SaveToFile(std::path::PathBuf),
    SaveToFileProcessed(Result<std::path::PathBuf, SaveToFileError>),
    UndoClicked,
    UndoProcessed(std::sync::Arc<Result<(), UndoErr>>),
    RedoClicked,
    RedoProcessed(std::sync::Arc<Result<(), RedoErr>>),
    CloseClicked,
    ExitRequest(window::Id),
    ExitValidated(window::Id),
}

fn close_warning_task(editor_state: &State, message_if_yes: GuiMessage) -> Task<GuiMessage> {
    if editor_state.is_modified() {
        Task::done(
            super::dialogs::Message::YesNoAlertDialog(
                "Abandonner les modifications ?".into(),
                "Les modifications du colloscope actuelles seront abandonnées.".into(),
                std::sync::Arc::new(move |result| {
                    if result {
                        message_if_yes.clone()
                    } else {
                        GuiMessage::Ignore
                    }
                }),
            )
            .into(),
        )
    } else {
        Task::done(message_if_yes)
    }
}

pub fn update(state: &mut GuiState, message: Message) -> Task<GuiMessage> {
    let GuiState::Editor(editor_state) = state else {
        panic!("Editor message received but GUI not in an editor state");
    };

    match message {
        Message::PanelChanged(new_panel) => {
            editor_state.panel = new_panel;
            Task::none()
        }
        Message::NewClicked => close_warning_task(editor_state, GuiMessage::OpenNewFile),
        Message::OpenClicked => close_warning_task(editor_state, GuiMessage::OpenExistingFile),
        Message::SaveClicked => match &editor_state.path {
            Some(file) => Task::done(Message::SaveToFile(file.clone()).into()),
            None => Task::done(
                super::dialogs::Message::FileChooserDialog(
                    "Enregistrer le colloscope".into(),
                    true,
                    std::sync::Arc::new(|path| match path {
                        Some(file) => Message::SaveToFile(file).into(),
                        None => GuiMessage::Ignore,
                    }),
                )
                .into(),
            ),
        },
        Message::SaveAsClicked => Task::done(
            super::dialogs::Message::FileChooserDialog(
                "Enregistrer le colloscope sous...".into(),
                true,
                std::sync::Arc::new(|path| match path {
                    Some(file) => Message::SaveToFile(file).into(),
                    None => GuiMessage::Ignore,
                }),
            )
            .into(),
        ),
        Message::SaveToFile(path) => {
            let app_state_lock = editor_state.app_state.read_lock();
            let Some(app_state) = &*app_state_lock else {
                panic!("No state to save");
            };
            let json_result = app_state.get_logic().get_storage().to_json();

            match json_result {
                Ok(json_content) => Task::perform(
                    async move {
                        tokio::fs::write(&path, json_content).await?;
                        Ok(path)
                    },
                    |x| Message::SaveToFileProcessed(x).into(),
                ),
                Err(e) => Task::done(
                    super::dialogs::Message::ErrorDialog(
                        "Erreur à l'enregistrement du fichier".into(),
                        e.to_string(),
                    )
                    .into(),
                ),
            }
        }
        Message::SaveToFileProcessed(path_result) => match path_result {
            Ok(path) => {
                let app_state_lock = editor_state.app_state.read_lock();
                let Some(app_state) = &*app_state_lock else {
                    panic!("No state to commit as initial state after saving");
                };
                editor_state.path = Some(path);
                editor_state.init_state = app_state.get_logic().get_storage().clone();
                Task::none()
            }
            Err(e) => Task::done(
                super::dialogs::Message::ErrorDialog(
                    "Erreur à l'enregistrement du fichier".into(),
                    e.to_string(),
                )
                .into(),
            ),
        },
        Message::UndoClicked => {
            /*let GuiState::Editor(editor_state) = state else {
                panic!("Received editor message while not in editor state");
            };

            let app_state_ref = editor_state.app_state.clone();

            Task::perform(
                async move {
                    use collomatique::frontend::state::Manager;
                    let res = app_state_ref.write().unwrap().undo().await;
                    std::sync::Arc::new(res)
                },
                |x| Message::UndoProcessed(x).into()
            )*/
            Task::none()
        }
        Message::UndoProcessed(result) => {
            use collomatique::frontend::state::UndoError;
            match &*result {
                Ok(_) => Task::none(),
                Err(e) => match e {
                    UndoError::HistoryDepleted => panic!("History depleted for undo but it was still possible to click the undo button"),
                    UndoError::InternalError(int_err) => Task::done(
                        super::dialogs::Message::ErrorDialog("Erreur dans la base de donnée".into(), int_err.to_string()).into()
                    )
                }
            }
        }
        Message::RedoClicked => {
            /*let GuiState::Editor(editor_state) = state else {
                panic!("Received editor message while not in editor state");
            };

            let app_state_ref = editor_state.app_state.clone();
            Task::perform(
                async move {
                    use collomatique::frontend::state::Manager;
                    let lock = app_state_ref.write().expect("Should lock successfully");
                    let res = lock.redo().await;
                    std::sync::Arc::new(res)
                },
                |x| Message::RedoProcessed(x).into()
            )*/
            Task::none()
        }
        Message::RedoProcessed(result) => {
            use collomatique::frontend::state::RedoError;
            match &*result {
                Ok(_) => Task::none(),
                Err(e) => match e {
                    RedoError::HistoryFullyRewounded => panic!("History fully rewounded for redo but it was still possible to click the redo button"),
                    RedoError::InternalError(int_err) => Task::done(
                        super::dialogs::Message::ErrorDialog("Erreur dans la base de donnée".into(), int_err.to_string()).into()
                    )
                }
            }
        }
        Message::CloseClicked => close_warning_task(editor_state, GuiMessage::GoToWelcomeScreen),
        Message::ExitRequest(id) => {
            close_warning_task(editor_state, Message::ExitValidated(id).into())
        }
        Message::ExitValidated(id) => window::close(id),
    }
}

fn icon_button<'a>(
    ico: tools::Icon,
    style: impl Fn(&Theme, button::Status) -> button::Style + 'a,
    label: &'a str,
    message: Option<GuiMessage>,
) -> Element<'a, GuiMessage> {
    let btn = button(container(tools::icon(ico)).center_x(25).center_y(25))
        .style(style)
        .padding(2)
        .on_press_maybe(message);

    tooltip(btn, text(label).size(10), tooltip::Position::FollowCursor)
        .style(container::bordered_box)
        .into()
}

pub fn view(state: &State) -> Element<GuiMessage> {
    let app_state_lock = state.app_state.read_lock();
    let Some(app_state) = &*app_state_lock else {
        return center(text("Chargement...")).into();
    };

    row![
        column![
            icon_button(
                tools::Icon::New,
                button::primary,
                "Créer un nouveau colloscope",
                Some(Message::NewClicked.into())
            ),
            icon_button(
                tools::Icon::Open,
                button::primary,
                "Ouvrir un colloscope existant",
                Some(Message::OpenClicked.into())
            ),
            Space::with_height(2),
            icon_button(
                tools::Icon::Save,
                button::primary,
                "Enregistrer",
                if state.is_saved() {
                    None
                } else {
                    Some(Message::SaveClicked.into())
                }
            ),
            icon_button(
                tools::Icon::Download,
                button::primary,
                "Enregistrer sous",
                Some(Message::SaveAsClicked.into()),
            ),
            Space::with_height(20),
            icon_button(tools::Icon::Undo, button::primary, "Annuler", {
                use collomatique::frontend::state::Manager;
                if app_state.can_undo() {
                    Some(Message::UndoClicked.into())
                } else {
                    None
                }
            }),
            icon_button(tools::Icon::Redo, button::primary, "Rétablir", {
                use collomatique::frontend::state::Manager;
                if app_state.can_redo() {
                    Some(Message::RedoClicked.into())
                } else {
                    None
                }
            }),
            Space::with_height(Length::Fill),
            icon_button(
                tools::Icon::Close,
                button::danger,
                "Fermer le colloscope",
                Some(Message::CloseClicked.into())
            ),
        ]
        .spacing(2)
        .padding(0),
        container(
            column![
                button("Groupements")
                    .width(Length::Fill)
                    .style(if state.panel == Panel::SubjectGroups {
                        button::primary
                    } else {
                        button::text
                    })
                    .on_press(Message::PanelChanged(Panel::SubjectGroups).into()),
                button("Matières")
                    .width(Length::Fill)
                    .style(if state.panel == Panel::Subjects {
                        button::primary
                    } else {
                        button::text
                    })
                    .on_press(Message::PanelChanged(Panel::Subjects).into()),
                button("Enseignants")
                    .width(Length::Fill)
                    .style(if state.panel == Panel::Teachers {
                        button::primary
                    } else {
                        button::text
                    })
                    .on_press(Message::PanelChanged(Panel::Teachers).into()),
                button("Élèves")
                    .width(Length::Fill)
                    .style(if state.panel == Panel::Students {
                        button::primary
                    } else {
                        button::text
                    })
                    .on_press(Message::PanelChanged(Panel::Students).into()),
            ]
            .width(Length::Fill)
            .spacing(2)
        )
        .padding(5)
        .height(Length::Fill)
        .center_x(200)
        .style(iced::widget::container::bordered_box),
        center(text(match state.panel {
            Panel::SubjectGroups => "Panneau groupements",
            Panel::Subjects => "Panneau matières",
            Panel::Teachers => "Panneau enseignants",
            Panel::Students => "Panneau élèves",
        }))
        .padding(5)
        .style(container::bordered_box)
    ]
    .spacing(5)
    .padding(5)
    .into()
}

pub fn title(state: &State) -> String {
    let main_title = if state.is_saved() {
        "Collomatique"
    } else {
        "Collomatique*"
    };
    match &state.path {
        Some(p) => format!("{} - {}", main_title, p.to_string_lossy()),
        None => format!("{}", main_title),
    }
}

pub fn exit_subscription(_state: &State) -> Subscription<GuiMessage> {
    iced::window::close_requests().map(|id| Message::ExitRequest(id).into())
}
