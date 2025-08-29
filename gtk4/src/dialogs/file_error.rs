use gtk::prelude::{DialogExt, GtkWindowExt, WidgetExt};
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};
use std::path::PathBuf;

#[derive(Debug)]
pub enum Type {
    Open,
    Save,
}

pub struct Dialog {
    hidden: bool,
    err_type: Type,
    path: PathBuf,
    err_info: String,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(Type, PathBuf, String),
    Hide,
}

impl Dialog {
    fn generate_secondary_text(&self) -> String {
        format!(
            "Erreur à l'{} du fichier {}\n\n{}",
            match self.err_type {
                Type::Open => "ouverture",
                Type::Save => "enregistrement",
            },
            self.path.display(),
            self.err_info
        )
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
            set_text: Some("Erreur !"),
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
            err_type: Type::Open,
            path: PathBuf::new(),
            err_info: String::new(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Show(err_type, path, err_info) => {
                self.path = path;
                self.err_type = err_type;
                self.err_info = err_info;
                self.hidden = false;
            }
            DialogInput::Hide => self.hidden = true,
        }
    }
}
