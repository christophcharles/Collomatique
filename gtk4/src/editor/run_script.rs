use gtk::prelude::{ButtonExt, GtkWindowExt, OrientableExt, TextBufferExt, TextViewExt, WidgetExt};
use relm4::{adw, gtk, Component, ComponentController};
use relm4::{
    ComponentParts, ComponentSender, Controller, RelmWidgetExt, SimpleComponent, WorkerController,
};

use std::path::PathBuf;

mod error_dialog;
mod process_worker;

pub struct Dialog {
    hidden: bool,
    path: PathBuf,
    is_running: bool,
    error_dialog: Controller<error_dialog::Dialog>,
    worker: WorkerController<process_worker::ProcessWorker>,
}

#[derive(Debug)]
pub enum DialogInput {
    Run(PathBuf, String),
    Cancel,
    Accept,

    ScriptFinished,
    StdErr(String),
    StdOut(String),
    Error(String),
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = ();

    view! {
        #[root]
        adw::Window {
            set_modal: true,
            set_size_request: (700, 400),
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Exécution du script Python"),
            add_css_class: "devel",

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        set_label: "Annuler",
                        set_sensitive: true,
                        connect_clicked => DialogInput::Cancel,
                    },
                    pack_end = &gtk::Button {
                        set_label: "Valider les modifications",
                        #[watch]
                        set_sensitive: !model.is_running,
                        add_css_class: "destructive-action",
                        connect_clicked => DialogInput::Accept,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_vexpand: true,
                    adw::Spinner {
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_size_request: (50, 50),
                        #[watch]
                        set_visible: model.is_running,
                    },
                    gtk::Image::from_icon_name("emblem-ok-symbolic") {
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_size_request: (50, 50),
                        set_icon_size: gtk::IconSize::Large,
                        #[watch]
                        set_visible: !model.is_running,
                    },
                    gtk::Label {
                        set_margin_all: 5,
                        set_halign: gtk::Align::Start,
                        set_label: "Opérations effectuées :",
                    },
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                        set_margin_all: 5,
                        gtk::ListBox {
                            set_hexpand: true,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_margin_all: 5,
                                    add_css_class: "success",
                                    set_orientation: gtk::Orientation::Horizontal,
                                    gtk::Image::from_icon_name("emblem-success") {
                                        set_margin_end: 5,
                                    },
                                    gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        set_label: "Test1",
                                    },
                                },
                            },
                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_margin_all: 5,
                                    add_css_class: "warning",
                                    set_orientation: gtk::Orientation::Horizontal,
                                    gtk::Image::from_icon_name("emblem-warning") {
                                        set_margin_end: 5,
                                    },
                                    gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        set_label: "Test2",
                                    },
                                },
                            },
                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_margin_all: 5,
                                    add_css_class: "error",
                                    set_orientation: gtk::Orientation::Horizontal,
                                    gtk::Image::from_icon_name("emblem-error") {
                                        set_margin_end: 5,
                                    },
                                    gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        set_label: "Test3",
                                    },
                                },
                            },
                        }
                    },
                    gtk::Expander {
                        set_margin_all: 5,
                        set_hexpand: true,
                        #[wrap(Some)]
                        set_label_widget = &gtk::Label {
                            set_label: "Informations de débogage",
                        },
                        #[wrap(Some)]
                        set_child = &gtk::ScrolledWindow {
                            set_hexpand: true,
                            set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                            set_margin_all: 5,
                            gtk::TextView {
                                add_css_class: "frame",
                                add_css_class: "osd",
                                set_wrap_mode: gtk::WrapMode::Char,
                                set_editable: false,
                                set_monospace: true,
                                #[wrap(Some)]
                                set_buffer = &gtk::TextBuffer {
                                    set_text: "Test",
                                },
                            }
                        }
                    },
                    gtk::Label {
                        set_margin_all: 5,
                        add_css_class: "dimmed",
                        #[watch]
                        set_label: &model.path.to_string_lossy(),
                    },
                }
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let error_dialog = error_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .detach();

        let worker = process_worker::ProcessWorker::builder()
            .detach_worker(())
            .forward(sender.input_sender(), |msg| match msg {
                process_worker::ProcessWorkerOutput::ScriptFinished => DialogInput::ScriptFinished,
                process_worker::ProcessWorkerOutput::StdErr(content) => {
                    DialogInput::StdErr(content)
                }
                process_worker::ProcessWorkerOutput::StdOut(content) => {
                    DialogInput::StdOut(content)
                }
                process_worker::ProcessWorkerOutput::Error(error) => DialogInput::Error(error),
            });
        let model = Dialog {
            hidden: true,
            path: PathBuf::new(),
            is_running: true,
            error_dialog,
            worker,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Run(path, script) => {
                self.hidden = false;
                self.path = path;
                self.is_running = true;
                self.worker
                    .sender()
                    .send(process_worker::ProcessWorkerInput::RunScript(script))
                    .unwrap();
            }
            DialogInput::Cancel => {
                self.hidden = true;
                self.is_running = false;
                self.worker
                    .sender()
                    .send(process_worker::ProcessWorkerInput::KillScript)
                    .unwrap();
            }
            DialogInput::Accept => {}
            DialogInput::ScriptFinished => {
                self.is_running = false;
            }
            DialogInput::StdErr(_content) => {}
            DialogInput::StdOut(_content) => {}
            DialogInput::Error(error) => {
                self.error_dialog
                    .sender()
                    .send(error_dialog::DialogInput::Show(error))
                    .unwrap();
                self.is_running = false;
            }
        }
    }
}
