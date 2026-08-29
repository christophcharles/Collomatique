use adw::prelude::{PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{OrientableExt, WidgetExt};
use relm4::FactorySender;
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::{adw, gtk};

/// State of one automatic-group-list entry in the right-hand list of the colloscope config
/// dialog: its title plus whether it should be recomputed and whether the current list is used
/// as an objective. This state is read back into the parent's `SolveConfig` via its
/// `config_from_data`.
#[derive(Debug, Clone)]
pub struct Data {
    pub title: String,
    pub recompute: bool,
    pub previous_values_as_objective: bool,
}

/// Plain-French sentence naming the concrete outcome of the switches. Unlike periods, a fixed
/// group list has no "take current values into account" axis, so there are only three cases.
fn summary(data: &Data) -> &'static str {
    match (data.recompute, data.previous_values_as_objective) {
        (true, true) => "Recalculée en restant proche de la liste actuelle.",
        (true, false) => "Recalculée sans référence à la liste actuelle.",
        (false, _) => "Figée ; la liste actuelle est maintenue.",
    }
}

pub struct GroupListGroup {
    data: Data,
    index: DynamicIndex,
}

#[derive(Debug)]
pub enum GroupListGroupInput {
    UpdateData(Data),
    RecomputeToggled(bool),
    ObjectiveToggled(bool),
}

#[derive(Debug)]
pub enum GroupListGroupOutput {
    RecomputeToggled(usize, bool),
    ObjectiveToggled(usize, bool),
}

#[relm4::factory(pub)]
impl FactoryComponent for GroupListGroup {
    type Init = Data;
    type Input = GroupListGroupInput;
    type Output = GroupListGroupOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            adw::PreferencesGroup {
                #[watch]
                set_title: &self.data.title,
                #[name(recompute_row)]
                adw::SwitchRow {
                    set_title: "Recalculer la liste",
                    #[track(recompute_row.is_active() != self.data.recompute)]
                    #[block_signal(recompute_handler)]
                    set_active: self.data.recompute,
                    connect_active_notify[sender] => move |widget| {
                        sender.input(GroupListGroupInput::RecomputeToggled(widget.is_active()));
                    } @recompute_handler,
                },
                #[name(objective_row)]
                adw::SwitchRow {
                    set_title: "Liste actuelle comme objectif",
                    #[watch]
                    set_visible: self.data.recompute,
                    #[track(objective_row.is_active() != self.data.previous_values_as_objective)]
                    #[block_signal(objective_handler)]
                    set_active: self.data.previous_values_as_objective,
                    connect_active_notify[sender] => move |widget| {
                        sender.input(GroupListGroupInput::ObjectiveToggled(widget.is_active()));
                    } @objective_handler,
                },
            },
            gtk::Label {
                #[watch]
                set_label: summary(&self.data),
                add_css_class: "dim-label",
                set_wrap: true,
                set_xalign: 0.0,
                set_margin_start: 12,
                set_margin_end: 12,
                set_margin_top: 6,
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
            GroupListGroupInput::UpdateData(data) => {
                self.data = data;
            }
            GroupListGroupInput::RecomputeToggled(value) => {
                sender
                    .output(GroupListGroupOutput::RecomputeToggled(
                        self.index.current_index(),
                        value,
                    ))
                    .unwrap();
            }
            GroupListGroupInput::ObjectiveToggled(value) => {
                sender
                    .output(GroupListGroupOutput::ObjectiveToggled(
                        self.index.current_index(),
                        value,
                    ))
                    .unwrap();
            }
        }
    }
}
