use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::factory::{FactoryComponent, FactorySender, FactoryView};
use relm4::prelude::DynamicIndex;
use relm4::{Component, ComponentController, Controller, RelmWidgetExt, adw, gtk};

use collomatique_strategies::{
    ConductorProgressData, FindClosestProgressData, FuzzyProgressData, NoObjectiveProgressData,
    NoObjectiveStarterProgressData, StrategyKind, StrategyProgressData,
};

use crate::widgets::debug_view::{DebugView, DebugViewInput};

#[derive(Debug, Clone)]
pub enum StrategyDisplayInput {
    Echo(String),
    /// The conductor was (re)launched: full reset of the display (metrics and echo)
    /// and (re)binding of this frame to the given worker number.
    Reset(u32),
    /// The displayed worker was (re)assigned: `Some` = a substrategy is running,
    /// `None` = the worker went idle. The echo is preserved; the display marks the
    /// boundary itself.
    Assigned(Option<StrategyKind>),
    StrategyUpdate(StrategyProgressData),
    ToggleDebug(bool),
}

pub struct StrategyFrame {
    debug_view: Controller<DebugView>,
    worker_num: u32,
    strategy_kind: Option<StrategyKind>,
    idle: bool,
    show_debug: bool,
    last_line: String,
    last_progress: Option<StrategyProgressData>,
    /// Quick-and-dirty retention of the fuzzy `Perturbed` payload
    /// (perturbed count, total, L1 distance): it is lost from
    /// `last_progress` once a later `FindClosest` message overwrites it, but
    /// the panel keeps showing it. A cleaner per-strategy state model is left
    /// for later.
    last_fuzzy_perturbed: Option<(usize, usize, f64)>,
}

#[relm4::factory(pub)]
impl FactoryComponent for StrategyFrame {
    type Init = u32;
    type Input = StrategyDisplayInput;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Stack;

    view! {
        #[root]
        gtk::Box {
            set_margin_all: 5,
            set_spacing: 5,
            set_hexpand: true,
            set_vexpand: true,
            set_orientation: gtk::Orientation::Vertical,

            gtk::Box {
                set_hexpand: true,
                set_vexpand: true,
                #[watch]
                set_visible: self.show_debug,
                append = self.debug_view.widget(),
            },
            gtk::Frame {
                set_margin_all: 5,
                set_hexpand: true,
                set_vexpand: true,
                #[watch]
                set_visible: !self.show_debug,
                gtk::Box {
                    set_margin_all: 0,
                    set_hexpand: true,
                    set_vexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_hexpand: true,
                        set_vexpand: true,

                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_spacing: 5,
                            #[watch]
                            set_visible: !self.idle,
                            adw::Spinner {
                                set_size_request: (60, 60),
                            },
                            gtk::Label {
                                set_margin_top: 15,
                                #[watch]
                                set_label: &format!("Tâche {} : {}", self.worker_num+1, self.strategy_kind.as_ref().map(
                                    |strat| {
                                        use collomatique_strategies::Strategy;
                                        strat.ui_name()
                                    }
                                ).unwrap_or("non-attribuée")),
                                set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                            },
                            gtk::Label {
                                set_label: "En cours d'exécution",
                            },
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_spacing: 5,
                            #[watch]
                            set_visible: self.idle,
                            gtk::Image::from_icon_name("media-playback-pause-symbolic") {
                                set_size_request: (60, 60),
                                set_pixel_size: 60,
                            },
                            gtk::Label {
                                set_margin_top: 15,
                                #[watch]
                                set_label: &format!("Tâche {} : {}", self.worker_num+1, self.strategy_kind.as_ref().map(
                                    |strat| {
                                        use collomatique_strategies::Strategy;
                                        strat.ui_name()
                                    }
                                ).unwrap_or_default()),
                                set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                            },
                            gtk::Label {
                                set_label: "À l'arrêt",
                            },
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_margin_all: 5,
                            set_hexpand: true,
                            set_vexpand: true,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_spacing: 5,
                            #[watch]
                            set_visible: matches!(self.strategy_kind, Some(StrategyKind::Default { .. })),
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                gtk::Label {
                                    set_label: "Meilleur coût trouvé : ",
                                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: &self.default_best_obj(),
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
                                    set_label: &self.default_best_bound(),
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
                                    set_label: &self.default_node_count(),
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
                                    set_label: &self.default_solutions_found(),
                                },
                            },
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_margin_all: 5,
                            set_hexpand: true,
                            set_vexpand: true,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_spacing: 5,
                            #[watch]
                            set_visible: matches!(self.strategy_kind, Some(StrategyKind::NoObjective { .. })),
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                gtk::Label {
                                    set_label: "Étape : ",
                                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: &self.no_obj_step(),
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
                                    set_label: &self.no_obj_cost(),
                                },
                            },
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_margin_all: 5,
                            set_hexpand: true,
                            set_vexpand: true,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_spacing: 5,
                            #[watch]
                            set_visible: matches!(self.strategy_kind, Some(StrategyKind::FindClosest { .. })),
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                gtk::Label {
                                    set_label: "Étape : ",
                                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: &self.find_closest_step(),
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
                                    set_label: &self.find_closest_cost(),
                                },
                            },
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_margin_all: 5,
                            set_hexpand: true,
                            set_vexpand: true,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_spacing: 5,
                            #[watch]
                            set_visible: matches!(self.strategy_kind, Some(StrategyKind::Fuzzy { .. })),
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                gtk::Label {
                                    set_label: "Étape : ",
                                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: &self.fuzzy_step(),
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
                                    set_label: &self.fuzzy_perturbed_count(),
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
                                    set_label: &self.fuzzy_l1_distance(),
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
                                    set_label: &self.fuzzy_cost(),
                                },
                            },
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_margin_all: 5,
                            set_hexpand: true,
                            set_vexpand: true,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_spacing: 5,
                            #[watch]
                            set_visible: matches!(self.strategy_kind, Some(StrategyKind::NoObjectiveStarter { .. })),
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                gtk::Label {
                                    set_label: "Étape : ",
                                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: &self.no_obj_starter_step(),
                                },
                            },
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                gtk::Label {
                                    set_label: "Meilleur coût trouvé : ",
                                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: &self.no_obj_starter_best_obj(),
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
                                    set_label: &self.no_obj_starter_best_bound(),
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
                                    set_label: &self.no_obj_starter_node_count(),
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
                                    set_label: &self.no_obj_starter_solutions_found(),
                                },
                            },
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_margin_all: 5,
                            set_hexpand: true,
                            set_vexpand: true,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_spacing: 5,
                            #[watch]
                            set_visible: matches!(self.strategy_kind, Some(StrategyKind::Conductor { .. })),
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                gtk::Label {
                                    set_label: "Meilleur coût trouvé : ",
                                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: &self.conductor_best_found_cost(),
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
                                    set_label: &self.conductor_best_possible_cost(),
                                },
                            },
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                    },
                    gtk::Box {
                        set_hexpand: true,
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_margin_all: 5,
                            add_css_class: "dimmed",
                            add_css_class: "monospace",
                            #[watch]
                            set_label: &self.last_line,
                        },
                    },
                },
            },
        },
        #[local_ref]
        returned_widget -> gtk::StackPage {
            set_name: &self.worker_num.to_string(),
        }
    }

    fn init_model(
        worker_num: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        let debug_view = DebugView::builder().launch(()).detach();

        StrategyFrame {
            debug_view,
            worker_num,
            strategy_kind: None,
            idle: true,
            show_debug: false,
            last_line: String::new(),
            last_progress: None,
            last_fuzzy_perturbed: None,
        }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();
        widgets
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            StrategyDisplayInput::Echo(line) => {
                self.last_line = super::truncate_line(line.trim_end());
                self.debug_view.emit(DebugViewInput::Append(line));
            }
            StrategyDisplayInput::Reset(worker_num) => {
                self.worker_num = worker_num;
                self.strategy_kind = None;
                self.show_debug = false;
                self.idle = true;
                self.last_progress = None;
                self.last_fuzzy_perturbed = None;
                self.last_line = String::new();
                self.debug_view.emit(DebugViewInput::Clear);
            }
            StrategyDisplayInput::Assigned(Some(name)) => {
                // A new substrategy starts: reset the metrics but keep the echo, marking
                // the boundary in the log so worker switches stay legible.
                use collomatique_strategies::Strategy;
                self.debug_view.emit(DebugViewInput::Append(format!(
                    "\n=== Worker assigned: {} ===\n\n",
                    name.name(),
                )));
                self.strategy_kind = Some(name);
                self.last_progress = None;
                self.last_fuzzy_perturbed = None;
                self.idle = false;
            }
            StrategyDisplayInput::Assigned(None) => {
                // The worker went idle: metrics and strategy name persist (final figures
                // stay on screen); just mark the boundary.
                self.debug_view.emit(DebugViewInput::Append(
                    "\n=== Worker is idle ===\n\n".to_owned(),
                ));
                self.idle = true;
            }
            StrategyDisplayInput::StrategyUpdate(progress) => {
                // Capture the fuzzy perturbation payload separately: it is
                // overwritten in `last_progress` by later repair messages, but
                // the panel keeps displaying it.
                if let StrategyProgressData::Fuzzy(FuzzyProgressData::Perturbed {
                    perturbed,
                    total,
                    l1_distance,
                }) = &progress
                {
                    self.last_fuzzy_perturbed = Some((*perturbed, *total, *l1_distance));
                }
                if Self::should_retain(&progress) {
                    self.last_progress = Some(progress);
                }
            }
            StrategyDisplayInput::ToggleDebug(active) => {
                self.show_debug = active;
            }
        }
    }
}

impl StrategyFrame {
    fn should_retain(progress: &StrategyProgressData) -> bool {
        match progress {
            StrategyProgressData::Default(_) => true,
            StrategyProgressData::NoObjective(_) => true,
            StrategyProgressData::NoObjectiveStarter(_) => true,
            StrategyProgressData::FindClosest(_) => true,
            StrategyProgressData::Fuzzy(_) => true,
            StrategyProgressData::Conductor(ConductorProgressData::Conductor(_)) => true,
            StrategyProgressData::Conductor(_) => false,
        }
    }

    fn default_best_obj(&self) -> String {
        match &self.last_progress {
            Some(StrategyProgressData::Default(p)) => format!("{:.1}", p.best_obj),
            _ => "-".to_owned(),
        }
    }

    fn default_best_bound(&self) -> String {
        match &self.last_progress {
            Some(StrategyProgressData::Default(p)) => format!("{:.1}", p.best_bound),
            _ => "-".to_owned(),
        }
    }

    fn default_node_count(&self) -> String {
        match &self.last_progress {
            Some(StrategyProgressData::Default(p)) => format!("{}", p.node_count),
            _ => "0".to_owned(),
        }
    }

    fn default_solutions_found(&self) -> String {
        match &self.last_progress {
            Some(StrategyProgressData::Default(p)) => format!("{}", p.solutions_found),
            _ => "0".to_owned(),
        }
    }

    fn no_obj_step(&self) -> String {
        match &self.last_progress {
            Some(StrategyProgressData::NoObjective(NoObjectiveProgressData::CheckerSolve(_))) => {
                "1/2 (démarrage)".to_string()
            }
            Some(StrategyProgressData::NoObjective(_)) => "2/2 (calcul du coût)".to_string(),
            _ => "-".to_owned(),
        }
    }

    fn no_obj_cost(&self) -> String {
        match &self.last_progress {
            Some(StrategyProgressData::NoObjective(
                NoObjectiveProgressData::ObjectiveReconstruction(p),
            )) => format!("{:.1}", p.best_obj),
            _ => "-".to_owned(),
        }
    }

    fn find_closest_step(&self) -> String {
        match &self.last_progress {
            // No progress message yet: the surrogate model is still being
            // assembled (this can take a while, hence a dedicated step).
            None => "1/3 (construction du modèle)".to_string(),
            Some(StrategyProgressData::FindClosest(FindClosestProgressData::ModelReady))
            | Some(StrategyProgressData::FindClosest(FindClosestProgressData::ClosenessSolve(_))) => {
                "2/3 (recherche du plus proche)".to_string()
            }
            Some(StrategyProgressData::FindClosest(
                FindClosestProgressData::ObjectiveReconstruction(_),
            )) => "3/3 (calcul du coût)".to_string(),
            _ => "-".to_owned(),
        }
    }

    fn find_closest_cost(&self) -> String {
        match &self.last_progress {
            Some(StrategyProgressData::FindClosest(
                FindClosestProgressData::ObjectiveReconstruction(p),
            )) => format!("{:.1}", p.best_obj),
            _ => "-".to_owned(),
        }
    }

    fn fuzzy_step(&self) -> String {
        match &self.last_progress {
            // No progress message yet: the warm start is still being
            // perturbed (a dedicated first step, one more than FindClosest).
            None => "1/4 (perturbation de la solution)".to_string(),
            Some(StrategyProgressData::Fuzzy(FuzzyProgressData::Perturbed { .. })) => {
                "2/4 (construction du modèle)".to_string()
            }
            Some(StrategyProgressData::Fuzzy(FuzzyProgressData::FindClosest(
                FindClosestProgressData::ModelReady | FindClosestProgressData::ClosenessSolve(_),
            ))) => "3/4 (recherche du plus proche)".to_string(),
            Some(StrategyProgressData::Fuzzy(FuzzyProgressData::FindClosest(
                FindClosestProgressData::ObjectiveReconstruction(_),
            ))) => "4/4 (calcul du coût)".to_string(),
            _ => "-".to_owned(),
        }
    }

    fn fuzzy_perturbed_count(&self) -> String {
        match self.last_fuzzy_perturbed {
            Some((perturbed, total, _)) => format!("{perturbed} / {total}"),
            None => "-".to_owned(),
        }
    }

    fn fuzzy_l1_distance(&self) -> String {
        match self.last_fuzzy_perturbed {
            Some((_, _, l1_distance)) => format!("{l1_distance:.1}"),
            None => "-".to_owned(),
        }
    }

    fn fuzzy_cost(&self) -> String {
        match &self.last_progress {
            Some(StrategyProgressData::Fuzzy(FuzzyProgressData::FindClosest(
                FindClosestProgressData::ObjectiveReconstruction(p),
            ))) => format!("{:.1}", p.best_obj),
            _ => "-".to_owned(),
        }
    }

    fn no_obj_starter_best_obj(&self) -> String {
        match &self.last_progress {
            Some(StrategyProgressData::NoObjectiveStarter(
                NoObjectiveStarterProgressData::Default(p),
            )) => format!("{:.1}", p.best_obj),
            _ => "-".to_owned(),
        }
    }

    fn no_obj_starter_best_bound(&self) -> String {
        match &self.last_progress {
            Some(StrategyProgressData::NoObjectiveStarter(
                NoObjectiveStarterProgressData::Default(p),
            )) => format!("{:.1}", p.best_bound),
            _ => "-".to_owned(),
        }
    }

    fn no_obj_starter_node_count(&self) -> String {
        match &self.last_progress {
            Some(StrategyProgressData::NoObjectiveStarter(
                NoObjectiveStarterProgressData::Default(p),
            )) => format!("{}", p.node_count),
            _ => "0".to_owned(),
        }
    }

    fn no_obj_starter_solutions_found(&self) -> String {
        match &self.last_progress {
            Some(StrategyProgressData::NoObjectiveStarter(
                NoObjectiveStarterProgressData::Default(p),
            )) => format!("{}", p.solutions_found),
            _ => "0".to_owned(),
        }
    }

    fn no_obj_starter_step(&self) -> String {
        match &self.last_progress {
            Some(StrategyProgressData::NoObjectiveStarter(
                NoObjectiveStarterProgressData::Starter(NoObjectiveProgressData::CheckerSolve(_)),
            )) => "1/3 (démarrage)".to_string(),
            Some(StrategyProgressData::NoObjectiveStarter(
                NoObjectiveStarterProgressData::Starter(_),
            )) => "2/3 (calcul du coût)".to_string(),
            Some(StrategyProgressData::NoObjectiveStarter(_)) => "3/3 (optimisation)".to_string(),
            _ => "-".to_owned(),
        }
    }

    fn conductor_best_found_cost(&self) -> String {
        match &self.last_progress {
            Some(StrategyProgressData::Conductor(ConductorProgressData::Conductor(p))) => {
                match &p.best_solution {
                    Some(sol) => format!("{:.1}", sol.objective),
                    None => "-".to_owned(),
                }
            }
            _ => "-".to_owned(),
        }
    }

    fn conductor_best_possible_cost(&self) -> String {
        match &self.last_progress {
            Some(StrategyProgressData::Conductor(ConductorProgressData::Conductor(p))) => {
                match p.best_bound {
                    Some(val) => format!("{:.1}", val),
                    None => "-".to_owned(),
                }
            }
            _ => "-".to_owned(),
        }
    }
}
