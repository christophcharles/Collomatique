use adw::prelude::NavigationPageExt;
use collomatique_state::traits::Manager;
use gtk::prelude::{ButtonExt, WidgetExt};
use relm4::prelude::ComponentController;
use relm4::{adw, gtk};
use relm4::{Component, ComponentParts, ComponentSender, Controller};
use std::num::NonZeroU32;
use std::path::PathBuf;

use collomatique_state::AppState;
use collomatique_state_colloscopes::Data;

use crate::dialogs;

#[derive(Debug)]
pub enum EditorInput {
    Ignore,
    NewFile {
        file_name: Option<PathBuf>,
        data: collomatique_state_colloscopes::Data,
        dirty: bool,
    },
    SaveCurrentFileAs(PathBuf),
    SaveAsClicked,
    SaveClicked,
    UndoClicked,
    RedoClicked,
}

#[derive(Debug)]
pub enum EditorOutput {
    UpdateActions,
    SaveError(PathBuf, String),
}

#[derive(Debug)]
pub enum EditorCommandOutput {
    SaveSuccessful(PathBuf),
    SaveFailed(PathBuf, String),
}

const DEFAULT_TOAST_TIMEOUT: Option<NonZeroU32> = NonZeroU32::new(5);

mod toast {
    use std::{num::NonZeroU32, sync::atomic::AtomicBool};

    pub struct ToastInfo {
        need_update: AtomicBool,
        text: Option<String>,
        timeout: Option<NonZeroU32>,
    }

    impl ToastInfo {
        pub fn new() -> Self {
            ToastInfo {
                need_update: AtomicBool::new(false),
                text: None,
                timeout: None,
            }
        }

        pub fn new_toast(&mut self, text: String, timeout: Option<NonZeroU32>) {
            self.need_update
                .store(true, std::sync::atomic::Ordering::Release);
            self.text = Some(text);
            self.timeout = timeout;
        }

        pub fn dismiss_toast(&mut self) {
            self.need_update
                .store(true, std::sync::atomic::Ordering::Release);
            self.text = None;
        }

        pub fn get_toast(&self) -> Option<&str> {
            self.text.as_ref().map(|x| x.as_str())
        }

        pub fn get_timeout(&self) -> Option<&NonZeroU32> {
            self.timeout.as_ref()
        }

        pub fn updated(&self) {
            self.need_update
                .store(false, std::sync::atomic::Ordering::Release);
        }

        pub fn need_update(&self) -> bool {
            self.need_update.load(std::sync::atomic::Ordering::Acquire)
        }
    }
}

pub struct EditorPanel {
    file_name: Option<PathBuf>,
    data: AppState<Data>,
    dirty: bool,
    save_dialog: Controller<dialogs::open_save::Dialog>,
    toast_info: toast::ToastInfo,
}

impl EditorPanel {
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn can_undo(&self) -> bool {
        self.data.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.data.can_redo()
    }
}

impl EditorPanel {
    fn generate_subtitle(&self) -> String {
        let default_name = "Fichier sans nom".into();
        let name = match &self.file_name {
            Some(path) => match path.file_name() {
                Some(file_name) => file_name.to_string_lossy().to_string(),
                None => default_name,
            },
            None => default_name,
        };
        if self.dirty {
            String::from("*") + &name
        } else {
            name
        }
    }
}

#[relm4::component(pub)]
impl Component for EditorPanel {
    type Input = EditorInput;
    type Output = EditorOutput;
    type Init = ();
    type CommandOutput = EditorCommandOutput;

    view! {
        #[root]
        nav_view = adw::NavigationSplitView {
            set_hexpand: true,
            set_vexpand: true,
            #[wrap(Some)]
            set_sidebar = &adw::NavigationPage {
                set_title: "Collomatique",
                #[wrap(Some)]
                set_child = &adw::ToolbarView {
                    add_top_bar = &adw::HeaderBar {
                        #[wrap(Some)]
                        set_title_widget = &adw::WindowTitle {
                            set_title: "Collomatique",
                            #[watch]
                            set_subtitle: &model.generate_subtitle(),
                        },
                        pack_end = &gtk::MenuButton {
                            set_icon_name: "open-menu-symbolic",
                            set_menu_model: Some(&main_menu),
                        },
                    },
                    #[wrap(Some)]
                    set_content = &gtk::StackSidebar {
                        set_vexpand: true,
                        set_size_request: (200, -1),
                        set_stack: &main_stack,
                    },
                },
            },
            #[wrap(Some)]
            set_content = &adw::NavigationPage {
                set_title: "Editor Panel",
                #[wrap(Some)]
                set_child = &adw::ToolbarView {
                    add_top_bar = &adw::HeaderBar {
                        pack_start = &gtk::Box {
                            add_css_class: "linked",
                            gtk::Button {
                                set_icon_name: "edit-undo",
                                #[watch]
                                set_sensitive: model.can_undo(),
                                connect_clicked => EditorInput::UndoClicked,
                            },
                            gtk::Button {
                                set_icon_name: "edit-redo",
                                #[watch]
                                set_sensitive: model.can_redo(),
                                connect_clicked => EditorInput::RedoClicked,
                            },
                        },
                        pack_end = &gtk::Box {
                            add_css_class: "linked",
                            gtk::Button::with_label("Enregistrer") {
                                #[watch]
                                set_sensitive: model.dirty,
                                connect_clicked => EditorInput::SaveClicked,
                            },
                            gtk::Button {
                                set_icon_name: "document-save-as",
                                connect_clicked => EditorInput::SaveAsClicked,
                            },
                        },
                    },
                    #[wrap(Some)]
                    #[name(toast_overlay)]
                    set_content = &adw::ToastOverlay {
                        #[name(main_stack)]
                        gtk::Stack {
                            set_hexpand: true,
                            add_titled: (&gtk::Label::new(Some("Test1 - content")), Some("test1"), &"Test1"),
                            add_titled: (&gtk::Label::new(Some("Test2 - content")), Some("test2"), &"Test2"),
                            add_titled: (&gtk::Label::new(Some("Test3 - content")), Some("test3"), &"Test3"),
                            set_transition_type: gtk::StackTransitionType::SlideUpDown,
                        }
                    },
                },
            },
        }
    }

    menu! {
        main_menu: {
            section! {
                "Nouveau" => super::NewAction,
                "Ouvrir" => super::OpenAction,
            },
            section! {
                "Annuler" => super::UndoAction,
                "Rétablir" => super::RedoAction,
            },
            section! {
                "Enregistrer" => super::SaveAction,
                "Enregistrer sous" => super::SaveAsAction,
            },
            section! {
                "Fermer" => super::CloseAction,
            },
            section! {
                "À propos" => super::AboutAction
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let save_dialog = dialogs::open_save::Dialog::builder()
            .transient_for_native(&root)
            .launch(dialogs::open_save::Type::Save)
            .forward(sender.input_sender(), |msg| match msg {
                dialogs::open_save::DialogOutput::Cancel => EditorInput::Ignore,
                dialogs::open_save::DialogOutput::FileSelected(path) => {
                    EditorInput::SaveCurrentFileAs(path)
                }
            });

        let model = EditorPanel {
            file_name: None,
            data: AppState::new(Data::new()),
            dirty: false,
            save_dialog,
            toast_info: toast::ToastInfo::new(),
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            EditorInput::Ignore => {}
            EditorInput::NewFile {
                file_name,
                data,
                dirty,
            } => {
                self.file_name = file_name;
                self.data = AppState::new(data);
                self.dirty = dirty;
                sender.output(EditorOutput::UpdateActions).unwrap();
            }
            EditorInput::SaveClicked => match &self.file_name {
                Some(path) => {
                    sender.input(EditorInput::SaveCurrentFileAs(path.clone()));
                }
                None => {
                    sender.input(EditorInput::SaveAsClicked);
                }
            },
            EditorInput::SaveAsClicked => {
                self.save_dialog
                    .sender()
                    .send(dialogs::open_save::DialogInput::ShowWithDefault(
                        match &self.file_name {
                            Some(path) => {
                                dialogs::open_save::DefaultFile::ExistingFile(path.clone())
                            }
                            None => dialogs::open_save::DefaultFile::SuggestedName(
                                "FichierSansNom.collomatique".into(),
                            ),
                        },
                    ))
                    .unwrap();
            }
            EditorInput::SaveCurrentFileAs(path) => {
                let data_copy = self.data.get_data().clone();
                self.dirty = false;
                self.file_name = Some(path.clone());
                self.toast_info.new_toast(
                    format!("Enregistrement en cours de {}...", path.to_string_lossy(),),
                    None,
                );
                sender.oneshot_command(async move {
                    match collomatique_storage::save_data_to_file(&data_copy, &path).await {
                        Ok(()) => EditorCommandOutput::SaveSuccessful(path),
                        Err(e) => EditorCommandOutput::SaveFailed(path, e.to_string()),
                    }
                });
                sender.output(EditorOutput::UpdateActions).unwrap();
            }
            EditorInput::UndoClicked => {
                if self.data.can_undo() {
                    self.data.undo().expect("Should be able to undo");
                    sender.output(EditorOutput::UpdateActions).unwrap();
                }
            }
            EditorInput::RedoClicked => {
                if self.data.can_redo() {
                    self.data.redo().expect("Should be able to undo");
                    sender.output(EditorOutput::UpdateActions).unwrap();
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            EditorCommandOutput::SaveSuccessful(path) => {
                self.toast_info.new_toast(
                    format!("{} enregistré", path.to_string_lossy()),
                    DEFAULT_TOAST_TIMEOUT,
                );
            }
            EditorCommandOutput::SaveFailed(path, error) => {
                if Some(&path) != self.file_name.as_ref() {
                    return;
                }
                self.toast_info.dismiss_toast();
                self.dirty = true;
                sender.output(EditorOutput::UpdateActions).unwrap();
                sender.output(EditorOutput::SaveError(path, error)).unwrap();
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.toast_info.need_update() {
            widgets.toast_overlay.dismiss_all();
            if let Some(text) = self.toast_info.get_toast() {
                let new_toast = adw::Toast::new(text);
                new_toast.set_timeout(match self.toast_info.get_timeout() {
                    Some(t) => t.get(),
                    None => 0,
                });
                widgets.toast_overlay.add_toast(new_toast);
            }
            self.toast_info.updated();
        }
    }
}
