use adw::prelude::NavigationPageExt;
use collomatique_state::traits::Manager;
use gtk::prelude::{ButtonExt, WidgetExt};
use relm4::prelude::ComponentController;
use relm4::{adw, gtk};
use relm4::{Component, ComponentParts, ComponentSender, Controller, SimpleComponent};
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
}

pub struct EditorPanel {
    file_name: Option<PathBuf>,
    data: AppState<Data>,
    dirty: bool,
    save_dialog: Controller<dialogs::open_save::Dialog>,
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
impl SimpleComponent for EditorPanel {
    type Input = EditorInput;
    type Output = EditorOutput;
    type Init = ();

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
                    #[name(main_stack)]
                    set_content = &gtk::Stack {
                        set_hexpand: true,
                        add_titled: (&gtk::Label::new(Some("Test1 - content")), Some("test1"), &"Test1"),
                        add_titled: (&gtk::Label::new(Some("Test2 - content")), Some("test2"), &"Test2"),
                        add_titled: (&gtk::Label::new(Some("Test3 - content")), Some("test3"), &"Test3"),
                        set_transition_type: gtk::StackTransitionType::SlideUpDown,
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
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
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
                                "FichierSansNom.colloscope".into(),
                            ),
                        },
                    ))
                    .unwrap();
            }
            EditorInput::SaveCurrentFileAs(_path) => {}
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
}
