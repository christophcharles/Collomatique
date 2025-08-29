use iced::widget::{button, center, column, container};
use iced::{Element, Length};

use super::{GuiMessage, GuiState};

#[derive(Debug, Clone)]
pub enum Message {
    NewClicked,
    OpenClicked,
}

pub fn update(state: &mut GuiState, message: Message) {
    match message {
        Message::NewClicked => {
            *state = GuiState::Editor(super::editor::State::default());
        }
        Message::OpenClicked => {}
    }
}

pub fn view<'a>() -> Element<'a, GuiMessage> {
    center(
        column![
            button(container("Créer un nouveau colloscope").center_x(Length::Fill))
                .width(Length::Fill)
                .on_press(GuiMessage::WelcomeMessage(Message::NewClicked)),
            button(container("Ouvrir un colloscope existant").center_x(Length::Fill))
                .width(Length::Fill)
                .on_press(GuiMessage::WelcomeMessage(Message::OpenClicked)),
        ]
        .width(400)
        .spacing(2),
    )
    .into()
}
