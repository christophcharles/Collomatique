use gtk::prelude::{OrientableExt, WidgetExt};
use relm4::FactorySender;
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent, RelmWidgetExt};

#[derive(Debug)]
pub struct Entry {
    message: String,
}

#[relm4::factory(pub)]
impl FactoryComponent for Entry {
    type Init = String;
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        root_widget = gtk::Box {
            set_margin_all: 5,
            set_orientation: gtk::Orientation::Horizontal,
            add_css_class: "error",
            gtk::Image {
                set_margin_end: 5,
                set_icon_name: Some("dialog-error-symbolic"),
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_label: &self.message,
            },
        },
    }

    fn init_model(
        message: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        Self { message }
    }

    fn update(&mut self, _msg: Self::Input, _sender: FactorySender<Self>) {}
}
