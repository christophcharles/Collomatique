use anyhow::Result;

use iced::widget::{button, column, container, row, text};
use iced::Element;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Panel {
    SubjectGroups,
    Subjects,
    Teachers,
    Students,
}

struct GuiState {
    panel: Panel,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            panel: Panel::SubjectGroups,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    PanelChanged(Panel),
}

fn update(state: &mut GuiState, message: Message) {
    match message {
        Message::PanelChanged(new_panel) => {
            state.panel = new_panel;
        }
    }
}

fn view(state: &GuiState) -> Element<Message> {
    use iced::Length;

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
                            .on_press(Message::PanelChanged(Panel::SubjectGroups)),
                        button("Matières")
                            .width(Length::Fill)
                            .style(if state.panel == Panel::Subjects {
                                button::primary
                            } else {
                                button::text
                            })
                            .on_press(Message::PanelChanged(Panel::Subjects)),
                        button("Enseignants")
                            .width(Length::Fill)
                            .style(if state.panel == Panel::Teachers {
                                button::primary
                            } else {
                                button::text
                            })
                            .on_press(Message::PanelChanged(Panel::Teachers)),
                        button("Élèves")
                            .width(Length::Fill)
                            .style(if state.panel == Panel::Students {
                                button::primary
                            } else {
                                button::text
                            })
                            .on_press(Message::PanelChanged(Panel::Students)),
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

pub fn run_gui(_create: bool, _db: Option<std::path::PathBuf>) -> Result<()> {
    iced::run("Collomatique", update, view)?;
    Ok(())
}
