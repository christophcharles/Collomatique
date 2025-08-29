use iced::widget::{button, column, container, row, text, Space};
use iced::{Element, Length, Subscription, Task};

use super::{tools, GuiMessage, GuiState};

#[derive(Debug, Clone)]
pub struct FileDesc {
    pub path: std::path::PathBuf,
    pub save: bool,
}

#[derive(Clone)]
pub enum Message {
    FileChooserDialog(
        String,
        bool,
        std::sync::Arc<dyn Fn(Option<std::path::PathBuf>) -> GuiMessage + Send + Sync>,
    ),
    FileChooserDialogClosed(Option<std::path::PathBuf>),
    YesNoAlertDialog(
        String,
        String,
        std::sync::Arc<dyn Fn(bool) -> GuiMessage + Send + Sync>,
    ),
    YesNoAlertDialogClosed(bool),
    ErrorDialog(String, String),
    ErrorDialogClosed,
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::FileChooserDialog(title, save, _msg) => {
                write!(f, "Message::FileChooserDialog({:?}, {:?}, Fn)", title, save)
            }
            Message::FileChooserDialogClosed(file_desc) => {
                write!(f, "Message::FileChooserDialogClosed({:?}", file_desc)
            }
            Message::YesNoAlertDialog(title, txt, _msg) => {
                write!(f, "Message::YesNoAlertDialog({:?}, {:?}, Fn)", title, txt)
            }
            Message::YesNoAlertDialogClosed(result) => {
                write!(f, "Message::YesNoAlertDialogClosed({:?})", result)
            }
            Message::ErrorDialog(title, txt) => {
                write!(f, "Message::ErrorDialog({:?}, {:?})", title, txt)
            }
            Message::ErrorDialogClosed => {
                write!(f, "Message::ErrorDialogClosed")
            }
        }
    }
}

#[derive(Clone)]
pub enum DialogShown {
    FileChooser(std::sync::Arc<dyn Fn(Option<std::path::PathBuf>) -> GuiMessage + Send + Sync>),
    YesNoAlert(
        String,
        String,
        std::sync::Arc<dyn Fn(bool) -> GuiMessage + Send + Sync>,
    ),
    Error(String, String),
}

impl std::fmt::Debug for DialogShown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DialogShown::FileChooser(_msg) => write!(f, "DialogShown::FileChooser(Fn)"),
            DialogShown::YesNoAlert(title, txt, _msg) => {
                write!(f, "DialogShown::YesNoAlert({:?}, {:?}, Fn)", title, txt)
            }
            DialogShown::Error(title, txt) => {
                write!(f, "DialogShown::Error({:?}, {:?})", title, txt)
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
        Message::FileChooserDialog(title, save, msg) => {
            *state = GuiState::DialogShown(State {
                previous_state: Box::new(state.clone()),
                dialog_shown: DialogShown::FileChooser(msg),
            });
            Task::perform(file_chooser_dialog(title, save), |x| {
                Message::FileChooserDialogClosed(x).into()
            })
        }
        Message::FileChooserDialogClosed(path) => {
            let GuiState::DialogShown(dialog_state) = state else {
                panic!("Dialog message but not in dialog state");
            };
            let DialogShown::FileChooser(msg) = dialog_state.dialog_shown.clone() else {
                panic!("File chooser dialog message but not in file chooser dialog state");
            };

            *state = dialog_state.previous_state.as_ref().clone();
            Task::done(msg(path))
        }
        Message::YesNoAlertDialog(title, txt, msg) => {
            *state = GuiState::DialogShown(State {
                previous_state: Box::new(state.clone()),
                dialog_shown: DialogShown::YesNoAlert(title, txt, msg),
            });
            Task::none()
        }
        Message::YesNoAlertDialogClosed(result) => {
            let GuiState::DialogShown(dialog_state) = state else {
                panic!("Dialog message but not in dialog state");
            };
            let DialogShown::YesNoAlert(_title, _txt, msg) = dialog_state.dialog_shown.clone()
            else {
                panic!("Yes/No Alert dialog message but not in Yes/No alert dialog state");
            };

            *state = dialog_state.previous_state.as_ref().clone();
            Task::done(msg(result))
        }
        Message::ErrorDialog(title, txt) => {
            *state = GuiState::DialogShown(State {
                previous_state: Box::new(state.clone()),
                dialog_shown: DialogShown::Error(title, txt),
            });
            Task::none()
        }
        Message::ErrorDialogClosed => {
            let GuiState::DialogShown(dialog_state) = state else {
                panic!("Dialog message but not in dialog state");
            };
            let DialogShown::Error(_title, _txt) = dialog_state.dialog_shown.clone() else {
                panic!("Error dialog message but not in error dialog state");
            };

            *state = dialog_state.previous_state.as_ref().clone();
            Task::none()
        }
    }
}

async fn file_chooser_dialog(title: String, save: bool) -> Option<std::path::PathBuf> {
    let dialog = rfd::AsyncFileDialog::new()
        .set_title(title)
        .add_filter("Fichier Collomatique", &["json"])
        .add_filter("Tous les fichiers", &["*"]);

    let file = if save {
        dialog.save_file().await
    } else {
        dialog.pick_file().await
    };

    file.map(|handle| handle.path().to_owned())
}

pub fn view(state: &State) -> Element<GuiMessage> {
    let normal_view = super::view(state.previous_state.as_ref());

    let content: Element<GuiMessage> = match &state.dialog_shown {
        DialogShown::FileChooser(_msg) => Space::new(Length::Shrink, Length::Shrink).into(),
        DialogShown::YesNoAlert(title, txt, _msg) => {
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
                            .on_press(Message::YesNoAlertDialogClosed(true).into()),
                        button(container("Non").center_x(Length::Fill))
                            .style(button::primary)
                            .width(Length::Fill)
                            .on_press(Message::YesNoAlertDialogClosed(false).into()),
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
        DialogShown::Error(title, txt) => {
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
                    button(container("OK").center_x(Length::Fill))
                        .style(button::danger)
                        .width(Length::Fill)
                        .on_press(Message::ErrorDialogClosed.into()),
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
