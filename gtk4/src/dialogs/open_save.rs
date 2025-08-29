use gtk::gio;
use gtk::prelude::{FileChooserExt, FileExt, NativeDialogExt};
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};
use std::path::PathBuf;

pub struct Dialog {
    hidden: bool,
    default_file: DefaultFile,
    dialog_type: Type,
}

#[derive(Clone, Debug)]
pub enum DefaultFile {
    None,
    ExistingFile(PathBuf),
    SuggestedName(String),
}

impl DefaultFile {
    fn use_set_file(&self) -> bool {
        match *self {
            DefaultFile::ExistingFile(_) => true,
            _ => false,
        }
    }

    fn use_set_current_name(&self) -> bool {
        match *self {
            DefaultFile::SuggestedName(_) => true,
            _ => false,
        }
    }

    fn extract_file(&self) -> PathBuf {
        if let DefaultFile::ExistingFile(ref path) = *self {
            path.clone()
        } else {
            let mut path = std::env::current_dir().unwrap_or(PathBuf::from(""));
            path.push("Test.collomatique");
            path
        }
    }

    fn extract_current_name(&self) -> String {
        if let DefaultFile::SuggestedName(ref name) = *self {
            name.clone()
        } else {
            String::from("")
        }
    }
}

#[derive(Eq, PartialEq)]
pub enum Type {
    Open,
    Save,
}

#[derive(Debug)]
pub enum DialogInput {
    Show,
    ShowWithDefault(DefaultFile),
    Hide,
}

#[derive(Debug)]
pub enum DialogOutput {
    Cancel,
    FileSelected(PathBuf),
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = Type;

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        file_chooser = gtk::FileChooserNative {
            set_action: match model.dialog_type {
                Type::Open => gtk::FileChooserAction::Open,
                Type::Save => gtk::FileChooserAction::Save,
            },
            set_select_multiple: false,
            set_create_folders: model.dialog_type == Type::Save,
            set_modal: true,
            set_accept_label: match model.dialog_type {
                Type::Open => Some(&"Ouvrir"),
                Type::Save => Some(&"Enregistrer"),
            },
            set_cancel_label: Some(&"Annuler"),
            add_filter = &gtk::FileFilter {
                set_name: Some(&"Fichiers collomatique (*.collomatique)"),
                add_suffix: &"collomatique",
            },
            add_filter = &gtk::FileFilter {
                set_name: Some(&"Tous les fichiers"),
                add_pattern: &"*",
            },
            #[track(model.default_file.use_set_file())]
            #[chain(expect(format!("set_file error for path {}", model.default_file.extract_file().display()).as_str()))]
            set_file: &gio::File::for_path(&model.default_file.extract_file()),

            #[watch]
            set_visible: !model.hidden,

            connect_response[sender] => move |dialog, res_ty| {
                match res_ty {
                    gtk::ResponseType::Accept => {
                        let path = dialog.file().expect("No file selected").path().expect("No path");
                        sender.output(DialogOutput::FileSelected(path)).unwrap();
                    }
                    _ => sender.output(DialogOutput::Cancel).unwrap(),
                }

                sender.input(DialogInput::Hide);
            }
        }
    }

    fn init(
        params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Dialog {
            hidden: true,
            default_file: DefaultFile::None,
            dialog_type: params,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            DialogInput::ShowWithDefault(default_file) => {
                self.default_file = default_file.clone();
                self.hidden = false;
            }
            DialogInput::Show => self.hidden = false,
            DialogInput::Hide => self.hidden = true,
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.dialog_type == Type::Save {
            if self.default_file.use_set_current_name() {
                widgets
                    .file_chooser
                    .set_current_name(&model.default_file.extract_current_name());
            }
        }
    }
}
