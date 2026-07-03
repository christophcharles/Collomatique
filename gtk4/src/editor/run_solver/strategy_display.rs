use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::factory::{FactoryComponent, FactorySender, FactoryView};
use relm4::prelude::DynamicIndex;
use relm4::{Component, ComponentController, Controller, RelmWidgetExt, adw, gtk};

use collomatique_strategies::{SolveProgressData, StrategyKind, StrategyProgressData};

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
    last_progress: Option<SolveProgressData>,
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
                            ).unwrap_or_default()),
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
                                set_label: &self.format_best_obj(),
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
                                set_label: &self.format_best_bound(),
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
                                set_label: &self.format_node_count(),
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
                                set_label: &self.format_solutions_found(),
                            },
                        },
                    },
                    gtk::Box {
                        set_hexpand: true,
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
            last_progress: None,
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
                self.debug_view.emit(DebugViewInput::Append(line));
            }
            StrategyDisplayInput::Reset(worker_num) => {
                self.worker_num = worker_num;
                self.strategy_kind = None;
                self.show_debug = false;
                self.idle = true;
                self.last_progress = None;
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
            StrategyDisplayInput::StrategyUpdate(progress) => match progress {
                StrategyProgressData::Default(p) => {
                    self.last_progress = Some(p);
                }
                StrategyProgressData::NoObjective(_) => todo!(),
                StrategyProgressData::NoObjectiveStarter(_) => todo!(),
                StrategyProgressData::Conductor(_) => todo!(),
            },
            StrategyDisplayInput::ToggleDebug(active) => {
                self.show_debug = active;
            }
        }
    }
}

impl StrategyFrame {
    fn format_best_obj(&self) -> String {
        match &self.last_progress {
            Some(p) => format!("{:.1}", p.best_obj),
            None => "-".to_owned(),
        }
    }

    fn format_best_bound(&self) -> String {
        match &self.last_progress {
            Some(p) => format!("{:.1}", p.best_bound),
            None => "-".to_owned(),
        }
    }

    fn format_node_count(&self) -> String {
        match &self.last_progress {
            Some(p) => format!("{}", p.node_count),
            None => "0".to_owned(),
        }
    }

    fn format_solutions_found(&self) -> String {
        match &self.last_progress {
            Some(p) => format!("{}", p.solutions_found),
            None => "0".to_owned(),
        }
    }
}
