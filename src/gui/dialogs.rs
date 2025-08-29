use iced::widget::{button, column, container, row, text, Space};
use iced::{Element, Length, Subscription, Task};

use super::{tools, GuiMessage, GuiState};

#[derive(Debug, Clone)]
pub struct FileDesc {
    pub path: std::path::PathBuf,
    pub create: bool,
}

#[derive(Clone)]
pub enum Message {
    OpenNewFile,
    OpenExistingFile,
    FileSelected(Option<FileDesc>),
    AlertDialog(
        String,
        String,
        std::sync::Arc<dyn Fn(bool) -> GuiMessage + Send + Sync>,
    ),
    AlertDialogClosed(bool),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::OpenNewFile => write!(f, "Message::OpenNewFile"),
            Message::OpenExistingFile => write!(f, "Message::OpenExistingFile"),
            Message::FileSelected(file_desc) => write!(f, "Message::FileSelected({:?}", file_desc),
            Message::AlertDialog(title, txt, _msg) => {
                write!(f, "Message::AlertDialog({:?}, {:?}, Fn)", title, txt)
            }
            Message::AlertDialogClosed(result) => {
                write!(f, "Message::AlertDialogClose({:?})", result)
            }
        }
    }
}

#[derive(Clone)]
pub enum DialogShown {
    FileChooser,
    Alert(
        String,
        String,
        std::sync::Arc<dyn Fn(bool) -> GuiMessage + Send + Sync>,
    ),
}

impl std::fmt::Debug for DialogShown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DialogShown::FileChooser => write!(f, "DialogShown::FileChooser"),
            DialogShown::Alert(title, txt, _msg) => {
                write!(f, "DialogShown::Alert({:?}, {:?}, Fn)", title, txt)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    previous_state: Box<GuiState>,
    dialog_shown: DialogShown,
}

pub fn update(state: &mut GuiState, message: Message) -> Task<GuiMessage> {
    match message {
        Message::OpenNewFile => {
            *state = GuiState::DialogShown(State {
                previous_state: Box::new(state.clone()),
                dialog_shown: DialogShown::FileChooser,
            });
            Task::perform(open_file_dialog(true), |x| Message::FileSelected(x).into())
        }
        Message::OpenExistingFile => {
            *state = GuiState::DialogShown(State {
                previous_state: Box::new(state.clone()),
                dialog_shown: DialogShown::FileChooser,
            });
            Task::perform(open_file_dialog(false), |x| Message::FileSelected(x).into())
        }
        Message::FileSelected(file_desc) => {
            let GuiState::DialogShown(dialog_state) = state else {
                panic!("Dialog message but not in dialog state");
            };

            *state = dialog_state.previous_state.as_ref().clone();
            Task::done(GuiMessage::FileSelected(file_desc))
        }
        Message::AlertDialog(title, txt, msg) => {
            *state = GuiState::DialogShown(State {
                previous_state: Box::new(state.clone()),
                dialog_shown: DialogShown::Alert(title, txt, msg),
            });
            Task::none()
        }
        Message::AlertDialogClosed(result) => {
            let GuiState::DialogShown(dialog_state) = state else {
                panic!("Dialog message but not in dialog state");
            };
            let DialogShown::Alert(_title, _txt, msg) = dialog_state.dialog_shown.clone() else {
                panic!("Alert dialog message but not in alert dialog state");
            };

            *state = dialog_state.previous_state.as_ref().clone();
            Task::done(msg(result))
        }
    }
}

async fn open_file_dialog(create: bool) -> Option<FileDesc> {
    let dialog = rfd::AsyncFileDialog::new();

    let file = if create {
        dialog.set_title("Créer un fichier").save_file().await
    } else {
        dialog.set_title("Ouvrir un fichier").pick_file().await
    };

    file.map(|handle| FileDesc {
        path: handle.path().to_owned(),
        create,
    })
}

pub fn view(state: &State) -> Element<GuiMessage> {
    let normal_view = super::view(state.previous_state.as_ref());

    let content: Element<GuiMessage> = match &state.dialog_shown {
        DialogShown::FileChooser => Space::new(Length::Shrink, Length::Shrink).into(),
        DialogShown::Alert(title, txt, _msg) => {
            let mut bold_font = iced::Font::default();
            bold_font.weight = iced::font::Weight::Bold;

            container(
                column![
                    container(text(title).style(text::danger).font(bold_font))
                        .center_x(Length::Fill)
                        .padding(5)
                        .style(container::bordered_box),
                    container(text(txt))
                        .center_x(Length::Fill)
                        .center_y(200)
                        .padding(5),
                    row![
                        button(container("Oui").center_x(Length::Fill))
                            .style(button::danger)
                            .width(Length::Fill)
                            .on_press(Message::AlertDialogClosed(true).into()),
                        button(container("Non").center_x(Length::Fill))
                            .style(button::primary)
                            .width(Length::Fill)
                            .on_press(Message::AlertDialogClosed(false).into()),
                    ]
                    .spacing(2)
                ]
                .spacing(2),
            )
            .style(container::rounded_box)
            .width(400)
            .padding(5)
            .into()
        }
    };

    tools::modal(normal_view, content)
}

pub fn title(state: &State) -> String {
    super::title(state.previous_state.as_ref())
}

pub fn exit_subscription(_state: &State) -> Subscription<GuiMessage> {
    Subscription::none()
}
