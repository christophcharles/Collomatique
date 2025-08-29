use gtk::prelude::{DialogExt, GtkWindowExt, WidgetExt};
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    error_msg: String,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(String),
    Hide,
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
            set_text: Some("L'opération ne peut être effectuée"),
            #[watch]
            set_secondary_text: Some(&model.error_msg),

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
            error_msg: String::new(),
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Show(text) => {
                self.hidden = false;
                self.error_msg = text;
            }
            DialogInput::Hide => self.hidden = true,
        }
    }
}
