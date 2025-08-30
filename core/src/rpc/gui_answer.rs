use super::*;

pub mod open_file_dialog;
pub use open_file_dialog::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuiAnswer {
    OpenFileDialog(OpenFileDialogAnswer),
    OkDialogClosed,
    ConfirmDialog(bool),
}
