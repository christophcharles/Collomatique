use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenFileDialogMsg {
    pub title: String,
    pub list: Vec<ExtensionDesc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionDesc {
    pub desc: String,
    pub extension: String,
}
