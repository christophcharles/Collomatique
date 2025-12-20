use gtk::prelude::{DialogExt, GtkWindowExt, WidgetExt};
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    info: String,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(String),
    ConfirmClicked,
    CancelClicked,
}

#[derive(Debug)]
pub enum DialogOutput {
    Confirmed,
    Cancelled,
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        dialog = gtk::MessageDialog {
            set_modal: true,
            #[watch]
            set_visible: !model.hidden,
            #[watch]
            set_text: Some("Requête du script Python"),
            #[watch]
            set_secondary_text: Some(&model.info),

            add_button: ("Annuler", gtk::ResponseType::Cancel),
            add_button: ("Confirmer", gtk::ResponseType::Accept),
            connect_response[sender] => move |_, response_type| {
                sender.input(
                    match response_type {
                        gtk::ResponseType::Cancel => DialogInput::CancelClicked,
                        gtk::ResponseType::Accept => DialogInput::ConfirmClicked,
                        _ => panic!("Unexpected response type"),
                    }
                )
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
            info: String::new(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Show(info) => {
                self.info = info;
                self.hidden = false;
            }
            DialogInput::ConfirmClicked => {
                self.hidden = true;
                sender.output(DialogOutput::Confirmed).unwrap();
            }
            DialogInput::CancelClicked => {
                self.hidden = true;
                sender.output(DialogOutput::Cancelled).unwrap();
            }
        }
    }
}
