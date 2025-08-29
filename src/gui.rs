use anyhow::Result;

use iced::widget::{button, column, container, text};
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
    use iced::Length;

    column![
        container(button("Ouvrir"))
            .padding(5)
            .width(Length::Fill)
            .center_y(Length::Shrink)
            .style(iced::widget::container::rounded_box),
        container(text(&state.txt))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
    ]
    .spacing(5)
    .into()
}

pub fn run_gui(_create: bool, _db: Option<std::path::PathBuf>) -> Result<()> {
    iced::run("Collomatique", update, view)?;
    Ok(())
}
