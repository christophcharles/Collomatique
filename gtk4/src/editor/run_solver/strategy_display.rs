use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, ToggleButtonExt, WidgetExt};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    SimpleComponent, adw, gtk,
};

use collomatique_strategies::{SolveProgress, StrategyKind, StrategyProgress};
use collomatique_subprocesses::{StrategyResult, StrategyStatus};

use crate::widgets::debug_view::{DebugView, DebugViewInput};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyName {
    Default,
}

pub fn strategy_name_from_kind(kind: &StrategyKind) -> StrategyName {
    match kind {
        StrategyKind::Default(_) => StrategyName::Default,
    }
}

pub struct StrategyDisplay {
    debug_view: Controller<DebugView>,
    strategy_name: Option<StrategyName>,
    show_debug: bool,
    is_running: bool,
    end_with_error: bool,
    last_progress: Option<SolveProgress>,
}

#[derive(Debug)]
pub enum StrategyDisplayInput {
    Echo(String),
    Clear(StrategyName),
    StrategyUpdate(Result<StrategyProgress, String>),
    Finished(StrategyResult),
    ToggleDebug(bool),
}

#[relm4::component(pub)]
impl SimpleComponent for StrategyDisplay {
    type Init = ();
    type Input = StrategyDisplayInput;
    type Output = ();

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
                set_visible: model.show_debug,
                append = model.debug_view.widget(),
            },
            gtk::Frame{
                set_margin_all: 5,
                set_hexpand: true,
                set_vexpand: true,
                #[watch]
                set_visible: !model.show_debug,

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
                        set_visible: model.strategy_name == Some(StrategyName::Default),
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            #[watch]
                            set_label: &model.format_best_obj(),
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            #[watch]
                            set_label: &model.format_best_bound(),
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            #[watch]
                            set_label: &model.format_node_count(),
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            #[watch]
                            set_label: &model.format_solutions_found(),
                        },
                    },
                },
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_hexpand: true,
                set_margin_all: 5,
                set_spacing: 10,
                gtk::Label {
                    #[watch]
                    set_label: &model.strategy_name_label(),
                    set_valign: gtk::Align::Center,
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                },
                gtk::Box {
                    set_hexpand: true,
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_valign: gtk::Align::Center,
                    #[watch]
                    set_visible: model.is_running,
                    set_spacing: 5,
                    adw::Spinner {},
                    gtk::Label {
                        set_label: "Exécution en cours",
                    },
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_valign: gtk::Align::Center,
                    #[watch]
                    set_visible: !model.is_running && !model.end_with_error,
                    gtk::Image::from_icon_name("emblem-ok-symbolic") {
                        set_size_request: (30, 30),
                        set_icon_size: gtk::IconSize::Normal,
                    },
                    gtk::Label {
                        set_label: "Exécution terminée",
                    },
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_valign: gtk::Align::Center,
                    #[watch]
                    set_visible: !model.is_running && model.end_with_error,
                    gtk::Image::from_icon_name("dialog-error-symbolic") {
                        set_size_request: (30, 30),
                        set_icon_size: gtk::IconSize::Normal,
                    },
                    gtk::Label {
                        set_label: "Erreur pendant l'exécution",
                    },
                },
                gtk::ToggleButton {
                    set_icon_name: "utilities-terminal-symbolic",
                    #[watch]
                    set_active: model.show_debug,
                    connect_toggled[sender] => move |btn| {
                        sender.input(StrategyDisplayInput::ToggleDebug(btn.is_active()));
                    },
                },
            },
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let debug_view = DebugView::builder().launch(()).detach();

        let model = StrategyDisplay {
            debug_view,
            strategy_name: None,
            show_debug: false,
            is_running: false,
            end_with_error: false,
            last_progress: None,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            StrategyDisplayInput::Echo(line) => {
                self.debug_view
                    .emit(DebugViewInput::Append(format!("{line}\n")));
            }
            StrategyDisplayInput::Clear(name) => {
                self.strategy_name = Some(name);
                self.show_debug = false;
                self.is_running = true;
                self.end_with_error = false;
                self.last_progress = None;
                self.debug_view.emit(DebugViewInput::Clear);
            }
            StrategyDisplayInput::StrategyUpdate(progress) => {
                match progress {
                    Ok(StrategyProgress::Default(p)) => {
                        self.last_progress = Some(p);
                    }
                    // TEMPORARY: route progress errors to stderr
                    Err(e) => eprintln!("  [strategy] [progress error] {e}"),
                }
            }
            StrategyDisplayInput::Finished(result) => {
                self.is_running = false;
                self.end_with_error = matches!(
                    result.status,
                    StrategyStatus::Error | StrategyStatus::Infeasible
                );
            }
            StrategyDisplayInput::ToggleDebug(active) => {
                self.show_debug = active;
            }
        }
    }
}

impl StrategyDisplay {
    fn strategy_name_label(&self) -> String {
        match self.strategy_name {
            Some(StrategyName::Default) => "Stratégie par défaut".to_owned(),
            None => String::new(),
        }
    }

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
