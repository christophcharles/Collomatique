use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use std::collections::BTreeSet;
use std::path::PathBuf;

pub struct Dialog {
    hidden: bool,
    move_front: bool,
    path: PathBuf,
    caveats: BTreeSet<collomatique_storage::Caveat>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(PathBuf, BTreeSet<collomatique_storage::Caveat>),
    Hide,
}

impl Dialog {
    fn generate_secondary_text(&self) -> String {
        let mut list = vec!["Certains points nécessitent votre attention.\n".to_string()];

        // The sentences come from `collomatique-ui-text`, the same ones the
        // python module's `str()` on a caveat writes; the bullet is this
        // dialog's own layout.
        list.extend(
            self.caveats
                .iter()
                .map(|caveat| format!("- {}", collomatique_ui_text::caveats::caveat_text(caveat))),
        );

        list.join("\n")
    }
}

#[derive(Debug)]
pub enum DialogOutput {
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
                sender.input(DialogInput::Hide);
                gtk::glib::Propagation::Stop
            },
            add_controller = gtk::EventControllerKey {
                connect_key_pressed[sender] => move |_, key, _, _| {
                    if key == gtk::gdk::Key::Escape {
                        sender.input(DialogInput::Hide);
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
                    set_wrap: true,
                    set_halign: gtk::Align::Center,
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_halign: gtk::Align::Center,

                    gtk::Button {
                        set_size_request: (200, 40),
                        set_label: "Ok",
                        connect_clicked[sender] => move |_| {
                            sender.input(DialogInput::Hide);
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
            path: PathBuf::new(),
            caveats: BTreeSet::new(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.move_front = false;
        match msg {
            DialogInput::Show(path, caveats) => {
                self.path = path;
                self.caveats = caveats;
                self.hidden = false;
                self.move_front = true;
            }
            DialogInput::Hide => {
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
