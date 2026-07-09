use adw::prelude::{PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::WidgetExt;
use relm4::FactorySender;
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::{adw, gtk};

/// State of one period entry in the left-hand list of the colloscope config dialog: its title
/// plus whether it should be recomputed and whether the current colloscope is used as an
/// objective. Nothing is persisted into a `SolveConfig` yet; this only drives the UI.
#[derive(Debug, Clone)]
pub struct Data {
    pub title: String,
    pub recompute: bool,
    pub previous_values_as_objective: bool,
}

pub struct PeriodGroup {
    data: Data,
    index: DynamicIndex,
}

#[derive(Debug)]
pub enum PeriodGroupInput {
    UpdateData(Data),
    RecomputeToggled(bool),
    ObjectiveToggled(bool),
}

#[derive(Debug)]
pub enum PeriodGroupOutput {
    RecomputeToggled(usize, bool),
    ObjectiveToggled(usize, bool),
}

#[relm4::factory(pub)]
impl FactoryComponent for PeriodGroup {
    type Init = Data;
    type Input = PeriodGroupInput;
    type Output = PeriodGroupOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        adw::PreferencesGroup {
            #[watch]
            set_title: &self.data.title,
            #[name(recompute_row)]
            adw::SwitchRow {
                set_title: "Recalculer la période",
                #[track(recompute_row.is_active() != self.data.recompute)]
                #[block_signal(recompute_handler)]
                set_active: self.data.recompute,
                connect_active_notify[sender] => move |widget| {
                    sender.input(PeriodGroupInput::RecomputeToggled(widget.is_active()));
                } @recompute_handler,
            },
            #[name(objective_row)]
            adw::SwitchRow {
                set_title: "Colloscope actuel comme objectif",
                #[watch]
                set_visible: self.data.recompute,
                #[track(objective_row.is_active() != self.data.previous_values_as_objective)]
                #[block_signal(objective_handler)]
                set_active: self.data.previous_values_as_objective,
                connect_active_notify[sender] => move |widget| {
                    sender.input(PeriodGroupInput::ObjectiveToggled(widget.is_active()));
                } @objective_handler,
            },
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
            PeriodGroupInput::UpdateData(data) => {
                self.data = data;
            }
            PeriodGroupInput::RecomputeToggled(value) => {
                sender
                    .output(PeriodGroupOutput::RecomputeToggled(
                        self.index.current_index(),
                        value,
                    ))
                    .unwrap();
            }
            PeriodGroupInput::ObjectiveToggled(value) => {
                sender
                    .output(PeriodGroupOutput::ObjectiveToggled(
                        self.index.current_index(),
                        value,
                    ))
                    .unwrap();
            }
        }
    }
}
