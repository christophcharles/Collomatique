use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, gtk};

use collomatique_strategies::{IncrementalProgressData, NoObjectiveSolveProgress};

#[derive(Debug)]
pub enum IncrementalPanelInput {
    /// Sent on (re)assignment: `visible` is whether the incremental strategy is the one now
    /// running on the worker. Also clears retained state to its fresh state.
    Reset {
        visible: bool,
    },
    Update(IncrementalProgressData),
}

pub struct IncrementalPanel {
    visible: bool,
    last: Option<IncrementalProgressData>,
    // (epoch, total, var_count) from the most recent EpochStarted/EpochSolve; kept so the epoch
    // label survives the Reconstruction phase (which carries no epoch/var_count).
    last_epoch: Option<(usize, usize, usize)>,
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
                    set_label: "Variables ajoutées (époque) : ",
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
        let model = IncrementalPanel {
            visible: false,
            last: None,
            last_epoch: None,
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
            }
            IncrementalPanelInput::Update(data) => {
                match &data {
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
                    }
                    IncrementalProgressData::Reconstruction { .. } => {}
                }
                self.last = Some(data);
            }
        }
    }
}

impl IncrementalPanel {
    fn step(&self) -> String {
        match &self.last {
            Some(IncrementalProgressData::Reconstruction { .. }) => {
                "Reconstruction finale".to_owned()
            }
            _ => match self.last_epoch {
                Some((epoch, total, _)) => format!("Époque {}/{}", epoch + 1, total),
                None => "-".to_owned(),
            },
        }
    }

    fn var_count(&self) -> String {
        match self.last_epoch {
            Some((_, _, var_count)) => format!("{var_count}"),
            None => "-".to_owned(),
        }
    }

    /// The nested sub-solver progress for the current phase, if any (present on `EpochSolve` and
    /// `Reconstruction`; `EpochStarted` has no sub-progress yet).
    fn progress(&self) -> Option<&NoObjectiveSolveProgress> {
        match &self.last {
            Some(IncrementalProgressData::EpochSolve { progress, .. })
            | Some(IncrementalProgressData::Reconstruction { progress, .. }) => Some(progress),
            _ => None,
        }
    }

    fn node_count(&self) -> String {
        match self.progress() {
            Some(p) => format!("{}", p.node_count),
            None => "0".to_owned(),
        }
    }

    fn solutions_found(&self) -> String {
        match self.progress() {
            Some(p) => format!("{}", p.solutions_found),
            None => "0".to_owned(),
        }
    }
}
