use gtk::prelude::{ButtonExt, GtkWindowExt, TextBufferExt, TextViewExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    text: String,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(String),
    Cancel,
    Accept,
    TextChanged(String),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(String),
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
            set_default_size: (800, 600),
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Modifier le script"),

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        set_label: "Annuler",
                        connect_clicked => DialogInput::Cancel,
                    },
                    pack_end = &gtk::Button {
                        set_label: "Valider",
                        add_css_class: "suggested-action",
                        connect_clicked => DialogInput::Accept,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_margin_all: 5,
                    set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Automatic),
                    gtk::TextView {
                        set_editable: true,
                        set_monospace: true,
                        #[wrap(Some)]
                        set_buffer = &gtk::TextBuffer {
                            #[track(model.should_redraw)]
                            set_text: &model.text,
                            connect_changed[sender] => move |buffer| {
                                let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                                sender.input(DialogInput::TextChanged(text.to_string()));
                            },
                        },
                    }
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Dialog {
            hidden: true,
            should_redraw: false,
            text: String::new(),
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(text) => {
                self.hidden = false;
                self.should_redraw = true;
                self.text = text;
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.text.clone()))
                    .unwrap();
            }
            DialogInput::TextChanged(text) => {
                self.text = text;
            }
        }
    }
}
