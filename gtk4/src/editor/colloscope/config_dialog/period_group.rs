use adw::prelude::{PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{OrientableExt, WidgetExt};
use relm4::FactorySender;
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::{adw, gtk};

/// State of one period entry in the left-hand list of the colloscope config dialog: its title
/// plus whether it should be recomputed and whether the current colloscope's values are taken
/// into account in the resolution. Those are two orthogonal choices: when recomputing, the
/// current values act as a soft objective (stay close); when fixed, they feed the balancing
/// constraints across periods. Nothing is persisted into a `SolveConfig` yet; this only drives
/// the UI.
#[derive(Debug, Clone)]
pub struct Data {
    pub title: String,
    pub recompute: bool,
    pub use_current_values: bool,
}

/// Plain-French sentence naming the concrete outcome of the two switches, so the "ignored"
/// state (both off) is spelled out rather than left as an emergent combination.
fn summary(data: &Data) -> &'static str {
    match (data.recompute, data.use_current_values) {
        (true, true) => "Recalculée en restant proche du colloscope actuel.",
        (true, false) => "Recalculée sans référence au colloscope actuel.",
        (false, true) => "Figée ; ses valeurs servent à équilibrer les autres périodes.",
        (false, false) => "Ignorée : absente du calcul.",
    }
}

pub struct PeriodGroup {
    data: Data,
    index: DynamicIndex,
}

#[derive(Debug)]
pub enum PeriodGroupInput {
    UpdateData(Data),
    RecomputeToggled(bool),
    UseCurrentToggled(bool),
}

#[derive(Debug)]
pub enum PeriodGroupOutput {
    RecomputeToggled(usize, bool),
    UseCurrentToggled(usize, bool),
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
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
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
                #[name(use_current_row)]
                adw::SwitchRow {
                    set_title: "Tenir compte du colloscope actuel",
                    #[track(use_current_row.is_active() != self.data.use_current_values)]
                    #[block_signal(use_current_handler)]
                    set_active: self.data.use_current_values,
                    connect_active_notify[sender] => move |widget| {
                        sender.input(PeriodGroupInput::UseCurrentToggled(widget.is_active()));
                    } @use_current_handler,
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
            PeriodGroupInput::UseCurrentToggled(value) => {
                sender
                    .output(PeriodGroupOutput::UseCurrentToggled(
                        self.index.current_index(),
                        value,
                    ))
                    .unwrap();
            }
        }
    }
}
