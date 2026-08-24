use relm4::FactorySender;
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent, RelmWidgetExt};

#[derive(Debug)]
pub struct WarningIcon {
    message: String,
}

#[relm4::factory(pub)]
impl FactoryComponent for WarningIcon {
    type Init = String;
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Image {
            set_icon_name: Some("dialog-warning-symbolic"),
            set_tooltip: &self.message,
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
