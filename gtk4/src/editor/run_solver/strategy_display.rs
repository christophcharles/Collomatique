use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::factory::{FactoryComponent, FactorySender, FactoryView};
use relm4::prelude::DynamicIndex;
use relm4::{Component, ComponentController, Controller, RelmWidgetExt, gtk};

use collomatique_strategies::{SolveProgressData, StrategyKind, StrategyProgressData};

use crate::widgets::debug_view::{DebugView, DebugViewInput};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyName {
    Default,
}

pub fn strategy_name_from_kind(kind: &StrategyKind) -> StrategyName {
    match kind {
        StrategyKind::Default(_) => StrategyName::Default,
        StrategyKind::NoObjective(_) => todo!(),
        StrategyKind::NoObjectiveStarter(_) => todo!(),
        StrategyKind::Conductor(_) => todo!(),
    }
}

#[derive(Debug, Clone)]
pub enum StrategyDisplayInput {
    Echo(String),
    /// The conductor was (re)launched: full reset of the display (metrics and echo)
    /// and (re)binding of this frame to the given worker number.
    Reset(u32),
    /// The displayed worker was (re)assigned: `Some` = a substrategy is running,
    /// `None` = the worker went idle. The echo is preserved; the display marks the
    /// boundary itself.
    Assigned(Option<StrategyName>),
    StrategyUpdate(StrategyProgressData),
    ToggleDebug(bool),
}

pub struct StrategyFrame {
    debug_view: Controller<DebugView>,
    worker_num: u32,
    strategy_name: Option<StrategyName>,
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
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_vexpand: true,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_margin_all: 5,
                        set_hexpand: true,
                        set_vexpand: true,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        #[watch]
                        set_visible: self.strategy_name == Some(StrategyName::Default),
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            #[watch]
                            set_label: &self.format_best_obj(),
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            #[watch]
                            set_label: &self.format_best_bound(),
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            #[watch]
                            set_label: &self.format_node_count(),
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            #[watch]
                            set_label: &self.format_solutions_found(),
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
            strategy_name: None,
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
                self.strategy_name = None;
                self.show_debug = false;
                self.last_progress = None;
                self.debug_view.emit(DebugViewInput::Clear);
            }
            StrategyDisplayInput::Assigned(Some(name)) => {
                // A new substrategy starts: reset the metrics but keep the echo, marking
                // the boundary in the log so worker switches stay legible.
                self.strategy_name = Some(name);
                self.last_progress = None;
                self.debug_view.emit(DebugViewInput::Append(format!(
                    "\n=== Worker assigned: {name:?} ===\n\n"
                )));
            }
            StrategyDisplayInput::Assigned(None) => {
                // The worker went idle: metrics and strategy name persist (final figures
                // stay on screen); just mark the boundary.
                self.debug_view.emit(DebugViewInput::Append(
                    "\n=== Worker is idle ===\n\n".to_owned(),
                ));
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
            Some(p) => format!("Objectif : {:.4}", p.best_obj),
            None => "Objectif : —".to_owned(),
        }
    }

    fn format_best_bound(&self) -> String {
        match &self.last_progress {
            Some(p) => format!("Borne : {:.4}", p.best_bound),
            None => "Borne : —".to_owned(),
        }
    }

    fn format_node_count(&self) -> String {
        match &self.last_progress {
            Some(p) => format!("Nœuds : {}", p.node_count),
            None => "Nœuds : —".to_owned(),
        }
    }

    fn format_solutions_found(&self) -> String {
        match &self.last_progress {
            Some(p) => format!("Solutions trouvées : {}", p.solutions_found),
            None => "Solutions trouvées : —".to_owned(),
        }
    }
}
