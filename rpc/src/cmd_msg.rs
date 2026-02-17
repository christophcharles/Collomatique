use super::*;

pub mod open_file_dialog;
pub use open_file_dialog::*;

pub mod save_file_dialog;
pub use save_file_dialog::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CmdMsg {
    Update(collomatique_ops::UpdateOp),
    GuiRequest(GuiMsg),
    GetData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuiMsg {
    OpenFileDialog(OpenFileDialogMsg),
    SaveFileDialog(SaveFileDialogMsg),
    OkDialog(String),
    ConfirmDialog(String),
    InputDialog(String, String),
}
