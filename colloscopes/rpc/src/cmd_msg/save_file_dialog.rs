use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveFileDialogMsg {
    pub title: String,
    pub list: Vec<ExtensionDesc>,
    pub suggested_name: Option<String>,
}
