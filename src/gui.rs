use anyhow::Result;

mod dialogs;
mod editor;
mod tools;
mod welcome;

use iced::{Element, Task};

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
}

fn update(state: &mut GuiState, message: GuiMessage) -> Task<GuiMessage> {
    match message {
        GuiMessage::EditorMessage(msg) => editor::update(state, msg),
        GuiMessage::WelcomeMessage(msg) => welcome::update(state, msg),
        GuiMessage::DialogMessage(msg) => dialogs::update(state, msg),
        GuiMessage::FileSelected(result) => match result {
            Some(file_desc) => {
                *state = GuiState::Editor(editor::State::new(file_desc.path));
                Task::none()
            }
            None => Task::none(),
        },
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

pub fn run_gui(create: bool, db: Option<std::path::PathBuf>) -> Result<()> {
    iced::application(title, update, view).run_with(move || {
        (
            match &db {
                Some(file) => GuiState::Editor(editor::State::new(file.clone())),
                None => GuiState::Welcome,
            },
            if create && db.is_none() {
                iced::Task::done(GuiMessage::DialogMessage(dialogs::Message::OpenNewFile))
            } else {
                iced::Task::none()
            },
        )
    })?;
    Ok(())
}
