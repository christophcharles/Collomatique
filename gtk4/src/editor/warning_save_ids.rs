use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
}

#[derive(Debug)]
pub enum DialogInput {
    Show,
    Compact,
    Cancel,
}

#[derive(Debug)]
pub enum DialogOutput {
    Compact,
    Cancel,
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
                set_size_request: (550, -1),

                gtk::Label {
                    set_label: "<big><b>Enregistrement impossible</b></big>",
                    set_use_markup: true,
                    set_halign: gtk::Align::Center,
                },

                gtk::Label {
                    set_label: "Le document ne peut pas être enregistré tel quel : ses identifiants internes dépassent la capacité du format de fichier.",
                    set_wrap: true,
                    set_halign: gtk::Align::Start,
                },

                gtk::Label {
                    set_label: "Le compactage renumérote tous les identifiants internes puis enregistre le fichier. Le contenu du colloscope (matières, colleurs, élèves, créneaux…) n'est pas modifié.",
                    set_wrap: true,
                    set_halign: gtk::Align::Start,
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
                        set_label: "Compacter et enregistrer",
                        // Wired in the next commit; shown inert so the
                        // rescue path is visible but cannot yet run.
                        set_sensitive: false,
                        connect_clicked[sender] => move |_| {
                            sender.input(DialogInput::Compact);
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
            DialogInput::Show => {
                self.hidden = false;
            }
            DialogInput::Compact => {
                self.hidden = true;
                sender.output(DialogOutput::Compact).unwrap()
            }
            DialogInput::Cancel => {
                self.hidden = true;
                sender.output(DialogOutput::Cancel).unwrap()
            }
        }
    }
}
