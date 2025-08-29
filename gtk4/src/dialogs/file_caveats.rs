use gtk::prelude::{DialogExt, GtkWindowExt, WidgetExt};
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};
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
        dialog = gtk::MessageDialog {
            set_modal: true,
            #[watch]
            set_visible: !model.hidden,
            #[watch]
            set_text: Some("Attention !"),
            #[watch]
            set_secondary_text: Some(&model.generate_secondary_text()),

            add_button: ("Ok", gtk::ResponseType::Ok),
            connect_response[sender] => move |_, _| {
                sender.input(DialogInput::Hide)
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
