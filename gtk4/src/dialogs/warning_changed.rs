use gtk::prelude::{DialogExt, GtkWindowExt, WidgetExt};
use relm4::gtk;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};

pub struct Dialog {
    hidden: bool,
}

#[derive(Debug)]
pub enum DialogInput {
    Show,
    Accept,
    Cancel,
}

#[derive(Debug)]
pub enum DialogOutput {
    Accept,
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
            set_text: Some("Abandonner les modifications ?"),
            set_secondary_text: Some("Toutes les modifications sur le colloscope seront perdues."),
            add_button: ("Abandonner", gtk::ResponseType::Accept),
            add_button: ("Annuler", gtk::ResponseType::Cancel),
            connect_response[sender] => move |_, resp| {
                sender.input(if resp == gtk::ResponseType::Accept {
                    DialogInput::Accept
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
        let model = Dialog { hidden: true };
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
            DialogInput::Show => self.hidden = false,
            DialogInput::Accept => {
                self.hidden = true;
                sender.output(DialogOutput::Accept).unwrap()
            }
            DialogInput::Cancel => self.hidden = true,
        }
    }
}
