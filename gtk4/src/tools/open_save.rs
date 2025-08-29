use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum DefaultSaveFile {
    None,
    ExistingFile(PathBuf),
    SuggestedName(String),
}

pub async fn save_dialog(default_name: DefaultSaveFile) -> Option<PathBuf> {
    let mut dialog = rfd::AsyncFileDialog::new()
        .set_title("Enregistrer sous...")
        .set_can_create_directories(true)
        .add_filter("Fichiers collomatique", &["collomatique"])
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
    SelectedFiles::open_file()
        .title("Ouvrir")
        .accept_label("Ouvrir")
        .filter(FileFilter::new("Fichiers collomatique (*.collomatique)").glob("*.collomatique"))
        .filter(FileFilter::new("Tous les fichiers").glob("*"))
        .modal(true)
        .send()
        .await
        .ok()?
        .response()
        .ok()?
        .uris()
        .first()?
        .to_file_path()
        .ok()
}

pub async fn open_python_dialog() -> Option<PathBuf> {
    SelectedFiles::open_file()
        .title("Ouvrir un script")
        .accept_label("Ouvrir")
        .filter(FileFilter::new("Scripts Python (*.py)").glob("*.py"))
        .filter(FileFilter::new("Tous les fichiers").glob("*"))
        .modal(true)
        .send()
        .await
        .ok()?
        .response()
        .ok()?
        .uris()
        .first()?
        .to_file_path()
        .ok()
}
