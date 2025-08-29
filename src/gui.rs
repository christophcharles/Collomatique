use anyhow::Result;

mod dialogs;
mod editor;
mod tools;
mod welcome;

use iced::{Element, Subscription, Task};

#[derive(Default, Debug, Clone)]
enum GuiState {
    #[default]
    Welcome,
    Editor(editor::State),
    DialogShown(dialogs::State),
}

#[derive(Debug, Clone)]
enum GuiMessage {
    WelcomeMessage(welcome::Message),
    EditorMessage(editor::Message),
    DialogMessage(dialogs::Message),
    FileSelected(Option<std::path::PathBuf>),
    FileLoaded(editor::OpenCollomatiqueFileResult<editor::State>),
    OpenExistingFile,
    OpenNewFile,
    GoToWelcomeScreen,
    Ignore,
}

impl From<welcome::Message> for GuiMessage {
    fn from(value: welcome::Message) -> Self {
        GuiMessage::WelcomeMessage(value)
    }
}

impl From<editor::Message> for GuiMessage {
    fn from(value: editor::Message) -> Self {
        GuiMessage::EditorMessage(value)
    }
}

impl From<dialogs::Message> for GuiMessage {
    fn from(value: dialogs::Message) -> Self {
        GuiMessage::DialogMessage(value)
    }
}

fn update(state: &mut GuiState, message: GuiMessage) -> Task<GuiMessage> {
    match message {
        GuiMessage::EditorMessage(msg) => editor::update(state, msg),
        GuiMessage::WelcomeMessage(msg) => welcome::update(state, msg),
        GuiMessage::DialogMessage(msg) => dialogs::update(state, msg),
        GuiMessage::FileSelected(result) => match result {
            Some(file_path) => Task::perform(
                editor::State::new_with_existing_file(file_path),
                GuiMessage::FileLoaded,
            ),
            None => Task::none(),
        },
        GuiMessage::FileLoaded(result) => match result {
            Ok(editor_state) => {
                *state = GuiState::Editor(editor_state);
                Task::none()
            }
            Err(e) => Task::done(
                dialogs::Message::ErrorDialog(
                    "Erreur à l'ouverture du fichier".into(),
                    e.to_string(),
                )
                .into(),
            ),
        },
        GuiMessage::OpenExistingFile => Task::done(
            dialogs::Message::FileChooserDialog(
                "Ouvrir un colloscope".into(),
                false,
                std::sync::Arc::new(|path| GuiMessage::FileSelected(path)),
            )
            .into(),
        ),
        GuiMessage::OpenNewFile => {
            Task::done(GuiMessage::FileLoaded(editor::State::new_with_empty_file()))
        }
        GuiMessage::GoToWelcomeScreen => {
            *state = GuiState::Welcome;
            Task::none()
        }
        GuiMessage::Ignore => Task::none(),
    }
}

fn view(state: &GuiState) -> Element<GuiMessage> {
    match state {
        GuiState::Welcome => welcome::view(),
        GuiState::Editor(editor_state) => editor::view(editor_state),
        GuiState::DialogShown(dialog_state) => dialogs::view(dialog_state),
    }
}

fn title(state: &GuiState) -> String {
    match state {
        GuiState::Welcome => welcome::title(),
        GuiState::Editor(editor_state) => editor::title(editor_state),
        GuiState::DialogShown(dialog_state) => dialogs::title(dialog_state),
    }
}

fn exit_subscription(state: &GuiState) -> Subscription<GuiMessage> {
    match state {
        GuiState::Welcome => welcome::exit_subscription(),
        GuiState::Editor(editor_state) => editor::exit_subscription(editor_state),
        GuiState::DialogShown(dialog_state) => dialogs::exit_subscription(dialog_state),
    }
}

pub fn run_gui(create: bool, path: Option<std::path::PathBuf>) -> Result<()> {
    iced::application(title, update, view)
        .font(include_bytes!("../fonts/collomatique-icons.ttf").as_slice())
        .exit_on_close_request(false)
        .subscription(exit_subscription)
        .run_with(move || {
            (
                GuiState::Welcome,
                if let Some(file) = &path {
                    if create {
                        match file.try_exists() {
                            Ok(exists) => {
                                if exists {
                                    Task::done(
                                        dialogs::Message::ErrorDialog(
                                            "Erreur à la création du fichier".into(),
                                            format!(
                                                "Le fichier {} existe déjà.",
                                                file.to_string_lossy()
                                            ),
                                        )
                                        .into(),
                                    )
                                } else {
                                    match collomatique::backend::json::JsonStore::new()
                                        .to_json_file(file)
                                    {
                                        Ok(_) => iced::Task::done(GuiMessage::FileSelected(Some(
                                            file.clone(),
                                        ))),
                                        Err(e) => Task::done(
                                            dialogs::Message::ErrorDialog(
                                                "Erreur à la création du fichier".into(),
                                                e.to_string(),
                                            )
                                            .into(),
                                        ),
                                    }
                                }
                            }
                            Err(e) => Task::done(
                                dialogs::Message::ErrorDialog(
                                    "Erreur en testant l'existence du fichier".into(),
                                    e.to_string(),
                                )
                                .into(),
                            ),
                        }
                    } else {
                        iced::Task::done(GuiMessage::FileSelected(Some(file.clone())))
                    }
                } else {
                    if create {
                        iced::Task::done(GuiMessage::OpenNewFile)
                    } else {
                        iced::Task::none()
                    }
                },
            )
        })?;
    Ok(())
}
