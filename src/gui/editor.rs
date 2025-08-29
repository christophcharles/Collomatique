use iced::widget::{button, center, column, container, row, text, tooltip, Space};
use iced::{Element, Length, Task, Theme};

use super::{tools, GuiMessage, GuiState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Panel {
    SubjectGroups,
    Subjects,
    Teachers,
    Students,
}

#[derive(Debug, Clone)]
pub struct State {
    panel: Panel,
    db: Option<std::path::PathBuf>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            panel: Panel::SubjectGroups,
            db: None,
        }
    }
}

impl State {
    pub fn new(file: std::path::PathBuf) -> Self {
        Self {
            panel: Panel::SubjectGroups,
            db: Some(file),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    PanelChanged(Panel),
    CloseClicked,
}

pub fn update(state: &mut GuiState, message: Message) -> Task<GuiMessage> {
    let GuiState::Editor(editor_state) = state else {
        panic!("Editor message received but GUI not in an editor state");
    };

    match message {
        Message::PanelChanged(new_panel) => {
            editor_state.panel = new_panel;
            Task::none()
        }
        Message::CloseClicked => Task::done(
            super::dialogs::Message::AlertDialog(
                "Attention".into(),
                "Fermer le colloscope ?".into(),
                |result| {
                    if result {
                        GuiMessage::GoToWelcomeScreen
                    } else {
                        GuiMessage::Ignore
                    }
                },
            )
            .into(),
        ),
    }
}

fn icon_button<'a>(
    ico: tools::Icon,
    style: impl Fn(&Theme, button::Status) -> button::Style + 'a,
    label: &'a str,
    message: Option<GuiMessage>,
) -> Element<'a, GuiMessage> {
    let btn = button(container(tools::icon(ico)).center_x(20))
        .style(style)
        .padding(2);
    let btn = match message {
        Some(msg) => btn.on_press(msg),
        None => btn,
    };

    tooltip(btn, text(label).size(10), tooltip::Position::FollowCursor)
        .style(container::bordered_box)
        .into()
}

pub fn view(state: &State) -> Element<GuiMessage> {
    column![
        row![
            icon_button(
                tools::Icon::New,
                button::primary,
                "Créer un nouveau colloscope",
                None
            ),
            icon_button(
                tools::Icon::Open,
                button::primary,
                "Ouvrir un colloscope existant",
                None
            ),
            Space::with_width(2),
            icon_button(
                tools::Icon::SaveAs,
                button::primary,
                "Enregistrer sous",
                None
            ),
            Space::with_width(20),
            icon_button(tools::Icon::Undo, button::primary, "Annuler", None),
            icon_button(tools::Icon::Redo, button::primary, "Rétablir", None),
            Space::with_width(Length::Fill),
            icon_button(
                tools::Icon::Close,
                button::danger,
                "Fermer le colloscope",
                Some(Message::CloseClicked.into())
            ),
        ]
        .spacing(2)
        .padding(0),
        row![
            container(
                column![
                    button("Groupements")
                        .width(Length::Fill)
                        .style(if state.panel == Panel::SubjectGroups {
                            button::primary
                        } else {
                            button::text
                        })
                        .on_press(Message::PanelChanged(Panel::SubjectGroups).into()),
                    button("Matières")
                        .width(Length::Fill)
                        .style(if state.panel == Panel::Subjects {
                            button::primary
                        } else {
                            button::text
                        })
                        .on_press(Message::PanelChanged(Panel::Subjects).into()),
                    button("Enseignants")
                        .width(Length::Fill)
                        .style(if state.panel == Panel::Teachers {
                            button::primary
                        } else {
                            button::text
                        })
                        .on_press(Message::PanelChanged(Panel::Teachers).into()),
                    button("Élèves")
                        .width(Length::Fill)
                        .style(if state.panel == Panel::Students {
                            button::primary
                        } else {
                            button::text
                        })
                        .on_press(Message::PanelChanged(Panel::Students).into()),
                ]
                .width(Length::Fill)
                .spacing(2)
            )
            .padding(5)
            .height(Length::Fill)
            .center_x(200)
            .style(iced::widget::container::bordered_box),
            center(text(match state.panel {
                Panel::SubjectGroups => "Panneau groupements",
                Panel::Subjects => "Panneau matières",
                Panel::Teachers => "Panneau enseignants",
                Panel::Students => "Panneau élèves",
            }))
            .padding(5)
            .style(container::bordered_box)
        ]
        .spacing(5)
        .padding(0),
    ]
    .spacing(5)
    .padding(5)
    .into()
}

pub fn title(state: &State) -> String {
    match &state.db {
        Some(file) => {
            format!("Collomatique - {}", file.to_string_lossy())
        }
        None => String::from("Collomatique"),
    }
}
