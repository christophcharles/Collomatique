use adw::prelude::{EditableExt, PreferencesRowExt};
use gtk::prelude::WidgetExt;
use relm4::FactorySender;
use relm4::adw;
use relm4::factory::FactoryView;
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent};

/// One editable-name row of the naming dialog. [`adw::EntryRow`] has no subtitle, so the
/// (fixed) title carries what the list covers — it stays visible while the name is edited —
/// and the (editable) text is the list name itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Data {
    /// e.g. "Liste pour Maths et Physique (périodes 1 et 2)".
    pub title: String,
    /// The list name, prefilled with the default and freely editable.
    pub name: String,
}

pub struct SpecRow {
    data: Data,
    index: DynamicIndex,
    should_redraw: bool,
}

#[derive(Debug)]
pub enum SpecRowInput {
    UpdateData(Data),
    UpdateName(String),
}

#[derive(Debug)]
pub enum SpecRowOutput {
    /// (spec index, new name)
    NameChanged(usize, String),
}

#[relm4::factory(pub)]
impl FactoryComponent for SpecRow {
    type Init = Data;
    type Input = SpecRowInput;
    type Output = SpecRowOutput;
    type CommandOutput = ();
    type ParentWidget = adw::PreferencesGroup;

    view! {
        #[root]
        adw::EntryRow {
            set_hexpand: true,
            #[watch]
            set_title: &self.data.title,
            #[track(self.should_redraw)]
            set_text: &self.data.name,
            connect_text_notify[sender] => move |widget| {
                let text: String = widget.text().into();
                sender.input(SpecRowInput::UpdateName(text));
            },
        }
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            data,
            index: index.clone(),
            should_redraw: false,
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
        self.should_redraw = false;
        match msg {
            SpecRowInput::UpdateData(new_data) => {
                self.data = new_data;
                self.should_redraw = true;
            }
            SpecRowInput::UpdateName(new_name) => {
                // Breaks the `set_text` -> `text_notify` -> `UpdateName` echo loop.
                if self.data.name == new_name {
                    return;
                }
                self.data.name = new_name.clone();
                sender
                    .output(SpecRowOutput::NameChanged(
                        self.index.current_index(),
                        new_name,
                    ))
                    .unwrap();
            }
        }
    }
}
