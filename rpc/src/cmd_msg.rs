use super::*;

pub mod open_file_dialog;
pub use open_file_dialog::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CmdMsg {
    Update(collomatique_ops::UpdateOp),
    GuiRequest(GuiMsg),
    GetData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuiMsg {
    OpenFileDialog(OpenFileDialogMsg),
    OkDialog(String),
    ConfirmDialog(String),
    InputDialog(String, String),
}
