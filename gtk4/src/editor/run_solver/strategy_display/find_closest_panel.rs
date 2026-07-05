use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, gtk};

use collomatique_strategies::FindClosestProgressData;

#[derive(Debug)]
pub enum FindClosestPanelInput {
    /// Sent on (re)assignment: `visible` is whether the find-closest strategy is the one
    /// now running on the worker. Also clears retained state to its fresh state.
    Reset {
        visible: bool,
    },
    Update(FindClosestProgressData),
}

pub struct FindClosestPanel {
    visible: bool,
    last: Option<FindClosestProgressData>,
}

#[relm4::component(pub)]
impl SimpleComponent for FindClosestPanel {
    type Init = ();
    type Input = FindClosestPanelInput;
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
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = FindClosestPanel {
            visible: false,
            last: None,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            FindClosestPanelInput::Reset { visible } => {
                self.visible = visible;
                self.last = None;
            }
            FindClosestPanelInput::Update(data) => {
                self.last = Some(data);
            }
        }
    }
}

impl FindClosestPanel {
    fn step(&self) -> String {
        match &self.last {
            // No progress message yet: the surrogate model is still being
            // assembled (this can take a while, hence a dedicated step).
            None => "1/3 (construction du modèle)".to_string(),
            Some(FindClosestProgressData::ModelReady)
            | Some(FindClosestProgressData::ClosenessSolve(_)) => {
                "2/3 (recherche du plus proche)".to_string()
            }
            Some(
                FindClosestProgressData::ClosestFound
                | FindClosestProgressData::ObjectiveReconstruction(_),
            ) => "3/3 (calcul du coût)".to_string(),
        }
    }

    fn cost(&self) -> String {
        match &self.last {
            Some(FindClosestProgressData::ObjectiveReconstruction(p)) => p
                .best_obj
                .map_or_else(|| "-".to_owned(), |o| format!("{o:.1}")),
            _ => "-".to_owned(),
        }
    }
}
