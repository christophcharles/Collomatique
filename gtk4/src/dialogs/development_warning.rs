use collomatique_settings::Version;
use gtk::prelude::{BoxExt, ButtonExt, CheckButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    version: Version,
    silence: bool,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(Version),
    Quit,
    Acknowledge,
    SetSilence(bool),
}

#[derive(Debug)]
pub enum DialogOutput {
    Quit,
    /// `Some(version)` when the user asked not to be warned about it again
    Acknowledged(Option<Version>),
}

impl Dialog {
    fn generate_secondary_text(&self) -> String {
        format!(
            "Cette version de Collomatique ({}) est une version de développement.\n\
             Ce n'est pas une version stable.\n\n\
             Des erreurs sont possibles dans la génération des colloscopes, et des données\n\
             peuvent être perdues. Vérifiez systématiquement les colloscopes produits.\n\n\
             <b>Sauvegardez toute donnée importante.</b>",
            self.version
        )
    }
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
            // Closing the box is not a request to close the application, so it
            // acknowledges rather than quits. The titlebar is hidden anyway, so
            // this only ever fires from the window manager.
            connect_close_request[sender] => move |_| {
                sender.input(DialogInput::Acknowledge);
                gtk::glib::Propagation::Stop
            },
            add_controller = gtk::EventControllerKey {
                connect_key_pressed[sender] => move |_, key, _, _| {
                    if key == gtk::gdk::Key::Escape {
                        sender.input(DialogInput::Acknowledge);
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
                    set_label: "<big><b>Attention !</b></big>",
                    set_use_markup: true,
                    set_halign: gtk::Align::Center,
                },

                gtk::Label {
                    #[watch]
                    set_label: &model.generate_secondary_text(),
                    set_use_markup: true,
                    set_wrap: true,
                    set_halign: gtk::Align::Center,
                },

                // « pour cette version » is not padding: acknowledging one
                // development version says nothing about the next one, and the
                // label is where the user finds that out.
                gtk::CheckButton {
                    set_label: Some("Ne plus afficher pour cette version"),
                    set_halign: gtk::Align::Center,
                    #[watch]
                    set_active: model.silence,
                    connect_toggled[sender] => move |check| {
                        sender.input(DialogInput::SetSilence(check.is_active()));
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 10,
                    set_halign: gtk::Align::Center,

                    // Neither button is styled: nothing here is destructive, and
                    // marking « Compris » as the suggested action would wave the
                    // user past a warning that exists not to be waved past.
                    gtk::Button {
                        set_size_request: (200, 40),
                        set_label: "Quitter",
                        connect_clicked[sender] => move |_| {
                            sender.input(DialogInput::Quit);
                        },
                    },

                    gtk::Button {
                        set_size_request: (200, 40),
                        set_label: "Compris",
                        connect_clicked[sender] => move |_| {
                            sender.input(DialogInput::Acknowledge);
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
            version: Version::new(0, 0, 0),
            silence: false,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Show(version) => {
                self.version = version;
                self.silence = false;
                self.hidden = false;
            }
            DialogInput::SetSilence(silence) => self.silence = silence,
            DialogInput::Acknowledge => {
                self.hidden = true;
                let silenced = self.silence.then(|| self.version.clone());
                sender.output(DialogOutput::Acknowledged(silenced)).unwrap()
            }
            // Quitting records nothing, even with the box ticked: the user
            // never got past the warning.
            DialogInput::Quit => {
                self.hidden = true;
                sender.output(DialogOutput::Quit).unwrap()
            }
        }
    }
}
