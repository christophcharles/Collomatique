use iced::widget::{button, center, column, container, row, text, tooltip, Space};
use iced::{window, Element, Length, Subscription, Task, Theme};

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
    NewClicked,
    OpenClicked,
    CloseClicked,
    ExitRequest(window::Id),
    ExitValidated(window::Id),
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
        Message::NewClicked => Task::done(
            super::dialogs::Message::AlertDialog(
                "Créer un nouveau colloscope ?".into(),
                "Le colloscope actuel sera fermé.".into(),
                std::sync::Arc::new(|result| {
                    if result {
                        GuiMessage::OpenNewFile
                    } else {
                        GuiMessage::Ignore
                    }
                }),
            )
            .into(),
        ),
        Message::OpenClicked => Task::done(
            super::dialogs::Message::AlertDialog(
                "Ouvrir un colloscope ?".into(),
                "Le colloscope actuel sera fermé.".into(),
                std::sync::Arc::new(|result| {
                    if result {
                        GuiMessage::OpenExistingFile
                    } else {
                        GuiMessage::Ignore
                    }
                }),
            )
            .into(),
        ),
        Message::CloseClicked => Task::done(
            super::dialogs::Message::AlertDialog(
                "Fermer le colloscope ?".into(),
                "Les modifications sont enregistrées.".into(),
                std::sync::Arc::new(|result| {
                    if result {
                        GuiMessage::GoToWelcomeScreen
                    } else {
                        GuiMessage::Ignore
                    }
                }),
            )
            .into(),
        ),
        Message::ExitRequest(id) => Task::done(
            super::dialogs::Message::AlertDialog(
                "Quitter Collomatique ?".into(),
                "Les modifications sont enregistrées.".into(),
                std::sync::Arc::new(move |result| {
                    if result {
                        Message::ExitValidated(id).into()
                    } else {
                        GuiMessage::Ignore
                    }
                }),
            )
            .into(),
        ),
        Message::ExitValidated(id) => window::close(id),
    }
}

fn icon_button<'a>(
    ico: tools::Icon,
    style: impl Fn(&Theme, button::Status) -> button::Style + 'a,
    label: &'a str,
    message: Option<GuiMessage>,
) -> Element<'a, GuiMessage> {
    let btn = button(container(tools::icon(ico)).center_x(25).center_y(25))
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
    row![
        column![
            icon_button(
                tools::Icon::New,
                button::primary,
                "Créer un nouveau colloscope",
                Some(Message::NewClicked.into())
            ),
            icon_button(
                tools::Icon::Open,
                button::primary,
                "Ouvrir un colloscope existant",
                Some(Message::OpenClicked.into())
            ),
            Space::with_height(2),
            icon_button(
                tools::Icon::SaveAs,
                button::primary,
                "Enregistrer sous",
                None
            ),
            Space::with_height(20),
            icon_button(tools::Icon::Undo, button::primary, "Annuler", None),
            icon_button(tools::Icon::Redo, button::primary, "Rétablir", None),
            Space::with_height(Length::Fill),
            icon_button(
                tools::Icon::Close,
                button::danger,
                "Fermer le colloscope",
                Some(Message::CloseClicked.into())
            ),
        ]
        .spacing(2)
        .padding(0),
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

pub fn exit_subscription(_state: &State) -> Subscription<GuiMessage> {
    iced::window::close_requests().map(|id| Message::ExitRequest(id).into())
}
