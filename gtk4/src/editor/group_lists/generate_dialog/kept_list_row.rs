use adw::prelude::{ActionRowExt, PreferencesRowExt};
use relm4::FactorySender;
use relm4::adw;
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent};

use collomatique_state_colloscopes::GroupListId;

/// One existing prefilled list in the right panel: whether its pairings should count as
/// already-shared. Carries its own id so the parent reads the request back directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Data {
    pub group_list_id: GroupListId,
    pub title: String,
    /// e.g. "4 groupes, 15 élèves" — enough to tell two lists apart at a glance.
    pub subtitle: String,
    pub keep: bool,
}

pub struct KeptListRow {
    data: Data,
    index: DynamicIndex,
}

#[derive(Debug)]
pub enum KeptListRowInput {
    UpdateData(Data),
    Toggled(bool),
}

#[derive(Debug)]
pub enum KeptListRowOutput {
    /// (prefilled-list index, new value)
    Toggled(usize, bool),
}

#[relm4::factory(pub)]
impl FactoryComponent for KeptListRow {
    type Init = Data;
    type Input = KeptListRowInput;
    type Output = KeptListRowOutput;
    type CommandOutput = ();
    type ParentWidget = adw::PreferencesGroup;

    view! {
        #[root]
        #[name(switch_row)]
        adw::SwitchRow {
            #[watch]
            set_title: &self.data.title,
            #[watch]
            set_subtitle: &self.data.subtitle,
            #[track(switch_row.is_active() != self.data.keep)]
            #[block_signal(keep_handler)]
            set_active: self.data.keep,
            connect_active_notify[sender] => move |widget| {
                sender.input(KeptListRowInput::Toggled(widget.is_active()));
            } @keep_handler,
        }
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            data,
            index: index.clone(),
        }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            KeptListRowInput::UpdateData(data) => {
                self.data = data;
            }
            KeptListRowInput::Toggled(value) => {
                sender
                    .output(KeptListRowOutput::Toggled(
                        self.index.current_index(),
                        value,
                    ))
                    .unwrap();
            }
        }
    }
}
