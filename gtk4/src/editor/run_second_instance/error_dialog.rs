use gtk::prelude::{DialogExt, GtkWindowExt, WidgetExt};
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    err_info: String,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(String),
    Hide,
}

impl Dialog {
    fn generate_secondary_text(&self) -> String {
        format!(
            "Erreur pendant l'exécution du script Python\n\n{}",
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
            err_info: String::new(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Show(err_info) => {
                self.err_info = err_info;
                self.hidden = false;
            }
            DialogInput::Hide => self.hidden = true,
        }
    }
}
