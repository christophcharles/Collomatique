use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, gtk};

use collomatique_strategies::{NoObjectiveProgressData, NoObjectiveStarterProgressData};

#[derive(Debug)]
pub enum NoObjectiveStarterPanelInput {
    /// Sent on (re)assignment: `visible` is whether the no-objective-starter strategy is
    /// the one now running on the worker. Also clears retained state to its fresh state.
    Reset {
        visible: bool,
    },
    Update(NoObjectiveStarterProgressData),
}

pub struct NoObjectiveStarterPanel {
    visible: bool,
    last: Option<NoObjectiveStarterProgressData>,
}

#[relm4::component(pub)]
impl SimpleComponent for NoObjectiveStarterPanel {
    type Init = ();
    type Input = NoObjectiveStarterPanelInput;
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
                    set_label: "Meilleur objectif trouvé : ",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
                gtk::Label {
                    #[watch]
                    set_label: &model.best_obj(),
                },
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                gtk::Label {
                    set_label: "Meilleur objectif possible : ",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
                gtk::Label {
                    #[watch]
                    set_label: &model.best_bound(),
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
        let model = NoObjectiveStarterPanel {
            visible: false,
            last: None,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            NoObjectiveStarterPanelInput::Reset { visible } => {
                self.visible = visible;
                self.last = None;
            }
            NoObjectiveStarterPanelInput::Update(data) => {
                self.last = Some(data);
            }
        }
    }
}

impl NoObjectiveStarterPanel {
    fn step(&self) -> String {
        match &self.last {
            None
            | Some(NoObjectiveStarterProgressData::Starter(
                NoObjectiveProgressData::CheckerSolve(_),
            )) => "1/3 (démarrage)".to_string(),
            Some(NoObjectiveStarterProgressData::Starter(_)) => {
                "2/3 (calcul de l'objectif)".to_string()
            }
            Some(_) => "3/3 (optimisation)".to_string(),
        }
    }

    fn best_obj(&self) -> String {
        match &self.last {
            Some(NoObjectiveStarterProgressData::Default(p)) => p
                .best_obj
                .map_or_else(|| "-".to_owned(), |o| format!("{o:.1}")),
            _ => "-".to_owned(),
        }
    }

    fn best_bound(&self) -> String {
        match &self.last {
            Some(NoObjectiveStarterProgressData::Default(p)) => format!("{:.1}", p.best_bound),
            _ => "-".to_owned(),
        }
    }

    fn node_count(&self) -> String {
        match &self.last {
            Some(NoObjectiveStarterProgressData::Default(p)) => format!("{}", p.node_count),
            _ => "0".to_owned(),
        }
    }

    fn solutions_found(&self) -> String {
        match &self.last {
            Some(NoObjectiveStarterProgressData::Default(p)) => format!("{}", p.solutions_found),
            _ => "0".to_owned(),
        }
    }
}
