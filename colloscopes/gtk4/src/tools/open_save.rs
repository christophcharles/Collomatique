use relm4::gtk;
use relm4::gtk::glib::object::IsA;
use std::path::{Path, PathBuf};

/// The window a file dialog belongs to, reduced to something `Send`.
///
/// The dialogs below are `async` and run off the main thread, so they cannot
/// hold a GTK object. On Windows the raw `HWND` is all `rfd` needs to give the
/// dialog an owner; on unix the portal parents dialogs on its own and this type
/// carries nothing.
///
/// `None` — no parent at all — stays a valid choice: a dialog may be asked for
/// by code that has no window of its own.
#[derive(Clone, Copy, Debug)]
pub struct ParentWindowHandle {
    #[cfg(windows)]
    hwnd: std::num::NonZeroIsize,
}

impl ParentWindowHandle {
    /// The handle of the toplevel window `widget` sits in, if that window is
    /// realized. On unix there is never a handle.
    pub fn from_widget(widget: &impl IsA<gtk::Widget>) -> Option<Self> {
        #[cfg(windows)]
        {
            use relm4::gtk::prelude::{Cast, NativeExt, WidgetExt};

            let window = widget.as_ref().root()?.downcast::<gtk::Window>().ok()?;
            let surface = window
                .surface()?
                .downcast::<gdk4_win32::Win32Surface>()
                .ok()?;
            let hwnd = std::num::NonZeroIsize::new(surface.handle().0)?;
            Some(ParentWindowHandle { hwnd })
        }
        #[cfg(not(windows))]
        {
            let _ = widget;
            None
        }
    }
}

#[cfg(windows)]
mod parent_window_handle {
    use raw_window_handle::{
        DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle,
        Win32WindowHandle, WindowHandle,
    };

    impl HasWindowHandle for super::ParentWindowHandle {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            let raw = RawWindowHandle::Win32(Win32WindowHandle::new(self.hwnd));
            // SAFETY: the window outlives the dialog -- the component that
            // opened the dialog keeps its own window alive meanwhile.
            Ok(unsafe { WindowHandle::borrow_raw(raw) })
        }
    }

    impl HasDisplayHandle for super::ParentWindowHandle {
        fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
            Ok(DisplayHandle::windows())
        }
    }
}

#[cfg(windows)]
fn with_parent(
    dialog: rfd::AsyncFileDialog,
    parent: Option<ParentWindowHandle>,
) -> rfd::AsyncFileDialog {
    match parent {
        Some(handle) => dialog.set_parent(&handle),
        None => dialog,
    }
}

#[cfg(not(windows))]
fn with_parent(
    dialog: rfd::AsyncFileDialog,
    _parent: Option<ParentWindowHandle>,
) -> rfd::AsyncFileDialog {
    dialog
}

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

pub async fn save_collomatique_dialog(
    parent: Option<ParentWindowHandle>,
    default: DefaultSaveFile,
) -> Option<PathBuf> {
    with_parent(
        build_save_dialog(
            "Enregistrer sous",
            &[
                ("Fichiers collomatique (*.collomatique)", "collomatique"),
                ("Tous les fichiers", "*"),
            ],
            default,
        ),
        parent,
    )
    .save_file()
    .await
    .map(|h| h.path().to_owned())
}

pub async fn save_xlsx_dialog(
    parent: Option<ParentWindowHandle>,
    default: DefaultSaveFile,
) -> Option<PathBuf> {
    with_parent(
        build_save_dialog(
            "Exporter en XLSX",
            &[
                ("Fichiers XLSX (*.xlsx)", "xlsx"),
                ("Tous les fichiers", "*"),
            ],
            default,
        ),
        parent,
    )
    .save_file()
    .await
    .map(|h| h.path().to_owned())
}

pub async fn save_mps_dialog(
    parent: Option<ParentWindowHandle>,
    default: DefaultSaveFile,
) -> Option<PathBuf> {
    with_parent(
        build_save_dialog(
            "Exporter le problème ILP (MPS)",
            &[("Fichiers MPS (*.mps)", "mps"), ("Tous les fichiers", "*")],
            default,
        ),
        parent,
    )
    .save_file()
    .await
    .map(|h| h.path().to_owned())
}

pub async fn open_dialog(parent: Option<ParentWindowHandle>) -> Option<PathBuf> {
    generic_open_dialog(
        parent,
        "Ouvrir",
        &[
            ("Fichiers collomatique (*.collomatique)", "collomatique"),
            ("Tous les fichiers", "*"),
        ],
        None,
    )
    .await
}

pub async fn open_python_dialog(parent: Option<ParentWindowHandle>) -> Option<PathBuf> {
    generic_open_dialog(
        parent,
        "Ouvrir un script",
        &[("Scripts Python (*.py)", "py"), ("Tous les fichiers", "*")],
        None,
    )
    .await
}

pub async fn generic_open_dialog(
    parent: Option<ParentWindowHandle>,
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

    let file = with_parent(dialog, parent).pick_file().await;

    file.map(|handle| handle.path().to_owned())
}

pub async fn generic_save_dialog(
    parent: Option<ParentWindowHandle>,
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

    let file = with_parent(dialog, parent).save_file().await;

    file.map(|handle| handle.path().to_owned())
}
