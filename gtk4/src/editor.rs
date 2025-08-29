use adw::prelude::NavigationPageExt;
use gtk::prelude::{ButtonExt, WidgetExt};
use relm4::component::{AsyncComponentParts, AsyncComponentSender, SimpleAsyncComponent};
use relm4::{adw, gtk};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorInput {
    NewFile(Option<PathBuf>),
    ExistingFile(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDesc {
    pub file_name: Option<PathBuf>,
}

pub struct EditorPanel {
    current_file: FileDesc,
}

impl EditorPanel {
    fn generate_subtitle(&self) -> String {
        match &self.current_file.file_name {
            Some(path) => path.to_string_lossy().to_string(),
            None => "Fichier sans nom".into(),
        }
    }
}

#[relm4::component(async, pub)]
impl SimpleAsyncComponent for EditorPanel {
    type Input = EditorInput;
    type Output = ();
    type Init = EditorInput;

    view! {
        #[root]
        adw::NavigationSplitView {
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
                        pack_end = &gtk::Button {
                            set_icon_name: "open-menu-symbolic",
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

    async fn init(
        params: Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = EditorPanel {
            current_file: match params {
                EditorInput::NewFile(file_name) => FileDesc { file_name },
                EditorInput::ExistingFile(file_name) => FileDesc {
                    file_name: Some(file_name),
                },
            },
        };
        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, message: Self::Input, _sender: AsyncComponentSender<Self>) {
        match message {
            EditorInput::NewFile(file_name) => self.current_file = FileDesc { file_name },
            EditorInput::ExistingFile(file_name) => {
                self.current_file = FileDesc {
                    file_name: Some(file_name),
                }
            }
        }
    }
}
