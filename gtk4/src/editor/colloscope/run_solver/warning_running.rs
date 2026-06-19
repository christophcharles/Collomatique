use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
}

#[derive(Debug)]
pub enum DialogInput {
    Show,
    Accept,
    Cancel,
}

#[derive(Debug)]
pub enum DialogOutput {
    Accept,
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        dialog = gtk::Window {
            set_modal: true,
            set_resizable: false,
            #[watch]
            set_visible: !model.hidden,
            connect_close_request[sender] => move |_| {
                sender.input(DialogInput::Cancel);
                gtk::glib::Propagation::Stop
            },
            add_controller = gtk::EventControllerKey {
                connect_key_pressed[sender] => move |_, key, _, _| {
                    if key == gtk::gdk::Key::Escape {
                        sender.input(DialogInput::Cancel);
                        gtk::glib::Propagation::Stop
                    } else {
                        gtk::glib::Propagation::Proceed
                    }
                }
            },
            #[wrap(Some)]
            set_titlebar = &gtk::Box {
                set_visible: false,
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 20,
                set_margin_all: 25,

                gtk::Label {
                    set_label: "<big><b>Interrompre l'exécution du script ?</b></big>",
                    set_use_markup: true,
                    set_halign: gtk::Align::Center,
                },

                gtk::Label {
                    set_label: "Toutes les modifications sur le colloscope seront perdues.",
                    set_wrap: true,
                    set_halign: gtk::Align::Center,
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 10,
                    set_halign: gtk::Align::Center,

                    gtk::Button {
                        set_size_request: (200, 40),
                        set_label: "Annuler",
                        connect_clicked[sender] => move |_| {
                            sender.input(DialogInput::Cancel);
                        },
                    },

                    gtk::Button {
                        set_size_request: (200, 40),
                        set_label: "Interrompre",
                        add_css_class: "destructive-action",
                        connect_clicked[sender] => move |_| {
                            sender.input(DialogInput::Accept);
                        },
                    },
                },
            },
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Dialog { hidden: true };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Show => self.hidden = false,
            DialogInput::Accept => {
                self.hidden = true;
                sender.output(DialogOutput::Accept).unwrap()
            }
            DialogInput::Cancel => self.hidden = true,
        }
    }
}
