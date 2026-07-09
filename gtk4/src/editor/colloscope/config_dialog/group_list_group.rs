use adw::prelude::PreferencesGroupExt;
use relm4::FactorySender;
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::{adw, gtk};

/// One automatic-group-list entry in the right-hand list of the colloscope config dialog. For now
/// it is just a titled [`adw::PreferencesGroup`]; the per-list recompute controls will be added
/// inside it later.
pub struct GroupListGroup {
    title: String,
}

#[derive(Debug)]
pub enum GroupListGroupInput {
    UpdateTitle(String),
}

#[relm4::factory(pub)]
impl FactoryComponent for GroupListGroup {
    type Init = String;
    type Input = GroupListGroupInput;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        adw::PreferencesGroup {
            #[watch]
            set_title: &self.title,
        }
    }

    fn init_model(title: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { title }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            GroupListGroupInput::UpdateTitle(title) => {
                self.title = title;
            }
        }
    }
}
