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
pub struct State {
    panel: Panel,
    db: std::path::PathBuf,
    app_state: std::sync::Arc<
        collomatique::frontend::state::AppState<collomatique::backend::sqlite::Store>,
    >,
}

#[derive(Debug, Error, Clone)]
pub enum ConnectDbError {
    #[error("Nom de fichier invalide (Chaine UTF-8 invalide)")]
    InvalidPath,
    #[error("Le fichier {0} existe déjà et ne sera pas écrasé")]
    DatabaseAlreadyExists(std::path::PathBuf),
    #[error("Le fichier {0} n'existe pas")]
    DatabaseDoesNotExist(std::path::PathBuf),
    #[error("Erreur sqlx : {0}")]
    SqlxError(std::sync::Arc<sqlx::Error>),
    #[error("Erreur d'entrée/sortie : {0}")]
    IOError(std::sync::Arc<std::io::Error>),
}

impl From<collomatique::backend::sqlite::NewError> for ConnectDbError {
    fn from(value: collomatique::backend::sqlite::NewError) -> Self {
        use collomatique::backend::sqlite::NewError;
        match value {
            NewError::InvalidPath => ConnectDbError::InvalidPath,
            NewError::DatabaseAlreadyExists(path) => ConnectDbError::DatabaseAlreadyExists(path),
            NewError::SqlxError(sqlx_error) => sqlx_error.into(),
        }
    }
}

impl From<collomatique::backend::sqlite::OpenError> for ConnectDbError {
    fn from(value: collomatique::backend::sqlite::OpenError) -> Self {
        use collomatique::backend::sqlite::OpenError;
        match value {
            OpenError::InvalidPath => ConnectDbError::InvalidPath,
            OpenError::DatabaseDoesNotExist(path) => ConnectDbError::DatabaseDoesNotExist(path),
            OpenError::SqlxError(sqlx_error) => sqlx_error.into(),
        }
    }
}

impl From<sqlx::Error> for ConnectDbError {
    fn from(value: sqlx::Error) -> Self {
        ConnectDbError::SqlxError(std::sync::Arc::new(value))
    }
}

impl From<std::io::Error> for ConnectDbError {
    fn from(value: std::io::Error) -> Self {
        ConnectDbError::IOError(std::sync::Arc::new(value))
    }
}

pub type ConnectDbResult<T> = std::result::Result<T, ConnectDbError>;

#[derive(Debug, Clone)]
pub enum CreatePolicy {
    Create,
    CreateAndOverride,
    Open,
}

async fn connect_db(
    create_policy: CreatePolicy,
    path: &std::path::Path,
) -> ConnectDbResult<collomatique::backend::sqlite::Store> {
    use collomatique::backend::sqlite;
    match create_policy {
        CreatePolicy::Create => Ok(sqlite::Store::new_db(path).await?),
        CreatePolicy::Open => Ok(sqlite::Store::open_db(path).await?),
        CreatePolicy::CreateAndOverride => {
            use tokio::fs;
            if fs::try_exists(path).await? {
                fs::remove_file(path).await?;
            }
            Ok(sqlite::Store::new_db(path).await?)
        }
    }
}

impl State {
    pub async fn new(
        create_policy: CreatePolicy,
        file: std::path::PathBuf,
    ) -> ConnectDbResult<Self> {
        use collomatique::backend::Logic;
        use collomatique::frontend::state::AppState;

        let logic = Logic::new(connect_db(create_policy, file.as_path()).await?);
        let app_state = AppState::new(logic);

        Ok(Self {
            panel: Panel::SubjectGroups,
            db: file,
            app_state: std::sync::Arc::new(app_state),
        })
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    PanelChanged(Panel),
    NewClicked,
    OpenClicked,
    CloseClicked,
    ExitRequest(window::Id),
    ExitValidated(window::Id),
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
        Message::NewClicked => Task::done(
            super::dialogs::Message::YesNoAlertDialog(
                "Créer un nouveau colloscope ?".into(),
                "Le colloscope actuel sera fermé.".into(),
                std::sync::Arc::new(|result| {
                    if result {
                        GuiMessage::OpenNewFile
                    } else {
                        GuiMessage::Ignore
                    }
                }),
            )
            .into(),
        ),
        Message::OpenClicked => Task::done(
            super::dialogs::Message::YesNoAlertDialog(
                "Ouvrir un colloscope ?".into(),
                "Le colloscope actuel sera fermé.".into(),
                std::sync::Arc::new(|result| {
                    if result {
                        GuiMessage::OpenExistingFile
                    } else {
                        GuiMessage::Ignore
                    }
                }),
            )
            .into(),
        ),
        Message::CloseClicked => Task::done(
            super::dialogs::Message::YesNoAlertDialog(
                "Fermer le colloscope ?".into(),
                "Les modifications sont enregistrées.".into(),
                std::sync::Arc::new(|result| {
                    if result {
                        GuiMessage::GoToWelcomeScreen
                    } else {
                        GuiMessage::Ignore
                    }
                }),
            )
            .into(),
        ),
        Message::ExitRequest(id) => Task::done(
            super::dialogs::Message::YesNoAlertDialog(
                "Quitter Collomatique ?".into(),
                "Les modifications sont enregistrées.".into(),
                std::sync::Arc::new(move |result| {
                    if result {
                        Message::ExitValidated(id).into()
                    } else {
                        GuiMessage::Ignore
                    }
                }),
            )
            .into(),
        ),
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
        .padding(2);
    let btn = match message {
        Some(msg) => btn.on_press(msg),
        None => btn,
    };

    tooltip(btn, text(label).size(10), tooltip::Position::FollowCursor)
        .style(container::bordered_box)
        .into()
}

pub fn view(state: &State) -> Element<GuiMessage> {
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
                tools::Icon::SaveAs,
                button::primary,
                "Enregistrer sous",
                None
            ),
            Space::with_height(20),
            icon_button(tools::Icon::Undo, button::primary, "Annuler", None),
            icon_button(tools::Icon::Redo, button::primary, "Rétablir", None),
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
    format!("Collomatique - {}", state.db.to_string_lossy())
}

pub fn exit_subscription(_state: &State) -> Subscription<GuiMessage> {
    iced::window::close_requests().map(|id| Message::ExitRequest(id).into())
}
