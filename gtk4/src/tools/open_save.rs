use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum DefaultSaveFile {
    None,
    ExistingFile(PathBuf),
    SuggestedName(String),
}

pub async fn save_dialog(default_name: DefaultSaveFile) -> Option<PathBuf> {
    let mut files = SelectedFiles::save_file()
        .title("Enregistrer sous...")
        .accept_label("Enregistrer")
        .filter(FileFilter::new("Fichiers collomatique (*.collomatique)").glob("*.collomatique"))
        .filter(FileFilter::new("Tous les fichiers").glob("*"))
        .modal(true);

    match default_name {
        DefaultSaveFile::None => {}
        DefaultSaveFile::ExistingFile(path) => {
            files = files.current_file(path).ok()?;
        }
        DefaultSaveFile::SuggestedName(name) => {
            files = files.current_name(name.as_str());
        }
    }

    files
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
