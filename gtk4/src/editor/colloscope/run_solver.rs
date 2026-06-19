use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{Component, ComponentController, adw, gtk};
use relm4::{ComponentParts, ComponentSender, Controller, RelmWidgetExt};

use std::sync::{Arc, Mutex};

use collomatique_strategies::StrategyKind;
use collomatique_subprocesses::{
    StrategyResult, StrategyStatus, StrategySubprocess, WorkerManager,
};

use crate::widgets::debug_view::{DebugView, DebugViewInput};

mod error_dialog;
mod warning_running;

type Colloscope = collomatique_state_colloscopes::colloscopes::Colloscope;

/// The ILP problem to solve, bundled with the parameters needed to rebuild a
/// colloscope from the solver's solution. `colloscope.rs` builds this on a
/// debounce and hands it to the solver dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IlpProblem {
    pub env: collomatique_state_colloscopes::colloscope_params::Parameters,
    pub problem: collomatique_constraints_colloscopes::ColloscopeModel,
}

pub struct Dialog {
    hidden: bool,
    is_running: bool,
    end_with_error: bool,
    worker_manager: Arc<Mutex<WorkerManager>>,
    debug_view: Controller<DebugView>,
    error_dialog: Controller<error_dialog::Dialog>,
    warning_running: Controller<warning_running::Dialog>,
    subprocess: Option<StrategySubprocess>,
    current_problem: Option<IlpProblem>,
    result_colloscope: Option<Colloscope>,
}

#[derive(Debug)]
pub enum DialogInput {
    Run(StrategyKind, IlpProblem),
    CancelRequest,
    Accept,

    Cancel,
    Echo(String),
    Finished(StrategyResult),
    SpawnError(String),
}

#[derive(Debug)]
pub enum DialogOutput {
    NewColloscope(Colloscope),
}

#[relm4::component(pub)]
impl Component for Dialog {
    type Init = Arc<Mutex<WorkerManager>>;

    type Input = DialogInput;
    type Output = DialogOutput;
    type CommandOutput = ();

    view! {
        #[root]
        adw::Window {
            set_modal: true,
            set_default_size: (700, 400),
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Résolution du colloscope"),
            add_css_class: "devel",

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        set_label: "Annuler",
                        set_sensitive: true,
                        connect_clicked => DialogInput::CancelRequest,
                    },
                    pack_end = &gtk::Button {
                        set_label: "Valider les modifications",
                        #[watch]
                        set_sensitive: !model.is_running,
                        add_css_class: "destructive-action",
                        connect_clicked => DialogInput::Accept,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_vexpand: true,
                    adw::Spinner {
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_size_request: (50, 50),
                        #[watch]
                        set_visible: model.is_running,
                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        #[watch]
                        set_visible: !model.is_running && !model.end_with_error,
                        gtk::Image::from_icon_name("emblem-ok-symbolic") {
                            set_size_request: (50, 50),
                            set_icon_size: gtk::IconSize::Large,
                        },
                        gtk::Label {
                            set_label: "Exécution terminée",
                        },
                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        #[watch]
                        set_visible: !model.is_running && model.end_with_error,
                        gtk::Image::from_icon_name("dialog-error-symbolic") {
                            set_size_request: (50, 50),
                            set_icon_size: gtk::IconSize::Large,
                        },
                        gtk::Label {
                            set_label: "Erreur pendant l'exécution",
                        },
                    },
                    gtk::Box {
                        set_margin_all: 5,
                        set_hexpand: true,
                        set_vexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: "Informations de débogage :",
                        },
                        append = model.debug_view.widget(),
                    },
                }
            }
        }
    }

    fn init(
        worker_manager: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let error_dialog = error_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .detach();

        let warning_running = warning_running::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                warning_running::DialogOutput::Accept => DialogInput::Cancel,
            });

        let debug_view = DebugView::builder().launch(()).detach();

        let model = Dialog {
            hidden: true,
            is_running: false,
            end_with_error: false,
            worker_manager,
            debug_view,
            error_dialog,
            warning_running,
            subprocess: None,
            current_problem: None,
            result_colloscope: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            DialogInput::Run(strategy, ilp_problem) => {
                self.hidden = false;
                self.is_running = true;
                self.end_with_error = false;
                self.result_colloscope = None;
                self.debug_view.emit(DebugViewInput::Clear);

                let (desc, _var_order) = ilp_problem.problem.problem().get_desc();
                self.current_problem = Some(ilp_problem);

                let input = sender.input_sender().clone();
                let log_input = input.clone();
                let result_input = input.clone();
                let log_cb = move |line: &str| {
                    log_input.emit(DialogInput::Echo(line.trim_end().to_owned()));
                };
                let progress_cb = move |progress| {
                    // TEMPORARY: route strategy progress to stderr until structured
                    // UI reporting lands.
                    match progress {
                        Ok(collomatique_strategies::StrategyProgress::Default(p)) => {
                            eprintln!(
                                "  [strategy] obj={:.4} bound={:.4} nodes={} solutions={}",
                                p.best_obj, p.best_bound, p.node_count, p.solutions_found
                            );
                        }
                        Err(e) => eprintln!("  [strategy] [progress error] {e}"),
                    }
                };
                let result_cb = move |result: StrategyResult| {
                    result_input.emit(DialogInput::Finished(result));
                };

                let spawn_result = {
                    let mut wm = self.worker_manager.lock().unwrap();
                    StrategySubprocess::spawn(
                        &mut wm,
                        desc,
                        strategy,
                        result_cb,
                        progress_cb,
                        log_cb,
                    )
                };

                match spawn_result {
                    Ok(handle) => self.subprocess = Some(handle),
                    Err(e) => sender.input(DialogInput::SpawnError(e)),
                }
            }
            DialogInput::CancelRequest => {
                if self.is_running {
                    self.warning_running
                        .sender()
                        .send(warning_running::DialogInput::Show)
                        .unwrap();
                } else {
                    sender.input(DialogInput::Cancel);
                }
            }
            DialogInput::Cancel => {
                self.hidden = true;
                if let Some(subprocess) = self.subprocess.take() {
                    let wm = self.worker_manager.lock().unwrap();
                    let _ = subprocess.kill(&wm);
                }
            }
            DialogInput::Echo(line) => {
                self.debug_view
                    .emit(DebugViewInput::Append(format!("{line}\n")));
            }
            DialogInput::Finished(result) => {
                self.is_running = false;
                self.subprocess = None;
                match self.rebuild_colloscope(&result) {
                    Some(colloscope) => self.result_colloscope = Some(colloscope),
                    None => self.end_with_error = true,
                }
            }
            DialogInput::SpawnError(error) => {
                self.is_running = false;
                self.subprocess = None;
                self.end_with_error = true;
                self.error_dialog
                    .sender()
                    .send(error_dialog::DialogInput::Show(error))
                    .unwrap();
            }
            DialogInput::Accept => {
                self.hidden = true;
                if let Some(colloscope) = self.result_colloscope.take() {
                    sender
                        .output(DialogOutput::NewColloscope(colloscope))
                        .unwrap();
                }
            }
        }
    }
}

impl Dialog {
    /// Rebuild a colloscope from a strategy result, mapping the raw internal-variable
    /// solution back to base variables. Returns `None` when the strategy produced no
    /// usable solution or the reconstruction fails.
    fn rebuild_colloscope(&self, result: &StrategyResult) -> Option<Colloscope> {
        if matches!(
            result.status,
            StrategyStatus::Error | StrategyStatus::Infeasible
        ) {
            return None;
        }
        let problem = self.current_problem.as_ref()?;
        let solution = result.solution.as_ref()?;

        let (_, var_order) = problem.problem.problem().get_desc();
        let config_data = collomatique_ilp::solution_to_config_data(solution, &var_order);
        let sol = problem.problem.solution_from_complete_data(config_data)?;
        let base_config = sol.get_data();
        collomatique_constraints_colloscopes::convert::build_colloscope(&problem.env, &base_config)
    }
}
