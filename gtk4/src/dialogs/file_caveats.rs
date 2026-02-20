use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use std::collections::BTreeSet;
use std::path::PathBuf;

pub struct Dialog {
    hidden: bool,
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
        let mut list = vec![
            "Il est préférable d'utiliser une version plus récente de Collomatique.\n".to_string(),
        ];

        use collomatique_storage::Caveat;
        list.extend(self.caveats.iter().map(|caveat| match caveat {
            Caveat::UnknownEntries => {
                "- Certaines entrées (non-indispensables) n'ont pas pu être décodées".to_string()
            }
            Caveat::CreatedWithNewerVersion(version) => format!(
                "- Fichier généré avec la version {} de Collomatique",
                version
            ),
        }));

        list.join("\n")
    }
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = ();

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
            path: PathBuf::new(),
            caveats: BTreeSet::new(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Show(path, caveats) => {
                self.path = path;
                self.caveats = caveats;
                self.hidden = false;
            }
            DialogInput::Hide => self.hidden = true,
        }
    }
}
