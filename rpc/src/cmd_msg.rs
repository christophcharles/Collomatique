use super::*;

pub mod open_file_dialog;
pub use open_file_dialog::*;

pub mod save_file_dialog;
pub use save_file_dialog::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CmdMsg {
    GuiRequest(GuiMsg),
    GetData,
    SetData(super::InternalDataStream),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuiMsg {
    OpenFileDialog(OpenFileDialogMsg),
    SaveFileDialog(SaveFileDialogMsg),
    OkDialog(String),
    ConfirmDialog(String),
    InputDialog(String, String),
}
