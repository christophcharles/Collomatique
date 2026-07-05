use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, gtk};

use collomatique_strategies::{NoObjectiveProgressData, NoObjectiveSolveProgress};

#[derive(Debug)]
pub enum NoObjectivePanelInput {
    /// Sent on (re)assignment: `visible` is whether the no-objective strategy is the one
    /// now running on the worker. Also clears retained state to its fresh state.
    Reset {
        visible: bool,
    },
    Update(NoObjectiveProgressData),
}

pub struct NoObjectivePanel {
    visible: bool,
    last: Option<NoObjectiveProgressData>,
    // Retained node/solution counts from the phase-1 feasibility search (CheckerSolve).
    // Reconstruction counts are intentionally ignored; kept across SolutionFound/phase 2.
    checker: Option<NoObjectiveSolveProgress>,
}

#[relm4::component(pub)]
impl SimpleComponent for NoObjectivePanel {
    type Init = ();
    type Input = NoObjectivePanelInput;
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
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = NoObjectivePanel {
            visible: false,
            last: None,
            checker: None,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            NoObjectivePanelInput::Reset { visible } => {
                self.visible = visible;
                self.last = None;
                self.checker = None;
            }
            NoObjectivePanelInput::Update(data) => {
                if let NoObjectiveProgressData::CheckerSolve(p) = &data {
                    self.checker = Some(p.clone());
                }
                self.last = Some(data);
            }
        }
    }
}

impl NoObjectivePanel {
    fn step(&self) -> String {
        match &self.last {
            None | Some(NoObjectiveProgressData::CheckerSolve(_)) => "1/2 (amorçage)".to_string(),
            Some(_) => "2/2 (calcul du coût)".to_string(),
        }
    }

    fn cost(&self) -> String {
        match &self.last {
            Some(NoObjectiveProgressData::ObjectiveReconstruction(p)) => p
                .best_obj
                .map_or_else(|| "-".to_owned(), |o| format!("{o:.1}")),
            _ => "-".to_owned(),
        }
    }

    fn node_count(&self) -> String {
        match &self.checker {
            Some(p) => format!("{}", p.node_count),
            None => "0".to_owned(),
        }
    }

    fn solutions_found(&self) -> String {
        match &self.checker {
            Some(p) => format!("{}", p.solutions_found),
            None => "0".to_owned(),
        }
    }
}
