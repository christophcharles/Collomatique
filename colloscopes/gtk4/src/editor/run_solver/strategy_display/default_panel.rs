use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, gtk};

use collomatique_strategies::SolveProgressData;

#[derive(Debug)]
pub enum DefaultPanelInput {
    /// Sent on (re)assignment: `visible` is whether the default strategy is the one now
    /// running on the worker. Also clears retained metrics to their fresh state.
    Reset {
        visible: bool,
    },
    Update(SolveProgressData),
}

pub struct DefaultPanel {
    visible: bool,
    last: Option<SolveProgressData>,
}

#[relm4::component(pub)]
impl SimpleComponent for DefaultPanel {
    type Init = ();
    type Input = DefaultPanelInput;
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
                    set_label: &model.best_obj(),
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
        let model = DefaultPanel {
            visible: false,
            last: None,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            DefaultPanelInput::Reset { visible } => {
                self.visible = visible;
                self.last = None;
            }
            DefaultPanelInput::Update(data) => {
                self.last = Some(data);
            }
        }
    }
}

impl DefaultPanel {
    fn best_obj(&self) -> String {
        match &self.last {
            Some(p) => p
                .best_obj
                .map_or_else(|| "-".to_owned(), |o| format!("{o:.1}")),
            None => "-".to_owned(),
        }
    }

    fn best_bound(&self) -> String {
        match &self.last {
            Some(p) => format!("{:.1}", p.best_bound),
            None => "-".to_owned(),
        }
    }

    fn node_count(&self) -> String {
        match &self.last {
            Some(p) => format!("{}", p.node_count),
            None => "0".to_owned(),
        }
    }

    fn solutions_found(&self) -> String {
        match &self.last {
            Some(p) => format!("{}", p.solutions_found),
            None => "0".to_owned(),
        }
    }
}
