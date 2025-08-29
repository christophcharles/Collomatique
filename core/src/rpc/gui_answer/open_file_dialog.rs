use super::*;

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenFileDialogAnswer {
    pub file_path: Option<PathBuf>,
}
