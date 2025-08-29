use anyhow::Result;

use iced::widget::text;
use iced::Element;

struct GuiState {
    txt: String,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            txt: String::from("Placeholder"),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {}

fn update(_state: &mut GuiState, _message: Message) {}

fn view(state: &GuiState) -> Element<Message> {
    text(&state.txt).into()
}

pub fn run_gui(_create: bool, _db: Option<std::path::PathBuf>) -> Result<()> {
    iced::run("Collomatique", update, view)?;
    Ok(())
}
