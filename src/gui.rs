use anyhow::Result;

mod editor;
mod welcome;

use iced::Element;

#[derive(Default)]
enum GuiState {
    #[default]
    Welcome,
    Editor(editor::State),
}

#[derive(Debug, Clone)]
enum GuiMessage {
    WelcomeMessage(welcome::Message),
    EditorMessage(editor::Message),
}

fn update(state: &mut GuiState, message: GuiMessage) {
    match message {
        GuiMessage::EditorMessage(msg) => editor::update(state, msg),
        GuiMessage::WelcomeMessage(msg) => welcome::update(state, msg),
    }
}

fn view(state: &GuiState) -> Element<GuiMessage> {
    match state {
        GuiState::Welcome => welcome::view(),
        GuiState::Editor(editor_state) => editor::view(editor_state),
    }
}

pub fn run_gui(_create: bool, _db: Option<std::path::PathBuf>) -> Result<()> {
    iced::run("Collomatique", update, view)?;
    Ok(())
}
