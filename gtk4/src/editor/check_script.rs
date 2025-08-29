use gtk::prelude::{ButtonExt, GtkWindowExt, TextBufferExt, TextViewExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
pub struct Dialog {
    hidden: bool,
    text: String,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(String),
    Cancel,
    Run,
}

#[derive(Debug)]
pub enum DialogOutput {
    Run(String),
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        #[root]
        adw::Window {
            set_modal: true,
            set_size_request: (700, 700),
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Exécution du script Python"),

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
                set_content = &gtk::ScrolledWindow {
                    set_hexpand: true,
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
            text: String::new(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Show(text) => {
                self.hidden = false;
                self.text = text;
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Run => {
                self.hidden = true;
                sender.output(DialogOutput::Run(self.text.clone())).unwrap();
            }
        }
    }
}
