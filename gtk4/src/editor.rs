use adw::prelude::NavigationPageExt;
use gtk::prelude::{ButtonExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, SimpleComponent};
use std::path::PathBuf;

use collomatique_state::AppState;
use collomatique_state_colloscopes::Data;

#[derive(Debug)]
pub enum EditorInput {
    NewFile {
        file_name: Option<PathBuf>,
        data: collomatique_state_colloscopes::Data,
        dirty: bool,
    },
}

pub struct EditorPanel {
    file_name: Option<PathBuf>,
    data: AppState<Data>,
    dirty: bool,
}

impl EditorPanel {
    pub fn is_dirty(&self) -> bool {
        self.dirty
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
    type Output = ();
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
                                set_sensitive: false,
                            },
                            gtk::Button {
                                set_icon_name: "edit-redo",
                                set_sensitive: false,
                            },
                        },
                        pack_end = &gtk::Box {
                            add_css_class: "linked",
                            gtk::Button::with_label("Enregistrer") {
                                set_sensitive: false,
                            },
                            gtk::Button {
                                set_icon_name: "document-save-as",
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
        let model = EditorPanel {
            file_name: None,
            data: AppState::new(Data::new()),
            dirty: false,
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            EditorInput::NewFile {
                file_name,
                data,
                dirty,
            } => {
                self.file_name = file_name;
                self.data = AppState::new(data);
                self.dirty = dirty;
            }
        }
    }
}
