use iced::widget::{button, center, column, container};
use iced::{window, Element, Length, Subscription, Task};

use super::{GuiMessage, GuiState};

#[derive(Debug, Clone)]
pub enum Message {
    NewClicked,
    OpenClicked,
    ExitRequest(window::Id),
}

pub fn update(_state: &mut GuiState, message: Message) -> Task<GuiMessage> {
    match message {
        Message::NewClicked => Task::done(GuiMessage::OpenNewFile),
        Message::OpenClicked => Task::done(GuiMessage::OpenExistingFile),
        Message::ExitRequest(id) => window::close(id),
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

pub fn exit_subscription() -> Subscription<GuiMessage> {
    iced::window::close_requests().map(|id| Message::ExitRequest(id).into())
}

pub fn title() -> String {
    "Collomatique".into()
}
