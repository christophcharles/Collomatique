use adw::prelude::NavigationPageExt;
use collomatique_state::traits::Manager;
use gtk::prelude::{ButtonExt, ObjectExt, OrientableExt, WidgetExt};
use relm4::prelude::ComponentController;
use relm4::{adw, gtk};
use relm4::{Component, ComponentParts, ComponentSender, Controller};
use std::collections::BTreeMap;
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

const DEFAULT_TOAST_TIMEOUT: Option<NonZeroU32> = NonZeroU32::new(3);

enum ToastInfo {
    Toast {
        text: String,
        timeout: Option<NonZeroU32>,
    },
    Dismiss,
}

pub struct EditorPanel {
    file_name: Option<PathBuf>,
    data: AppState<Data>,
    dirty: bool,
    save_dialog: Controller<dialogs::open_save::Dialog>,
    toast_info: Option<ToastInfo>,
    pages_names: Vec<&'static str>,
    pages_titles_map: BTreeMap<&'static str, &'static str>,
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

    fn generate_tooltip_text(&self) -> Option<String> {
        self.file_name
            .as_ref()
            .map(|x| x.to_string_lossy().to_string())
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
                            #[watch]
                            set_tooltip_text: model.generate_tooltip_text().as_deref(),
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
                #[watch]
                set_title: match main_stack.visible_child_name() {
                    Some(n) => model.pages_titles_map.get(n.as_str()).unwrap(),
                    None => "Editor Panel",
                },
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
                        pack_end = &gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                            add_css_class: "spacer",
                        },
                        pack_end = &gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                            add_css_class: "spacer",
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
                            set_transition_type: gtk::StackTransitionType::SlideUpDown,
                            // Force update_view when visible-child is changed
                            // This maintains the title up top
                            connect_notify: (
                                Some("visible-child"),
                                {
                                    let sender = sender.clone();
                                    move |_widget,_| {
                                        sender.input(EditorInput::Ignore);
                                    }
                                }
                            ),
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

        let pages_names = vec!["test1", "test2", "test3"];

        let pages_titles_map =
            BTreeMap::from([("test1", "Test1"), ("test2", "Test2"), ("test3", "Test3")]);

        let model = EditorPanel {
            file_name: None,
            data: AppState::new(Data::new()),
            dirty: false,
            save_dialog,
            toast_info: None,
            pages_names,
            pages_titles_map,
        };
        let widgets = view_output!();

        widgets.main_stack.add_titled(
            &gtk::Label::new(Some("Test1 - content")),
            Some(model.pages_names[0]),
            model.pages_titles_map.get(model.pages_names[0]).unwrap(),
        );
        widgets.main_stack.add_titled(
            &gtk::Label::new(Some("Test2 - content")),
            Some(model.pages_names[1]),
            model.pages_titles_map.get(model.pages_names[1]).unwrap(),
        );
        widgets.main_stack.add_titled(
            &gtk::Label::new(Some("Test3 - content")),
            Some(model.pages_names[2]),
            model.pages_titles_map.get(model.pages_names[2]).unwrap(),
        );

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
                self.toast_info = Some(ToastInfo::Toast {
                    text: format!("Enregistrement en cours de {}...", path.to_string_lossy(),),
                    timeout: None,
                });
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
                    self.dirty = true;
                    sender.output(EditorOutput::UpdateActions).unwrap();
                }
            }
            EditorInput::RedoClicked => {
                if self.data.can_redo() {
                    self.data.redo().expect("Should be able to undo");
                    self.dirty = true;
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
                self.toast_info = Some(ToastInfo::Toast {
                    text: format!("{} enregistré", path.to_string_lossy()),
                    timeout: DEFAULT_TOAST_TIMEOUT,
                });
            }
            EditorCommandOutput::SaveFailed(path, error) => {
                if Some(&path) != self.file_name.as_ref() {
                    return;
                }
                self.toast_info = Some(ToastInfo::Dismiss);
                self.dirty = true;
                sender.output(EditorOutput::UpdateActions).unwrap();
                sender.output(EditorOutput::SaveError(path, error)).unwrap();
            }
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.update(message, sender.clone(), root);
        self.update_toast(widgets);
        self.update_view(widgets, sender);
    }

    fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.update_cmd(message, sender.clone(), root);
        self.update_toast(widgets);
        self.update_view(widgets, sender);
    }
}

impl EditorPanel {
    fn update_toast(&mut self, widgets: &mut <Self as Component>::Widgets) {
        if let Some(toast_info) = self.toast_info.take() {
            widgets.toast_overlay.dismiss_all();
            match toast_info {
                ToastInfo::Toast { text, timeout } => {
                    let new_toast = adw::Toast::new(&text);
                    new_toast.set_timeout(match timeout {
                        Some(t) => t.get(),
                        None => 0,
                    });
                    widgets.toast_overlay.add_toast(new_toast);
                }
                ToastInfo::Dismiss => {} // Nothing else to do
            }
        }
    }
}
