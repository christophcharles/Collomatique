use iced::widget::text;
use iced::{Element, Task};

use super::{tools, GuiMessage, GuiState};

#[derive(Debug, Clone)]
pub struct FileDesc {
    pub path: std::path::PathBuf,
    pub create: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenNewFile,
    OpenExistingFile,
    FileSelected(Option<FileDesc>),
}

#[derive(Debug, Clone)]
pub struct State {
    previous_state: Box<GuiState>,
}

pub fn update(state: &mut GuiState, message: Message) -> Task<GuiMessage> {
    match message {
        Message::OpenNewFile => {
            *state = GuiState::DialogShown(State {
                previous_state: Box::new(state.clone()),
            });
            Task::perform(open_file_dialog(true), |x| {
                GuiMessage::DialogMessage(Message::FileSelected(x))
            })
        }
        Message::OpenExistingFile => {
            *state = GuiState::DialogShown(State {
                previous_state: Box::new(state.clone()),
            });
            Task::perform(open_file_dialog(false), |x| {
                GuiMessage::DialogMessage(Message::FileSelected(x))
            })
        }
        Message::FileSelected(file_desc) => {
            let GuiState::DialogShown(dialog_state) = state else {
                panic!("Dialog message but not in dialog state");
            };

            *state = dialog_state.previous_state.as_ref().clone();
            Task::done(GuiMessage::FileSelected(file_desc))
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

    tools::modal(normal_view, text(""))
}

pub fn title(state: &State) -> String {
    super::title(state.previous_state.as_ref())
}
