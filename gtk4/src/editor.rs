use adw::prelude::NavigationPageExt;
use gtk::prelude::WidgetExt;
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
                    add_top_bar: &adw::HeaderBar::new(),
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
                set_title: "test.collomatique",
                #[wrap(Some)]
                set_child = &adw::ToolbarView {
                    add_top_bar: &adw::HeaderBar::new(),
                    #[wrap(Some)]
                    #[name(main_stack)]
                    set_content = &gtk::Stack {
                        set_hexpand: true,
                        add_titled: (&gtk::Label::new(Some("Test1 - content")), Some("Test1"), &"Test1"),
                        add_titled: (&gtk::Label::new(Some("Test2 - content")), Some("Test2"), &"Test2"),
                        add_titled: (&gtk::Label::new(Some("Test3 - content")), Some("Test3"), &"Test3"),
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
