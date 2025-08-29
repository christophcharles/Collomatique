use gtk::prelude::{GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk, ComponentParts, ComponentSender, SimpleComponent};

pub struct AppModel {}

#[derive(Debug)]
pub enum AppInput {}

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
                gtk::Label {
                    set_vexpand: true,
                    set_label: "Stub",
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AppModel {};
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, _message: Self::Input, _sender: ComponentSender<Self>) {}
}
