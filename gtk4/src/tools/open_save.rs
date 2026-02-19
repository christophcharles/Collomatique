use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub enum DefaultSaveFile {
    None,
    ExistingFile(PathBuf),
    SuggestedName(String),
}

fn build_save_dialog(
    title: &str,
    extensions: &[(&str, &str)],
    default: DefaultSaveFile,
) -> rfd::AsyncFileDialog {
    let mut dialog = rfd::AsyncFileDialog::new()
        .set_title(title)
        .set_can_create_directories(true);

    for (desc, ext) in extensions {
        dialog = dialog.add_filter(*desc, &[ext]);
    }

    match default {
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

    dialog
}

pub async fn save_collomatique_dialog(default: DefaultSaveFile) -> Option<PathBuf> {
    build_save_dialog(
        "Enregistrer sous",
        &[
            ("Fichiers collomatique (*.collomatique)", "collomatique"),
            ("Tous les fichiers", "*"),
        ],
        default,
    )
    .save_file()
    .await
    .map(|h| h.path().to_owned())
}

pub async fn save_xlsx_dialog(default: DefaultSaveFile) -> Option<PathBuf> {
    build_save_dialog(
        "Exporter en XLSX",
        &[
            ("Fichiers XLSX (*.xlsx)", "xlsx"),
            ("Tous les fichiers", "*"),
        ],
        default,
    )
    .save_file()
    .await
    .map(|h| h.path().to_owned())
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

pub async fn generic_save_dialog(
    title: &str,
    extensions: &[(&str, &str)],
    suggested_name: Option<&str>,
) -> Option<PathBuf> {
    let mut dialog = rfd::AsyncFileDialog::new()
        .set_title(title)
        .set_can_create_directories(true);

    for (desc, ext) in extensions {
        dialog = dialog.add_filter(*desc, &[ext]);
    }

    if let Some(name) = suggested_name {
        dialog = dialog.set_file_name(name);
    }

    let file = dialog.save_file().await;

    file.map(|handle| handle.path().to_owned())
}
