use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk, ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct AppModel {
    file_opened: bool,
}

#[derive(Debug)]
pub enum AppInput {
    OpenNewColloscope,
    OpenExistingColloscope,
}

pub struct AppWidgets {}

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Input = AppInput;
    type Output = ();
    type Init = ();

    view! {
        #[root]
        root_window = adw::ApplicationWindow {
            set_default_width: 800,
            set_default_height: 600,
            set_title: Some("Collomatique"),
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                adw::HeaderBar::new(),
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 5,
                    set_spacing: 5,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_vexpand: true,

                    #[watch]
                    set_visible: !model.file_opened,

                    gtk::Button::with_label("Commencer un nouveau colloscope") {
                        set_margin_all: 5,
                        add_css_class: "suggested-action",
                        connect_clicked => AppInput::OpenNewColloscope,
                    },
                    gtk::Button::with_label("Ouvrir un colloscope existant") {
                        set_margin_all: 5,
                        add_css_class: "suggested-action",
                        connect_clicked => AppInput::OpenExistingColloscope,
                    },
                }
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AppModel { file_opened: false };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppInput::OpenNewColloscope => {
                self.file_opened = true;
            }
            AppInput::OpenExistingColloscope => {
                // Ignore for now
            }
        }
    }
}
