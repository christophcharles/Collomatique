use std::time::{Duration, Instant};

use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, gtk};

use collomatique_strategies::{IncrementalProgressData, NoObjectiveSolveProgress};

/// How many of the most recent epoch durations feed the remaining-time estimate. The estimate
/// stays `-` until at least this many epochs have completed.
const REMAINING_ESTIMATE_WINDOW: usize = 5;

#[derive(Debug)]
pub enum IncrementalPanelInput {
    /// Sent on (re)assignment: `visible` is whether the incremental strategy is the one now
    /// running on the worker. Also clears retained state to its fresh state.
    Reset {
        visible: bool,
    },
    Update(IncrementalProgressData),
    /// Periodic tick: re-render so the live step timer / remaining-time labels recompute.
    Refresh,
}

/// The step currently in progress: either one of the staggered epochs (0-based `seq`) or the
/// final reconstruction ("calcul du coût"). Used to detect step transitions so epoch durations
/// can be finalized and the per-step timer restarted.
#[derive(Clone, Copy, PartialEq)]
enum Step {
    Epoch(usize),
    Reconstruction,
}

pub struct IncrementalPanel {
    visible: bool,
    // Most recent message; drives the "Étape" label (Reconstruction/Done → cost computation).
    last: Option<IncrementalProgressData>,
    // (epoch, total, var_count) from the most recent EpochStarted/EpochSolve; kept so the epoch
    // label survives the Reconstruction phase (which carries no epoch/var_count).
    last_epoch: Option<(usize, usize, usize)>,
    // Last sub-solve statistics (from EpochSolve/Reconstruction); retained so the terminal `Done`
    // event does not zero the node/solution counts.
    last_progress: Option<NoObjectiveSolveProgress>,
    // True objective on the original model, delivered by the terminal `Done` event.
    final_cost: Option<f64>,
    // Per-epoch new-variable count, indexed by epoch seq; summed for the running total.
    epoch_var_counts: Vec<usize>,
    // The step currently in progress, for transition detection.
    current_step: Option<Step>,
    // When the current step started, for the per-step timer.
    step_start: Option<Instant>,
    // Wall-clock durations of completed epochs, for the rolling remaining-time estimate.
    epoch_durations: Vec<Duration>,
}

#[relm4::component(pub)]
impl SimpleComponent for IncrementalPanel {
    type Init = ();
    type Input = IncrementalPanelInput;
    type Output = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_margin_all: 5,
            set_hexpand: true,
            set_vexpand: true,
            set_halign: gtk::Align::Center,
            set_valign: gtk::Align::Center,
            set_spacing: 5,
            #[watch]
            set_visible: model.visible,
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                gtk::Label {
                    set_label: "Étape : ",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
                gtk::Label {
                    #[watch]
                    set_label: &model.step(),
                },
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                gtk::Label {
                    set_label: "Variables ajoutées : ",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
                gtk::Label {
                    #[watch]
                    set_label: &model.var_count(),
                },
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                gtk::Label {
                    set_label: "Coût obtenu : ",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
                gtk::Label {
                    #[watch]
                    set_label: &model.cost(),
                },
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                gtk::Label {
                    set_label: "Nœuds explorés : ",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
                gtk::Label {
                    #[watch]
                    set_label: &model.node_count(),
                },
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                gtk::Label {
                    set_label: "Solutions trouvées : ",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
                gtk::Label {
                    #[watch]
                    set_label: &model.solutions_found(),
                },
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                gtk::Label {
                    set_label: "Durée de l'étape : ",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
                gtk::Label {
                    #[watch]
                    set_label: &model.step_duration(),
                },
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                gtk::Label {
                    set_label: "Temps restant estimé : ",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
                gtk::Label {
                    #[watch]
                    set_label: &model.remaining_estimate(),
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = IncrementalPanel {
            visible: false,
            last: None,
            last_epoch: None,
            last_progress: None,
            final_cost: None,
            epoch_var_counts: Vec::new(),
            current_step: None,
            step_start: None,
            epoch_durations: Vec::new(),
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            IncrementalPanelInput::Reset { visible } => {
                self.visible = visible;
                self.last = None;
                self.last_epoch = None;
                self.last_progress = None;
                self.final_cost = None;
                self.epoch_var_counts.clear();
                self.current_step = None;
                self.step_start = None;
                self.epoch_durations.clear();
            }
            IncrementalPanelInput::Update(data) => {
                let step = match &data {
                    IncrementalProgressData::EpochStarted {
                        epoch,
                        total,
                        var_count,
                    }
                    | IncrementalProgressData::EpochSolve {
                        epoch,
                        total,
                        var_count,
                        ..
                    } => {
                        self.last_epoch = Some((*epoch, *total, *var_count));
                        if self.epoch_var_counts.len() <= *epoch {
                            self.epoch_var_counts.resize(*epoch + 1, 0);
                        }
                        self.epoch_var_counts[*epoch] = *var_count;
                        if let IncrementalProgressData::EpochSolve { progress, .. } = &data {
                            self.last_progress = Some(progress.clone());
                        }
                        Step::Epoch(*epoch)
                    }
                    IncrementalProgressData::Reconstruction { progress, .. } => {
                        self.last_progress = Some(progress.clone());
                        Step::Reconstruction
                    }
                    IncrementalProgressData::Done { objective, .. } => {
                        self.final_cost = *objective;
                        Step::Reconstruction
                    }
                };
                if self.current_step != Some(step) {
                    // Finalize the epoch we are leaving so its wall-clock feeds the estimate.
                    if let (Some(Step::Epoch(_)), Some(start)) =
                        (self.current_step, self.step_start)
                    {
                        self.epoch_durations.push(start.elapsed());
                    }
                    self.current_step = Some(step);
                    self.step_start = Some(Instant::now());
                }
                self.last = Some(data);
            }
            // Running the handler is enough to recompute the `#[watch]` timer/estimate labels.
            IncrementalPanelInput::Refresh => {}
        }
    }
}

impl IncrementalPanel {
    fn step(&self) -> String {
        match &self.last {
            Some(IncrementalProgressData::Reconstruction { total, .. })
            | Some(IncrementalProgressData::Done { total, .. }) => {
                format!("{}/{} (calcul du coût)", total + 1, total + 1)
            }
            _ => match self.last_epoch {
                Some((epoch, total, _)) => {
                    format!("{}/{} (époque {})", epoch + 1, total + 1, epoch + 1)
                }
                None => "-".to_owned(),
            },
        }
    }

    fn var_count(&self) -> String {
        match self.last_epoch {
            Some((epoch, _, var_count)) => {
                let total: usize = self.epoch_var_counts.iter().take(epoch + 1).sum();
                format!("{var_count} (total : {total})")
            }
            None => "-".to_owned(),
        }
    }

    fn cost(&self) -> String {
        match self.final_cost {
            Some(obj) => format!("{obj:.1}"),
            None => "-".to_owned(),
        }
    }

    fn node_count(&self) -> String {
        match &self.last_progress {
            Some(p) => format!("{}", p.node_count),
            None => "0".to_owned(),
        }
    }

    fn solutions_found(&self) -> String {
        match &self.last_progress {
            Some(p) => format!("{}", p.solutions_found),
            None => "0".to_owned(),
        }
    }

    fn step_duration(&self) -> String {
        match self.step_start {
            Some(start) => super::super::format_elapsed(start.elapsed()),
            None => "-".to_owned(),
        }
    }

    fn remaining_estimate(&self) -> String {
        match self.current_step {
            Some(Step::Reconstruction) => "presque fini...".to_owned(),
            Some(Step::Epoch(seq)) => {
                if self.epoch_durations.len() < REMAINING_ESTIMATE_WINDOW {
                    return "-".to_owned();
                }
                let total = match self.last_epoch {
                    Some((_, total, _)) => total,
                    None => return "-".to_owned(),
                };
                let window =
                    &self.epoch_durations[self.epoch_durations.len() - REMAINING_ESTIMATE_WINDOW..];
                let mean =
                    window.iter().copied().sum::<Duration>() / REMAINING_ESTIMATE_WINDOW as u32;
                // `total - seq` counts the current epoch as still-remaining; subtracting its
                // elapsed time removes the double-count.
                let remaining_epochs = total.saturating_sub(seq) as u32;
                let elapsed = self
                    .step_start
                    .map(|s| s.elapsed())
                    .unwrap_or(Duration::ZERO);
                let est = mean
                    .saturating_mul(remaining_epochs)
                    .saturating_sub(elapsed);
                super::super::format_elapsed(est)
            }
            None => "-".to_owned(),
        }
    }
}
