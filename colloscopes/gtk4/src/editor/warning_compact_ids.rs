use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    move_front: bool,
}

#[derive(Debug)]
pub enum DialogInput {
    Show,
    Compact,
    Cancel,
}

/// Only the confirmation travels.
///
/// Unlike [super::warning_save_ids], no payload waits in the editor while this
/// dialog is up, so a cancel has nothing to clean up.
#[derive(Debug)]
pub enum DialogOutput {
    Compact,
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
                    set_label: "<big><b>Compacter les identifiants</b></big>",
                    set_use_markup: true,
                    set_halign: gtk::Align::Center,
                },

                gtk::Label {
                    set_label: "Chaque élément du document (matière, colleur, élève, créneau…) porte un identifiant interne.\nLes suppressions y laissent des trous, que le compactage referme.",
                    set_wrap: true,
                    set_halign: gtk::Align::Start,
                },

                gtk::Label {
                    set_label: "Le compactage renumérote tous les identifiants internes du document.\nLe contenu du colloscope (matières, colleurs, élèves, créneaux…) ne sera pas modifié.\nPar contre l'historique des modifications (annuler/rétablir) est <b>perdu</b>.\nLe document n'est pas enregistré : il faudra l'enregistrer ensuite pour conserver le résultat.",
                    set_use_markup: true,
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
                        set_label: "Compacter",
                        add_css_class: "warning",
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
        let model = Dialog {
            hidden: true,
            move_front: false,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.move_front = false;
        match msg {
            DialogInput::Show => {
                self.hidden = false;
                self.move_front = true;
            }
            DialogInput::Compact => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                    sender.output(DialogOutput::Compact).unwrap()
                }
            }
            // Nothing is waiting on an answer in the editor: closing is enough.
            DialogInput::Cancel => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                }
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.dialog.present();
        }
    }
}
