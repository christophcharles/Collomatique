use gtk::prelude::{
    BoxExt, ButtonExt, EntryBufferExt, EntryBufferExtManual, EntryExt, GtkWindowExt, WidgetExt,
};
use relm4::gtk::prelude::OrientableExt;
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    info_text: String,
    placeholder_text: String,
    entry: String,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(String, String),
    Cancel,
    Accept,
    UpdateEntry(String),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(String),
    Cancelled,
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
            set_resizable: false,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Requête du script Python"),
            set_size_request: (-1, -1),

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
                set_content = &gtk::Box {
                    set_hexpand: true,
                    set_margin_all: 10,
                    set_spacing: 15,
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Label {
                        set_hexpand: true,
                        set_halign: gtk::Align::Start,
                        #[track(model.should_redraw)]
                        set_label: &model.info_text,
                    },
                    #[name(entry)]
                    gtk::Entry {
                        #[track(model.should_redraw)]
                        set_placeholder_text: Some(&model.placeholder_text),
                        set_buffer = &gtk::EntryBuffer {
                            #[track(model.should_redraw)]
                            set_text: &model.entry,
                            connect_text_notify[sender] => move |widget| {
                                let text : String = widget.text().into();
                                sender.input(DialogInput::UpdateEntry(text));
                            },
                        },
                        connect_activate[sender] => move |_widget| {
                            sender.input(DialogInput::Accept);
                        },
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
            should_redraw: false,
            info_text: String::new(),
            placeholder_text: String::new(),
            entry: String::new(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(info_text, placeholder_text) => {
                self.hidden = false;
                self.should_redraw = true;
                self.info_text = info_text;
                self.placeholder_text = placeholder_text;
                self.entry = String::new();
            }
            DialogInput::Cancel => {
                self.hidden = true;
                sender.output(DialogOutput::Cancelled).unwrap();
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.entry.clone()))
                    .unwrap();
            }
            DialogInput::UpdateEntry(entry) => {
                self.entry = entry;
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.should_redraw {
            widgets.entry.grab_focus();
        }
    }
}
