mod conductor_panel;
mod default_panel;
mod find_closest_panel;
mod fuzzy_panel;
mod no_objective_panel;
mod no_objective_starter_panel;

use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::factory::{FactoryComponent, FactorySender, FactoryView};
use relm4::prelude::DynamicIndex;
use relm4::{Component, ComponentController, Controller, RelmWidgetExt, adw, gtk};

use std::time::Instant;

use collomatique_strategies::{StrategyKind, StrategyProgressData};

use crate::widgets::debug_view::{DebugView, DebugViewInput};

use conductor_panel::{ConductorPanel, ConductorPanelInput};
use default_panel::{DefaultPanel, DefaultPanelInput};
use find_closest_panel::{FindClosestPanel, FindClosestPanelInput};
use fuzzy_panel::{FuzzyPanel, FuzzyPanelInput};
use no_objective_panel::{NoObjectivePanel, NoObjectivePanelInput};
use no_objective_starter_panel::{NoObjectiveStarterPanel, NoObjectiveStarterPanelInput};

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
    /// Periodic no-op tick from the dialog: forces the elapsed-time label to recompute.
    Refresh,
}

pub struct StrategyFrame {
    debug_view: Controller<DebugView>,
    worker_num: u32,
    strategy_kind: Option<StrategyKind>,
    idle: bool,
    show_debug: bool,
    last_line: String,
    // Elapsed-time bookkeeping for this worker: `timer_start` set on assignment, `timer_end`
    // frozen when the worker goes idle.
    timer_start: Option<Instant>,
    timer_end: Option<Instant>,
    // One display panel per strategy kind. Each owns its retained state and its own
    // visibility, driven by the `Reset`/`Update` signals routed from `update`.
    default_panel: Controller<DefaultPanel>,
    no_objective_panel: Controller<NoObjectivePanel>,
    find_closest_panel: Controller<FindClosestPanel>,
    fuzzy_panel: Controller<FuzzyPanel>,
    no_objective_starter_panel: Controller<NoObjectiveStarterPanel>,
    conductor_panel: Controller<ConductorPanel>,
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
                            set_orientation: gtk::Orientation::Vertical,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_spacing: 5,
                            set_size_request: (150,-1),
                            #[watch]
                            set_visible: !self.idle,
                            adw::Spinner {
                                set_size_request: (60, 60),
                            },
                            gtk::Label {
                                set_margin_top: 15,
                                set_hexpand: true,
                                set_justify: gtk::Justification::Center,
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
                            gtk::Label {
                                add_css_class: "monospace",
                                set_margin_top: 10,
                                #[watch]
                                set_label: &self.elapsed(),
                            },
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_spacing: 5,
                            set_size_request: (150,-1),
                            #[watch]
                            set_visible: self.idle,
                            gtk::Image::from_icon_name("media-playback-pause-symbolic") {
                                set_size_request: (60, 60),
                                set_pixel_size: 60,
                            },
                            gtk::Label {
                                set_margin_top: 15,
                                set_hexpand: true,
                                set_justify: gtk::Justification::Center,
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
                                set_label: "À l'arrêt",
                            },
                            gtk::Label {
                                add_css_class: "monospace",
                                set_margin_top: 10,
                                #[watch]
                                set_label: &self.elapsed(),
                            },
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        append = self.default_panel.widget(),
                        append = self.no_objective_panel.widget(),
                        append = self.find_closest_panel.widget(),
                        append = self.fuzzy_panel.widget(),
                        append = self.no_objective_starter_panel.widget(),
                        append = self.conductor_panel.widget(),
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
            timer_start: None,
            timer_end: None,
            default_panel: DefaultPanel::builder().launch(()).detach(),
            no_objective_panel: NoObjectivePanel::builder().launch(()).detach(),
            find_closest_panel: FindClosestPanel::builder().launch(()).detach(),
            fuzzy_panel: FuzzyPanel::builder().launch(()).detach(),
            no_objective_starter_panel: NoObjectiveStarterPanel::builder().launch(()).detach(),
            conductor_panel: ConductorPanel::builder().launch(()).detach(),
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
                self.last_line = String::new();
                self.timer_start = None;
                self.timer_end = None;
                self.debug_view.emit(DebugViewInput::Clear);
                // Hide and clear every panel until a strategy is assigned.
                self.reset_panels(None);
            }
            StrategyDisplayInput::Assigned(Some(name)) => {
                // A new substrategy starts: reset the metrics but keep the echo, marking
                // the boundary in the log so worker switches stay legible.
                use collomatique_strategies::Strategy;
                self.debug_view.emit(DebugViewInput::Append(format!(
                    "\n=== Worker assigned: {} ===\n\n",
                    name.name(),
                )));
                self.idle = false;
                self.timer_start = Some(Instant::now());
                self.timer_end = None;
                // Make the matching panel visible with fresh state; clear and hide the rest.
                self.reset_panels(Some(&name));
                self.strategy_kind = Some(name);
            }
            StrategyDisplayInput::Assigned(None) => {
                // The worker went idle: metrics and strategy name persist (final figures
                // stay on screen); just mark the boundary.
                self.debug_view.emit(DebugViewInput::Append(
                    "\n=== Worker is idle ===\n\n".to_owned(),
                ));
                self.idle = true;
                self.timer_end = Some(Instant::now());
            }
            StrategyDisplayInput::StrategyUpdate(progress) => self.route_update(progress),
            StrategyDisplayInput::ToggleDebug(active) => {
                self.show_debug = active;
            }
            // The refresh only needs to re-render the view (elapsed-time label); the model
            // stays put.
            StrategyDisplayInput::Refresh => {}
        }
    }
}

impl StrategyFrame {
    /// Elapsed wall-clock time since this worker was assigned, `HH:MM:SS`: live while running,
    /// frozen to the total once the worker goes idle, `00:00:00` before any assignment.
    fn elapsed(&self) -> String {
        let d = match (self.timer_start, self.timer_end) {
            (Some(s), Some(e)) => e.saturating_duration_since(s),
            (Some(s), None) => s.elapsed(),
            _ => std::time::Duration::ZERO,
        };
        super::format_elapsed(d)
    }

    /// (Re)set every panel's visibility and clear its retained state. Exactly the panel
    /// whose strategy matches `kind` becomes visible; `None` hides them all.
    fn reset_panels(&self, kind: Option<&StrategyKind>) {
        self.default_panel.emit(DefaultPanelInput::Reset {
            visible: matches!(kind, Some(StrategyKind::Default { .. })),
        });
        self.no_objective_panel.emit(NoObjectivePanelInput::Reset {
            visible: matches!(kind, Some(StrategyKind::NoObjective { .. })),
        });
        self.find_closest_panel.emit(FindClosestPanelInput::Reset {
            visible: matches!(kind, Some(StrategyKind::FindClosest { .. })),
        });
        self.fuzzy_panel.emit(FuzzyPanelInput::Reset {
            visible: matches!(kind, Some(StrategyKind::Fuzzy { .. })),
        });
        self.no_objective_starter_panel
            .emit(NoObjectiveStarterPanelInput::Reset {
                visible: matches!(kind, Some(StrategyKind::NoObjectiveStarter { .. })),
            });
        // The conductor panel also needs the slot count to render "Tâches actives : X/Y",
        // which is known from the assigned conductor strategy.
        let (conductor_visible, conductor_total) = match kind {
            Some(StrategyKind::Conductor(cs)) => (true, cs.worker_count.get()),
            _ => (false, 0),
        };
        self.conductor_panel.emit(ConductorPanelInput::Reset {
            visible: conductor_visible,
            total: conductor_total,
        });
    }

    /// Forward a progress update to the panel that owns its strategy kind.
    fn route_update(&self, progress: StrategyProgressData) {
        match progress {
            StrategyProgressData::Default(p) => {
                self.default_panel.emit(DefaultPanelInput::Update(p))
            }
            StrategyProgressData::NoObjective(p) => self
                .no_objective_panel
                .emit(NoObjectivePanelInput::Update(p)),
            StrategyProgressData::FindClosest(p) => self
                .find_closest_panel
                .emit(FindClosestPanelInput::Update(p)),
            StrategyProgressData::Fuzzy(p) => self.fuzzy_panel.emit(FuzzyPanelInput::Update(p)),
            StrategyProgressData::NoObjectiveStarter(p) => self
                .no_objective_starter_panel
                .emit(NoObjectiveStarterPanelInput::Update(p)),
            StrategyProgressData::Conductor(p) => {
                self.conductor_panel.emit(ConductorPanelInput::Update(p))
            }
        }
    }
}
