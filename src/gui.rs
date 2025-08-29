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
    FileSelected(Option<dialogs::FileDesc>),
    FileLoaded(editor::ConnectDbResult<editor::State>),
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
            Some(file_desc) => Task::perform(
                editor::State::new(file_desc.create, file_desc.path),
                GuiMessage::FileLoaded,
            ),
            None => Task::none(),
        },
        GuiMessage::FileLoaded(result) => match result {
            Ok(editor_state) => {
                *state = GuiState::Editor(editor_state);
                Task::none()
            }
            Err(_e) => Task::none(),
        },
        GuiMessage::OpenExistingFile => Task::done(
            dialogs::Message::FileChooserDialog(
                "Ouvrir un colloscope".into(),
                false,
                std::sync::Arc::new(|file_desc| GuiMessage::FileSelected(file_desc)),
            )
            .into(),
        ),
        GuiMessage::OpenNewFile => Task::done(
            dialogs::Message::FileChooserDialog(
                "Créer un colloscope".into(),
                true,
                std::sync::Arc::new(|file_desc| GuiMessage::FileSelected(file_desc)),
            )
            .into(),
        ),
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

pub fn run_gui(create: bool, db: Option<std::path::PathBuf>) -> Result<()> {
    iced::application(title, update, view)
        .font(include_bytes!("../fonts/collomatique-icons.ttf").as_slice())
        .exit_on_close_request(false)
        .subscription(exit_subscription)
        .run_with(move || {
            (
                GuiState::Welcome,
                if let Some(file) = &db {
                    iced::Task::done(GuiMessage::FileSelected(Some(dialogs::FileDesc {
                        path: file.clone(),
                        create,
                    })))
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
