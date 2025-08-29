use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::RelmWidgetExt;
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, SimpleComponent};

pub struct WelcomePanel {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WelcomeMessage {
    OpenNewColloscope,
    OpenExistingColloscope,
}

#[relm4::component(pub)]
impl SimpleComponent for WelcomePanel {
    type Input = WelcomeMessage;
    type Output = WelcomeMessage;
    type Init = ();

    view! {
        #[root]
        adw::ToolbarView {
            add_top_bar: &adw::HeaderBar::new(),
            #[wrap(Some)]
            set_content = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 5,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
                set_hexpand: true,
                set_vexpand: true,

                gtk::Button::with_label("Commencer un nouveau colloscope") {
                    set_margin_all: 5,
                    add_css_class: "suggested-action",
                    connect_clicked => WelcomeMessage::OpenNewColloscope,
                },
                gtk::Button::with_label("Ouvrir un colloscope existant") {
                    set_margin_all: 5,
                    add_css_class: "suggested-action",
                    connect_clicked => WelcomeMessage::OpenExistingColloscope,
                },
            },
        },
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = WelcomePanel {};
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        sender.output(message).unwrap();
    }
}
