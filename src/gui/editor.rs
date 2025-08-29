use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

use super::{GuiMessage, GuiState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Panel {
    SubjectGroups,
    Subjects,
    Teachers,
    Students,
}

pub struct State {
    panel: Panel,
}

impl Default for State {
    fn default() -> Self {
        Self {
            panel: Panel::SubjectGroups,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    PanelChanged(Panel),
}

pub fn update(state: &mut GuiState, message: Message) {
    let GuiState::Editor(editor_state) = state else {
        return;
    };

    match message {
        Message::PanelChanged(new_panel) => {
            editor_state.panel = new_panel;
        }
    }
}

pub fn view(state: &State) -> Element<GuiMessage> {
    container(
        row![
            column![
                container(
                    column![
                        button("Groupements")
                            .width(Length::Fill)
                            .style(if state.panel == Panel::SubjectGroups {
                                button::primary
                            } else {
                                button::text
                            })
                            .on_press(GuiMessage::EditorMessage(Message::PanelChanged(
                                Panel::SubjectGroups
                            ))),
                        button("Matières")
                            .width(Length::Fill)
                            .style(if state.panel == Panel::Subjects {
                                button::primary
                            } else {
                                button::text
                            })
                            .on_press(GuiMessage::EditorMessage(Message::PanelChanged(
                                Panel::Subjects
                            ))),
                        button("Enseignants")
                            .width(Length::Fill)
                            .style(if state.panel == Panel::Teachers {
                                button::primary
                            } else {
                                button::text
                            })
                            .on_press(GuiMessage::EditorMessage(Message::PanelChanged(
                                Panel::Teachers
                            ))),
                        button("Élèves")
                            .width(Length::Fill)
                            .style(if state.panel == Panel::Students {
                                button::primary
                            } else {
                                button::text
                            })
                            .on_press(GuiMessage::EditorMessage(Message::PanelChanged(
                                Panel::Students
                            ))),
                    ]
                    .width(Length::Fill)
                    .spacing(2)
                )
                .padding(5)
                .height(Length::Fill)
                .center_x(Length::Shrink)
                .style(iced::widget::container::rounded_box),
                button(container(text("Menu")).center_x(Length::Fill)).width(Length::Fill),
            ]
            .width(200)
            .spacing(2),
            container(text(match state.panel {
                Panel::SubjectGroups => "Panneau groupements",
                Panel::Subjects => "Panneau matières",
                Panel::Teachers => "Panneau enseignants",
                Panel::Students => "Panneau élèves",
            }))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
        ]
        .spacing(5),
    )
    .padding(5)
    .into()
}
