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

pub fn run_gui(create: bool, db: Option<std::path::PathBuf>) -> Result<()> {
    iced::application("Collomatique", update, view).run_with(move || {
        (
            match db {
                Some(file) => GuiState::Editor(editor::State::new(file)),
                None => {
                    if create {
                        GuiState::Editor(editor::State::default())
                    } else {
                        GuiState::Welcome
                    }
                }
            },
            iced::Task::none(),
        )
    })?;
    Ok(())
}
