use super::*;

pub mod open_file_dialog;
pub use open_file_dialog::*;

pub mod save_file_dialog;
pub use save_file_dialog::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuiAnswer {
    OpenFileDialog(OpenFileDialogAnswer),
    SaveFileDialog(SaveFileDialogAnswer),
    OkDialogClosed,
    ConfirmDialog(bool),
    InputDialog(Option<String>),
}
