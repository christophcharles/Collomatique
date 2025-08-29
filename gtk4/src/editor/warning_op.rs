use gtk::prelude::{DialogExt, GtkWindowExt, WidgetExt};
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    warnings: Vec<String>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(Vec<String>),
    Continue,
    Cancel,
}

#[derive(Debug)]
pub enum DialogOutput {
    Continue,
}

impl Dialog {
    fn generate_secondary_text(&self) -> String {
        let mut output = String::from(
            "L'opération est potentiellement destructive et aura les conséquences suivantes :",
        );

        for warning in &self.warnings {
            output += "\n - ";
            output += &warning;
        }

        output
    }
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
            set_text: Some("Attention !"),
            #[watch]
            set_secondary_text: Some(&model.generate_secondary_text()),
            add_button: ("Poursuivre", gtk::ResponseType::Accept),
            add_button: ("Annuler", gtk::ResponseType::Cancel),
            connect_response[sender] => move |_, resp| {
                sender.input(if resp == gtk::ResponseType::Accept {
                    DialogInput::Continue
                } else {
                    DialogInput::Cancel
                })
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
            warnings: vec![],
        };
        let widgets = view_output!();

        let accept_widget = widgets
            .dialog
            .widget_for_response(gtk::ResponseType::Accept)
            .expect("There should be an accept button");
        accept_widget.add_css_class("destructive-action");

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Show(warnings) => {
                self.hidden = false;
                self.warnings = warnings;
            }
            DialogInput::Continue => {
                self.hidden = true;
                sender.output(DialogOutput::Continue).unwrap()
            }
            DialogInput::Cancel => self.hidden = true,
        }
    }
}
