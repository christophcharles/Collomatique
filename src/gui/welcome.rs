use iced::widget::{button, center, column, container};
use iced::{Element, Length, Task};

use super::{GuiMessage, GuiState};

#[derive(Debug, Clone)]
pub enum Message {
    NewClicked,
    OpenClicked,
}

pub fn update(_state: &mut GuiState, message: Message) -> Task<GuiMessage> {
    match message {
        Message::NewClicked => Task::done(super::dialogs::Message::OpenNewFile.into()),
        Message::OpenClicked => Task::done(super::dialogs::Message::OpenExistingFile.into()),
    }
}

pub fn view<'a>() -> Element<'a, GuiMessage> {
    center(
        column![
            button(container("Créer un nouveau colloscope").center_x(Length::Fill))
                .width(Length::Fill)
                .on_press(Message::NewClicked.into()),
            button(container("Ouvrir un colloscope existant").center_x(Length::Fill))
                .width(Length::Fill)
                .on_press(Message::OpenClicked.into()),
        ]
        .width(400)
        .spacing(2),
    )
    .into()
}

pub fn title() -> String {
    "Collomatique".into()
}
