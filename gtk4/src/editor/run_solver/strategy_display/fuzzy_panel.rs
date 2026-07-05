use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, gtk};

use collomatique_strategies::{FindClosestProgressData, FuzzyProgressData};

#[derive(Debug)]
pub enum FuzzyPanelInput {
    /// Sent on (re)assignment: `visible` is whether the fuzzy strategy is the one now
    /// running on the worker. Also clears retained state to its fresh state.
    Reset {
        visible: bool,
    },
    Update(FuzzyProgressData),
}

pub struct FuzzyPanel {
    visible: bool,
    last: Option<FuzzyProgressData>,
    /// Retention of the `Perturbed` payload (perturbed count, total, L1 distance): it is
    /// overwritten in `last` by later repair messages, but the panel keeps showing it.
    last_perturbed: Option<(usize, usize, f64)>,
}

#[relm4::component(pub)]
impl SimpleComponent for FuzzyPanel {
    type Init = ();
    type Input = FuzzyPanelInput;
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
                    set_label: "Variables perturbées : ",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
                gtk::Label {
                    #[watch]
                    set_label: &model.perturbed_count(),
                },
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                gtk::Label {
                    set_label: "Distance L1 : ",
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
                gtk::Label {
                    #[watch]
                    set_label: &model.l1_distance(),
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
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = FuzzyPanel {
            visible: false,
            last: None,
            last_perturbed: None,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            FuzzyPanelInput::Reset { visible } => {
                self.visible = visible;
                self.last = None;
                self.last_perturbed = None;
            }
            FuzzyPanelInput::Update(data) => {
                // Capture the perturbation payload separately: it is overwritten in
                // `last` by later repair messages, but the panel keeps displaying it.
                if let FuzzyProgressData::Perturbed {
                    perturbed,
                    total,
                    l1_distance,
                } = &data
                {
                    self.last_perturbed = Some((*perturbed, *total, *l1_distance));
                }
                self.last = Some(data);
            }
        }
    }
}

impl FuzzyPanel {
    fn step(&self) -> String {
        match &self.last {
            // No progress message yet: the warm start is still being
            // perturbed (a dedicated first step, one more than FindClosest).
            None => "1/4 (perturbation)".to_string(),
            Some(FuzzyProgressData::Perturbed { .. }) => "2/4 (construction du modèle)".to_string(),
            Some(FuzzyProgressData::FindClosest(
                FindClosestProgressData::ModelReady | FindClosestProgressData::ClosenessSolve(_),
            )) => "3/4 (recherche du plus proche)".to_string(),
            Some(FuzzyProgressData::FindClosest(
                FindClosestProgressData::ObjectiveReconstruction(_)
                | FindClosestProgressData::ClosestFound,
            )) => "4/4 (calcul du coût)".to_string(),
        }
    }

    fn perturbed_count(&self) -> String {
        match self.last_perturbed {
            Some((perturbed, total, _)) => format!("{perturbed} / {total}"),
            None => "-".to_owned(),
        }
    }

    fn l1_distance(&self) -> String {
        match self.last_perturbed {
            Some((_, _, l1_distance)) => format!("{l1_distance:.1}"),
            None => "-".to_owned(),
        }
    }

    fn cost(&self) -> String {
        match &self.last {
            Some(FuzzyProgressData::FindClosest(
                FindClosestProgressData::ObjectiveReconstruction(p),
            )) => p
                .best_obj
                .map_or_else(|| "-".to_owned(), |o| format!("{o:.1}")),
            _ => "-".to_owned(),
        }
    }
}
