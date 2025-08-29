use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum DefaultSaveFile {
    None,
    ExistingFile(PathBuf),
    SuggestedName(String),
}

pub async fn save_dialog(default_name: DefaultSaveFile) -> Option<PathBuf> {
    let mut dialog = rfd::AsyncFileDialog::new()
        .set_title("Enregistrer sous")
        .set_can_create_directories(true)
        .add_filter("Fichiers collomatique (*.collomatique)", &["collomatique"])
        .add_filter("Tous les fichiers", &["*"]);

    match default_name {
        DefaultSaveFile::None => {}
        DefaultSaveFile::ExistingFile(mut path) => {
            let filename = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            path.pop();
            dialog = dialog.set_file_name(filename).set_directory(path);
        }
        DefaultSaveFile::SuggestedName(name) => {
            dialog = dialog.set_file_name(name);
        }
    }

    let file = dialog.save_file().await;

    file.map(|handle| handle.path().to_owned())
}

pub async fn open_dialog() -> Option<PathBuf> {
    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Ouvrir")
        .set_can_create_directories(false)
        .add_filter("Fichiers collomatique (*.collomatique)", &["collomatique"])
        .add_filter("Tous les fichiers", &["*"]);

    let file = dialog.pick_file().await;

    file.map(|handle| handle.path().to_owned())
}

pub async fn open_python_dialog() -> Option<PathBuf> {
    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Ouvrir un script")
        .set_can_create_directories(false)
        .add_filter("Scripts Python (*.py)", &["py"])
        .add_filter("Tous les fichiers", &["*"]);

    let file = dialog.pick_file().await;

    file.map(|handle| handle.path().to_owned())
}
