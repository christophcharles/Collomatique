use gtk::prelude::{ButtonExt, GtkWindowExt, OrientableExt, TextBufferExt, TextViewExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

use std::path::PathBuf;

pub struct Dialog {
    hidden: bool,
    path: PathBuf,
}

#[derive(Debug)]
pub enum DialogInput {
    Run(PathBuf, String),
    Close,
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
                    pack_end = &gtk::Button {
                        set_label: "Fermer",
                        set_sensitive: true,
                        add_css_class: "suggested-action",
                        connect_clicked => DialogInput::Close,
                    },
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
                        set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
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
        let model = Dialog {
            hidden: true,
            path: PathBuf::new(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Run(path, _script) => {
                self.hidden = false;
                self.path = path;
            }
            DialogInput::Close => {
                self.hidden = true;
            }
        }
    }
}
