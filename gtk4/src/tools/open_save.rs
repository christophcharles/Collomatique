use std::path::{Path, PathBuf};

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
    generic_open_dialog(
        "Ouvrir",
        &[
            ("Fichiers collomatique (*.collomatique)", "collomatique"),
            ("Tous les fichiers", "*"),
        ],
        None,
    )
    .await
}

pub async fn open_python_dialog() -> Option<PathBuf> {
    generic_open_dialog(
        "Ouvrir un script",
        &[("Scripts Python (*.py)", "py"), ("Tous les fichiers", "*")],
        None,
    )
    .await
}

pub async fn generic_open_dialog(
    title: &str,
    extensions: &[(&str, &str)],
    default_dir: Option<&Path>,
) -> Option<PathBuf> {
    let mut dialog = rfd::AsyncFileDialog::new()
        .set_title(title)
        .set_can_create_directories(false);

    for (desc, ext) in extensions {
        dialog = dialog.add_filter(*desc, &[ext]);
    }

    if let Some(dir) = default_dir {
        dialog = dialog.set_directory(dir);
    }

    let file = dialog.pick_file().await;

    file.map(|handle| handle.path().to_owned())
}
