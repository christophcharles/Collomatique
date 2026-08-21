use gtk::prelude::{ButtonExt, GtkWindowExt, OrientableExt, TextBufferExt, TextViewExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use std::path::PathBuf;

pub struct Dialog {
    hidden: bool,
    move_front: bool,
    path: PathBuf,
    text: String,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(PathBuf, String),
    Cancel,
    Run,
}

#[derive(Debug)]
pub enum DialogOutput {
    Run(PathBuf, String),
    /// The dialog just closed: whoever owns the window underneath should bring
    /// it back to the front, because Windows will not do it on its own.
    PresentParent,
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        #[root]
        root_window = adw::Window {
            set_modal: true,
            set_default_size: (700, 700),
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Exécuter un script Python"),

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        set_label: "Annuler",
                        connect_clicked => DialogInput::Cancel,
                    },
                    pack_end = &gtk::Button {
                        set_label: "Exécuter",
                        add_css_class: "destructive-action",
                        connect_clicked => DialogInput::Run,
                    },
                },
                add_top_bar = &adw::Banner {
                    set_title: "N'exécutez pas de scripts d'origine inconnue !",
                    set_revealed: true,
                },
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_vexpand: true,
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_margin_all: 5,
                        set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Automatic),
                        gtk::TextView {
                            set_editable: false,
                            set_monospace: true,
                            #[wrap(Some)]
                            set_buffer = &gtk::TextBuffer {
                                #[watch]
                                set_text: &model.text,
                            },
                        }
                    },
                    gtk::Label {
                        set_margin_all: 5,
                        add_css_class: "dimmed",
                        #[watch]
                        set_label: &model.path.to_string_lossy(),
                    },
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Dialog {
            hidden: true,
            move_front: false,
            path: PathBuf::new(),
            text: String::new(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.move_front = false;
        match msg {
            DialogInput::Show(path, text) => {
                self.hidden = false;
                self.move_front = true;
                self.path = path;
                self.text = text;
            }
            DialogInput::Cancel => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                }
            }
            DialogInput::Run => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                    sender
                        .output(DialogOutput::Run(self.path.clone(), self.text.clone()))
                        .unwrap();
                }
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
    }
}
