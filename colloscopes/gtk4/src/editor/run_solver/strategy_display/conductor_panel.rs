use std::collections::HashMap;

use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, gtk};

use collomatique_strategies::{ConductorProgressData, ConductorStatusData};

#[derive(Debug)]
pub enum ConductorPanelInput {
    /// Sent on (re)assignment: `visible` is whether the conductor strategy is the one now
    /// running on the worker; `total` is the conductor's worker-slot count. Also clears
    /// retained state to its fresh state.
    Reset {
        visible: bool,
        total: u32,
    },
    Update(ConductorProgressData),
}

pub struct ConductorPanel {
    visible: bool,
    /// Only the aggregated `Conductor` status is meaningful here; the per-worker
    /// sub-progress variants live in the surrogate coordinate system and are ignored.
    last: Option<ConductorStatusData>,
    /// Number of worker slots of the (nested) conductor being displayed.
    total: u32,
    /// Sub-worker number → whether it currently has a strategy assigned. Learned from the
    /// conductor's `WorkerAssigned` events; the active count is those mapped to `true`.
    worker_active: HashMap<u32, bool>,
}

#[relm4::component(pub)]
impl SimpleComponent for ConductorPanel {
    type Init = ();
    type Input = ConductorPanelInput;
    type Output = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_margin_all: 5,
            set_hexpand: true,
            set_vexpand: true,
            set_halign: gtk::Align::Start,
            set_valign: gtk::Align::Center,
            set_spacing: 5,
            #[watch]
            set_visible: model.visible,
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                gtk::Label {
                    set_label: "Meilleur coût trouvé : ",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
                gtk::Label {
                    #[watch]
                    set_label: &model.best_found_cost(),
                },
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                gtk::Label {
                    set_label: "Meilleur coût possible : ",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
                gtk::Label {
                    #[watch]
                    set_label: &model.best_possible_cost(),
                },
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                gtk::Label {
                    set_label: "Tâches actives : ",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
                gtk::Label {
                    #[watch]
                    set_label: &model.active_tasks(),
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ConductorPanel {
            visible: false,
            last: None,
            total: 0,
            worker_active: HashMap::new(),
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            ConductorPanelInput::Reset { visible, total } => {
                self.visible = visible;
                self.total = total;
                self.last = None;
                self.worker_active.clear();
            }
            // Retain only the aggregated status; other variants are non-contributing.
            ConductorPanelInput::Update(ConductorProgressData::Conductor(status)) => {
                self.last = Some(status);
            }
            // Track each sub-worker's assignment so we can report the active-task count.
            ConductorPanelInput::Update(ConductorProgressData::WorkerAssigned {
                worker_num,
                strategy,
            }) => {
                self.worker_active.insert(worker_num, strategy.is_some());
            }
            ConductorPanelInput::Update(_) => {}
        }
    }
}

impl ConductorPanel {
    fn best_found_cost(&self) -> String {
        match &self.last {
            Some(status) => match &status.best_solution {
                Some(sol) => format!("{:.1}", sol.objective),
                None => "-".to_owned(),
            },
            None => "-".to_owned(),
        }
    }

    fn best_possible_cost(&self) -> String {
        match &self.last {
            Some(status) => match status.best_bound {
                Some(val) => format!("{:.1}", val),
                None => "-".to_owned(),
            },
            None => "-".to_owned(),
        }
    }

    fn active_tasks(&self) -> String {
        let active = self.worker_active.values().filter(|a| **a).count();
        format!("{}/{}", active, self.total)
    }
}
