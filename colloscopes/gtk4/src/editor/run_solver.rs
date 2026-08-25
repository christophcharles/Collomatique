use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, ToggleButtonExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{Component, ComponentController, adw, gtk};
use relm4::{ComponentParts, ComponentSender, Controller, RelmWidgetExt};

use std::marker::PhantomData;
use std::time::{Duration, Instant};

use collomatique_ilp::{ConfigData, UsableData};
use collomatique_ilp_modeler::{InternalVar, Model};
use collomatique_strategies::{
    ConductorPayload, ConductorProgress, ConductorStatus, ConductorStrategy, Solution, SolveStatus,
    SolveVerdict, Strategy, StrategyKind, StrategyOutcome, StrategyProgressData,
    VarOrderSerializable,
};
use collomatique_subprocesses::{EngineExe, StrategySubprocess};

pub mod conductor_config;
mod error_dialog;
mod strategy_display;
pub mod strategy_extension;
mod warning_icon;
mod warning_running;

use crate::widgets::debug_view::{DebugView, DebugViewInput};
use strategy_display::{StrategyDisplayInput, StrategyFrame};
use warning_icon::WarningIcon;

/// Caller-supplied texts of the dialog: the window title, and the body of the confirmation shown
/// when the user cancels a still-running solve (it names what is being thrown away, which only the
/// caller knows — the colloscope resolution and the group-list generation discard different things).
#[derive(Debug, Clone)]
pub struct DialogSettings {
    pub title: String,
    pub cancel_warning: String,
}

pub struct Dialog<B: UsableData, E: UsableData, C: UsableData> {
    hidden: bool,
    move_front: bool,
    is_running: bool,
    // True while the (slow, off-thread) `StrategySubprocess::spawn` is in flight; the view shows
    // a dedicated "Initialisation..." screen and hides the normal solve content meanwhile.
    initializing: bool,
    end_with_error: bool,
    // What the last finished run amounts to, `None` until one finishes. Computed by
    // `collomatique_strategies::verdict` rather than here, so the scripting api reports the same
    // thing about the same run.
    verdict: Option<SolveVerdict>,
    show_debug: bool,
    global_debug_view: Controller<DebugView>,
    title: String,
    last_line: String,
    worker_strategies: Vec<Option<StrategyKind>>,
    displayed_worker: Option<u32>,
    strategy_frames: FactoryVecDeque<StrategyFrame>,
    report_errors: FactoryVecDeque<WarningIcon>,
    worker_dropdown: Controller<crate::widgets::droplist::Widget>,
    error_dialog: Controller<error_dialog::Dialog>,
    warning_running: Controller<warning_running::Dialog>,
    warning_validate: Controller<warning_running::Dialog>,
    subprocess: Option<StrategySubprocess>,
    conductor_status: ConductorStatus<InternalVar<B, E>>,
    // Elapsed-time bookkeeping for the whole solve. `run_start` doubles as the tick-generation
    // epoch; `run_end` freezes the total once the solve stops.
    run_start: Option<Instant>,
    run_end: Option<Instant>,
    // Instant the best solution was last improved; its end point is the shared `run_end`, so the
    // "time since the best solution" clock freezes when the solve stops. `None` before any solution.
    best_solution_at: Option<Instant>,
    _phantom: PhantomData<fn() -> C>,
}

#[derive(Debug)]
pub enum DialogInput<B: UsableData, E: UsableData, C: UsableData> {
    Run(ConductorStrategy, Model<B, E, C>, ConductorPayload<B>),
    CancelRequest,
    AcceptRequest,
    Accept,

    Cancel,
    Echo(String),
    WorkerEcho(u32, String),
    WorkerAssigned(u32, Option<StrategyKind>),
    StrategyUpdate(u32, StrategyProgressData),
    ConductorStatus(ConductorStatus<InternalVar<B, E>>),
    SelectWorker(Option<usize>),
    Finished(StrategyOutcome<InternalVar<B, E>>),
    ToggleDebug(bool),
    ReportError(String),
    SpawnError(String),
    /// One of this window's own dialogs just closed: bring this window back to
    /// the front.
    Present,
}

#[derive(Debug)]
pub enum DialogOutput<B: UsableData, E: UsableData> {
    NewConfig(ConfigData<InternalVar<B, E>>),
    /// This window just closed: whoever owns the window underneath should bring
    /// it back to the front, because Windows will not do it on its own.
    PresentParent,
}

/// Outputs of the dialog's background commands.
///
/// `Tick` is the periodic elapsed-time refresh; it carries the run's start instant as a
/// generation epoch so a stale tick from a previous run is dropped rather than reviving the loop.
/// `SpawnResult` carries the outcome of the off-thread `StrategySubprocess::spawn`.
///
/// `Debug` is implemented by hand because `StrategySubprocess` is not `Debug` (its `Worker`
/// holds a `Box<dyn Write + Send>`), yet relm4 requires `CommandOutput: Debug`.
pub enum DialogCommandOutput {
    Tick(Instant),
    SpawnResult(Result<StrategySubprocess, String>),
}

impl std::fmt::Debug for DialogCommandOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DialogCommandOutput::Tick(i) => f.debug_tuple("Tick").field(i).finish(),
            DialogCommandOutput::SpawnResult(Ok(_)) => write!(f, "SpawnResult(Ok(_))"),
            DialogCommandOutput::SpawnResult(Err(e)) => f
                .debug_tuple("SpawnResult")
                .field(&Err::<(), _>(e))
                .finish(),
        }
    }
}

#[relm4::component(pub)]
impl<B, E, C> Component for Dialog<B, E, C>
where
    B: UsableData + 'static,
    E: UsableData + 'static,
    C: UsableData + 'static,
{
    type Init = DialogSettings;

    type Input = DialogInput<B, E, C>;
    type Output = DialogOutput<B, E>;
    type CommandOutput = DialogCommandOutput;

    view! {
        #[root]
        root_window = adw::Window {
            set_modal: true,
            set_default_size: (700, 400),
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some(model.title.as_str()),

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        set_label: "Annuler",
                        set_sensitive: true,
                        set_tooltip: "Abandonner la résolution",
                        connect_clicked => DialogInput::CancelRequest,
                    },
                    pack_end = &gtk::Button {
                        set_label: "Valider les modifications",
                        #[watch]
                        set_sensitive: model.conductor_status.best_solution.is_some(),
                        add_css_class: "destructive-action",
                        set_tooltip: "Utiliser la meilleure solution trouvée",
                        connect_clicked => DialogInput::AcceptRequest,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_vexpand: true,
                    // Shown while the worker subprocess is being spawned off-thread; all the
                    // normal solve content below is hidden meanwhile (gated on `!initializing`).
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_vexpand: true,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_spacing: 5,
                        #[watch]
                        set_visible: model.initializing,
                        adw::Spinner {
                            set_size_request: (64, 64),
                        },
                        gtk::Label {
                            set_margin_top: 15,
                            set_label: "Initialisation...",
                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                        },
                    },
                    gtk::Box {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_margin_all: 0,
                        #[watch]
                        set_visible: !model.initializing && model.displayed_worker.is_none() && !model.show_debug,
                        gtk::Frame {
                            set_margin_all: 5,
                            set_hexpand: true,
                            set_vexpand: true,
                            gtk::Box {
                                set_margin_all: 0,
                                set_hexpand: true,
                                set_vexpand: true,
                                set_orientation: gtk::Orientation::Vertical,
                                gtk::Box {
                                    set_margin_all: 0,
                                    set_hexpand: true,
                                    set_vexpand: true,
                                    set_orientation: gtk::Orientation::Horizontal,
                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_halign: gtk::Align::Center,
                                        set_valign: gtk::Align::Center,
                                        set_spacing: 5,
                                        set_size_request: (350,-1),
                                        #[watch]
                                        set_visible: model.is_running,
                                        adw::Spinner {
                                            set_size_request: (60, 60),
                                        },
                                        gtk::Label {
                                            set_justify: gtk::Justification::Center,
                                            set_margin_top: 15,
                                            set_label: "Exécution en cours",
                                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                                        },
                                        gtk::Label {
                                            add_css_class: "monospace",
                                            set_margin_top: 10,
                                            #[watch]
                                            set_label: &model.global_elapsed(),
                                        },
                                    },
                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_halign: gtk::Align::Center,
                                        set_valign: gtk::Align::Center,
                                        set_spacing: 5,
                                        set_size_request: (350,-1),
                                        #[watch]
                                        set_visible: !model.is_running && !model.end_with_error,
                                        gtk::Image::from_icon_name("object-select-symbolic") {
                                            set_size_request: (60, 60),
                                            set_pixel_size: 60,
                                        },
                                        gtk::Label {
                                            set_justify: gtk::Justification::Center,
                                            set_margin_top: 15,
                                            set_label: "Exécution terminée",
                                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                                        },
                                        gtk::Label {
                                            add_css_class: "monospace",
                                            set_margin_top: 10,
                                            #[watch]
                                            set_label: &model.global_elapsed(),
                                        },
                                    },
                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_halign: gtk::Align::Center,
                                        set_valign: gtk::Align::Center,
                                        set_spacing: 5,
                                        set_size_request: (350,-1),
                                        #[watch]
                                        set_visible: !model.is_running && model.end_with_error,
                                        gtk::Image::from_icon_name("dialog-error-symbolic") {
                                            set_size_request: (60, 60),
                                            set_pixel_size: 60,
                                        },
                                        gtk::Label {
                                            set_justify: gtk::Justification::Center,
                                            set_margin_top: 15,
                                            set_label: "Erreur pendant l'exécution",
                                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                                        },
                                        gtk::Label {
                                            add_css_class: "monospace",
                                            set_margin_top: 10,
                                            #[watch]
                                            set_label: &model.global_elapsed(),
                                        },
                                    },
                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_halign: gtk::Align::Start,
                                        set_valign: gtk::Align::Center,
                                        set_spacing: 5,
                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Horizontal,
                                            gtk::Label {
                                                set_label: "Meilleur coût trouvé : ",
                                                set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                                            },
                                            gtk::Label {
                                                #[watch]
                                                set_label: &model.best_found_cost(),
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
                                                set_label: &model.best_possible_cost(),
                                            },
                                        },
                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Horizontal,
                                            gtk::Label {
                                                set_label: "Meilleure solution depuis : ",
                                                set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                                            },
                                            gtk::Label {
                                                add_css_class: "monospace",
                                                #[watch]
                                                set_label: &model.best_solution_elapsed(),
                                            },
                                        },
                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Horizontal,
                                            gtk::Label {
                                                set_label: "Tâches actives : ",
                                                set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                                            },
                                            gtk::Label {
                                                #[watch]
                                                set_label: &format!(
                                                    "{}/{}",
                                                    model.worker_strategies.iter().filter(|x| x.is_some()).count(),
                                                    model.worker_strategies.len(),
                                                ),
                                            },
                                        },
                                        // One label for the three sentences that were three labels:
                                        // only ever one of them could be visible at a time.
                                        gtk::Label {
                                            set_margin_top: 15,
                                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                                            #[watch]
                                            set_label: model.verdict.map_or("", collomatique_ui_text::solver::solve_verdict_text),
                                            #[watch]
                                            set_visible: !model.is_running && model.verdict.is_some(),
                                        },
                                    },
                                    gtk::Box {
                                        set_hexpand: true,
                                    },
                                },
                                gtk::Box {
                                    set_hexpand: true,
                                    gtk::Label {
                                        set_hexpand: true,
                                        set_justify: gtk::Justification::Center,
                                        set_margin_all: 5,
                                        add_css_class: "dimmed",
                                        add_css_class: "monospace",
                                        #[watch]
                                        set_label: &model.last_line,
                                    },
                                },
                            },
                        },
                    },
                    gtk::Box {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_margin_all: 0,
                        #[watch]
                        set_visible: !model.initializing && model.displayed_worker.is_none() && model.show_debug,
                        append: model.global_debug_view.widget(),
                    },
                    #[local_ref]
                    strategy_frames_stack -> gtk::Stack {
                        set_hexpand: true,
                        set_vexpand: true,
                        #[watch]
                        set_visible: !model.initializing && model.displayed_worker.is_some(),
                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_hexpand: true,
                        set_margin_all: 5,
                        set_spacing: 10,
                        #[watch]
                        set_visible: !model.initializing,
                        append: model.worker_dropdown.widget(),
                        #[local_ref]
                        report_errors_box -> gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_valign: gtk::Align::Center,
                            set_spacing: 5,
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_valign: gtk::Align::Center,
                            set_spacing: 5,
                            #[watch]
                            set_visible: model.is_running,
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
                            gtk::Image::from_icon_name("object-select-symbolic") {
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
                        #[name(terminal_toggle)]
                        gtk::ToggleButton {
                            set_icon_name: "utilities-terminal-symbolic",
                            // Block the `toggled` handler while we set `active` programmatically:
                            // otherwise the setter re-emits `toggled`, which re-sends `ToggleDebug`,
                            // which sets `active` again — an infinite loop under rapid clicking.
                            // `#[track]` keeps the setter (and its update) from running for nothing.
                            #[track(terminal_toggle.is_active() != model.show_debug)]
                            #[block_signal(toggled_handler)]
                            set_active: model.show_debug,
                            set_tooltip: "Afficher/Cacher la sortie de débogage",
                            connect_toggled[sender] => move |btn| {
                                sender.input(DialogInput::ToggleDebug(btn.is_active()));
                            } @toggled_handler,
                        },
                    },
                }
            }
        }
    }

    fn init(
        settings: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let error_dialog = error_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                error_dialog::DialogOutput::PresentParent => DialogInput::Present,
            });

        let warning_running = warning_running::Dialog::builder()
            .transient_for(&root)
            .launch(settings.cancel_warning)
            .forward(sender.input_sender(), |msg| match msg {
                warning_running::DialogOutput::Accept => DialogInput::Cancel,
                warning_running::DialogOutput::PresentParent => DialogInput::Present,
            });

        let warning_validate = warning_running::Dialog::builder()
            .transient_for(&root)
            .launch(
                "Le résolveur n'a pas encore pu prouver l'optimalité de la solution".to_string(),
            )
            .forward(sender.input_sender(), |msg| match msg {
                warning_running::DialogOutput::Accept => DialogInput::Accept,
                warning_running::DialogOutput::PresentParent => DialogInput::Present,
            });

        let strategy_frames = FactoryVecDeque::builder()
            .launch(gtk::Stack::default())
            .detach();

        let report_errors = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .detach();

        let worker_dropdown = crate::widgets::droplist::Widget::builder()
            .launch(crate::widgets::droplist::WidgetParams {
                initial_list: Vec::new(),
                initial_selected: None,
                enable_search: false,
                width_request: 200,
            })
            .forward(sender.input_sender(), |msg| match msg {
                crate::widgets::droplist::WidgetOutput::SelectionChanged(num) => {
                    DialogInput::SelectWorker(num)
                }
            });

        let global_debug_view = DebugView::builder().launch(()).detach();

        let model = Dialog {
            hidden: true,
            move_front: false,
            is_running: false,
            initializing: false,
            end_with_error: false,
            verdict: None,
            show_debug: false,
            global_debug_view,
            title: settings.title,
            last_line: String::new(),
            worker_strategies: Vec::new(),
            displayed_worker: None,
            strategy_frames,
            report_errors,
            worker_dropdown,
            error_dialog,
            warning_running,
            warning_validate,
            subprocess: None,
            conductor_status: ConductorStatus {
                best_solution: None,
                best_bound: None,
            },
            run_start: None,
            run_end: None,
            best_solution_at: None,
            _phantom: PhantomData,
        };

        let strategy_frames_stack = model.strategy_frames.widget();
        let report_errors_box = model.report_errors.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.move_front = false;
        match msg {
            DialogInput::Run(strategy, model, payload) => {
                self.hidden = false;
                self.move_front = true;
                self.is_running = true;
                self.initializing = true;
                self.end_with_error = false;
                self.verdict = None;
                self.show_debug = false;
                self.last_line = String::new();

                // Start the elapsed-time clock and kick off the periodic refresh loop.
                let epoch = Instant::now();
                self.run_start = Some(epoch);
                self.run_end = None;
                self.best_solution_at = None;
                sender.oneshot_command(async move {
                    tokio::time::sleep(REFRESH_INTERVAL).await;
                    DialogCommandOutput::Tick(epoch)
                });

                self.conductor_status = ConductorStatus {
                    best_solution: None,
                    best_bound: None,
                };
                self.worker_strategies = vec![None; strategy.worker_count.get() as usize];
                crate::tools::factories::update_vec_deque(
                    &mut self.strategy_frames,
                    0..strategy.worker_count.get(),
                    StrategyDisplayInput::Reset,
                );
                self.report_errors.guard().clear();
                self.displayed_worker = None;
                self.worker_dropdown
                    .sender()
                    .send(crate::widgets::droplist::WidgetInput::UpdateList(
                        self.dropdown_labels(),
                        Some(Self::worker_num_to_selected(self.displayed_worker)),
                    ))
                    .unwrap();
                self.global_debug_view.emit(DebugViewInput::Clear);

                // Building the model description, serializing it and spawning the worker
                // subprocess takes over a second of blocking work. Run it on the blocking
                // thread pool so the UI stays responsive and shows the "Initialisation..."
                // screen; the resulting handle (or error) comes back as a `SpawnResult`
                // command output. See `update_cmd`.
                let input = sender.input_sender().clone();
                sender.spawn_oneshot_command(move || {
                    let log_input = input.clone();
                    let progress_input = input.clone();
                    let result_input = input.clone();
                    let log_cb = move |line: &str| {
                        log_input.emit(DialogInput::Echo(line.to_owned()));
                    };
                    // The conductor hands us typed per-worker progress
                    let (_, progress_var_order) = model.to_desc();
                    let progress_cb =
                        move |progress: Result<ConductorProgress<InternalVar<B, E>>, String>| {
                            match progress {
                                Ok(ConductorProgress::Conductor(status)) => {
                                    progress_input.emit(DialogInput::ConductorStatus(status));
                                }
                                Ok(ConductorProgress::WorkerProgress {
                                    worker_num,
                                    progress,
                                }) => {
                                    let data = VarOrderSerializable::into_data(
                                        &*progress,
                                        &progress_var_order,
                                    )
                                    .unwrap_or_else(|e| match e {});
                                    progress_input
                                        .emit(DialogInput::StrategyUpdate(worker_num, data));
                                }
                                Ok(ConductorProgress::WorkerEcho { worker_num, echo }) => {
                                    progress_input.emit(DialogInput::WorkerEcho(worker_num, echo));
                                }
                                Ok(ConductorProgress::WorkerAssigned {
                                    worker_num,
                                    strategy,
                                }) => {
                                    progress_input.emit(DialogInput::WorkerAssigned(
                                        worker_num,
                                        strategy.map(|b| *b),
                                    ));
                                }
                                // A top-level Err is not attributable to a specific worker
                                // (we can't decode which one), so surface it as a warning icon
                                // next to the dropdown rather than dropping it.
                                Err(e) => {
                                    progress_input.emit(DialogInput::ReportError(e));
                                }
                            }
                        };
                    let result_cb = move |outcome: StrategyOutcome<InternalVar<B, E>>| {
                        result_input.emit(DialogInput::Finished(outcome));
                    };

                    let spawn_result = StrategySubprocess::spawn(
                        &EngineExe::Current,
                        &model,
                        &strategy,
                        None,
                        payload,
                        result_cb,
                        progress_cb,
                        log_cb,
                    );

                    DialogCommandOutput::SpawnResult(spawn_result.map_err(|e| e.to_string()))
                });
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
            DialogInput::AcceptRequest => {
                if self.is_running {
                    self.warning_validate
                        .sender()
                        .send(warning_running::DialogInput::Show)
                        .unwrap();
                } else {
                    sender.input(DialogInput::Accept);
                }
            }
            DialogInput::Cancel => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                }
                self.is_running = false;
                self.run_end = Some(Instant::now());
                if let Some(subprocess) = self.subprocess.take() {
                    subprocess.kill();
                }
            }
            DialogInput::Echo(line) => {
                self.last_line = truncate_line(line.trim_end());
                self.global_debug_view.emit(DebugViewInput::Append(line));
            }
            DialogInput::WorkerEcho(worker_num, line) => {
                self.strategy_frames
                    .send(worker_num as usize, StrategyDisplayInput::Echo(line));
            }
            DialogInput::WorkerAssigned(worker_num, assignment) => {
                self.worker_strategies[worker_num as usize] = assignment.clone();
                self.strategy_frames.send(
                    worker_num as usize,
                    StrategyDisplayInput::Assigned(assignment),
                );
                self.worker_dropdown
                    .sender()
                    .send(crate::widgets::droplist::WidgetInput::UpdateList(
                        self.dropdown_labels(),
                        Some(Self::worker_num_to_selected(self.displayed_worker)),
                    ))
                    .unwrap();
            }
            DialogInput::StrategyUpdate(worker_num, progress) => {
                self.strategy_frames.send(
                    worker_num as usize,
                    StrategyDisplayInput::StrategyUpdate(progress),
                );
            }
            DialogInput::ConductorStatus(status) => {
                self.note_best_solution(&status);
                self.conductor_status = status;
            }
            DialogInput::SelectWorker(selected) => {
                if let Some(selected) = selected {
                    let display_worker = Self::selected_to_worker_num(selected);
                    if self.displayed_worker == display_worker {
                        return;
                    }
                    self.displayed_worker = display_worker;
                    if let Some(worker_num) = display_worker {
                        self.strategy_frames
                            .widget()
                            .set_visible_child_name(&worker_num.to_string());
                    }
                }
            }
            DialogInput::Finished(outcome) => {
                self.is_running = false;
                self.run_end = Some(Instant::now());
                self.subprocess = None;

                // Before the incumbent is taken out of the outcome below: the verdict is read
                // from the whole of it, solution included.
                self.verdict = Some(collomatique_strategies::verdict(&outcome));

                let usable =
                    !matches!(outcome.status, SolveStatus::Error | SolveStatus::Infeasible);
                let best_solution = if usable {
                    outcome
                        .solution
                        .zip(outcome.objective)
                        .map(|(config, objective)| Solution { config, objective })
                } else {
                    None
                };
                if outcome.status == SolveStatus::Error {
                    self.end_with_error = true;
                }
                let status = ConductorStatus {
                    best_solution,
                    best_bound: outcome.best_bound,
                };
                self.note_best_solution(&status);
                self.conductor_status = status;
            }
            DialogInput::ToggleDebug(active) => {
                if self.show_debug == active {
                    return;
                }
                self.show_debug = active;
                for i in 0..self.strategy_frames.len() {
                    self.strategy_frames
                        .send(i, StrategyDisplayInput::ToggleDebug(active));
                }
            }
            DialogInput::ReportError(message) => {
                self.report_errors.guard().push_back(message);
            }
            DialogInput::SpawnError(error) => {
                self.is_running = false;
                self.run_end = Some(Instant::now());
                self.subprocess = None;
                self.end_with_error = true;
                self.error_dialog
                    .sender()
                    .send(error_dialog::DialogInput::Show(error))
                    .unwrap();
            }
            DialogInput::Accept => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                }
                self.is_running = false;
                if let Some(subprocess) = self.subprocess.take() {
                    subprocess.kill();
                }
                if let Some(solution) = self.conductor_status.best_solution.take() {
                    sender
                        .output(DialogOutput::NewConfig(solution.config))
                        .unwrap();
                }
            }
            DialogInput::Present => {
                self.move_front = true;
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        self.move_front = false;
        match msg {
            DialogCommandOutput::Tick(epoch) => {
                // Drop stale ticks from a previous run and let the loop die once the solve has
                // stopped. Running this handler is itself enough to re-render the global
                // `#[watch]` timer label.
                if !self.is_running || self.run_start != Some(epoch) {
                    return;
                }
                // Fan the refresh out to every worker frame so their timer labels recompute too.
                for i in 0..self.strategy_frames.len() {
                    self.strategy_frames.send(i, StrategyDisplayInput::Refresh);
                }
                sender.oneshot_command(async move {
                    tokio::time::sleep(REFRESH_INTERVAL).await;
                    DialogCommandOutput::Tick(epoch)
                });
            }
            DialogCommandOutput::SpawnResult(result) => {
                self.initializing = false;
                match result {
                    // If the solve was cancelled during initialization,
                    // `hidden` is true; don't store (and thus leak) the worker —
                    // kill it immediately.
                    Ok(handle) if !self.hidden => self.subprocess = Some(handle),
                    Ok(handle) => handle.kill(),
                    Err(e) if !self.hidden => sender.input(DialogInput::SpawnError(e)),
                    Err(_e) => {} // Ignore message if the dialog was hidden during init
                }
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
    }
}

impl<B: UsableData, E: UsableData, C: UsableData> Dialog<B, E, C> {
    fn dropdown_labels(&self) -> Vec<String> {
        std::iter::once("Vue générale".to_string())
            .chain(
                self.worker_strategies
                    .iter()
                    .enumerate()
                    .map(|(i, kind)| match kind {
                        Some(k) => format!("Tâche {} : {}", i + 1, k.ui_name()),
                        None => format!("Tâche {}", i + 1),
                    }),
            )
            .collect()
    }

    fn selected_to_worker_num(selected: usize) -> Option<u32> {
        if selected == 0 {
            None
        } else {
            Some((selected - 1) as u32)
        }
    }

    fn worker_num_to_selected(num: Option<u32>) -> usize {
        match num {
            None => 0,
            Some(i) => (i + 1) as usize,
        }
    }

    fn best_found_cost(&self) -> String {
        match &self.conductor_status.best_solution {
            Some(sol) => format!("{:.1}", sol.objective),
            None => "-".to_string(),
        }
    }

    fn best_possible_cost(&self) -> String {
        match self.conductor_status.best_bound {
            Some(bound) => format!("{:.1}", bound),
            None => "-".to_string(),
        }
    }

    /// Elapsed wall-clock time for the whole solve, `HH:MM:SS`: live while running, frozen to the
    /// total once stopped, `00:00:00` before any run.
    fn global_elapsed(&self) -> String {
        let d = match (self.run_start, self.run_end) {
            (Some(s), Some(e)) => e.saturating_duration_since(s),
            (Some(s), None) => s.elapsed(),
            _ => Duration::ZERO,
        };
        format_elapsed(d)
    }

    /// Stamp `best_solution_at` if the incoming status carries a newly-found or improved best
    /// solution; clear it if there is no best solution; leave it running otherwise (e.g. when only
    /// `best_bound` moved). Objectives are compared as copied `Option<f64>` to sidestep borrows.
    fn note_best_solution(&mut self, status: &ConductorStatus<InternalVar<B, E>>) {
        let old_obj = self
            .conductor_status
            .best_solution
            .as_ref()
            .map(|s| s.objective);
        let new_obj = status.best_solution.as_ref().map(|s| s.objective);
        if new_obj.is_none() {
            self.best_solution_at = None;
        } else if old_obj != new_obj {
            self.best_solution_at = Some(Instant::now());
        }
    }

    /// Elapsed time since the best solution was last improved, `HH:MM:SS`: live while running,
    /// frozen once stopped, `-` (as for the cost fields) while no solution has been found.
    fn best_solution_elapsed(&self) -> String {
        match (self.best_solution_at, self.run_end) {
            (Some(s), Some(e)) => format_elapsed(e.saturating_duration_since(s)),
            (Some(s), None) => format_elapsed(s.elapsed()),
            _ => "-".to_string(),
        }
    }
}

/// How often the elapsed-time timers refresh while a solve is running. Kept well under a second
/// so no whole displayed second is ever skipped despite scheduling jitter; adjust freely.
const REFRESH_INTERVAL: Duration = Duration::from_millis(250);

/// Format a duration as `HH:MM:SS`. Shared by the global dialog timer and the per-worker frames
/// (via `super::format_elapsed`).
fn format_elapsed(duration: Duration) -> String {
    let secs = duration.as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

fn truncate_line(line: &str) -> String {
    const MAX_LEN: usize = 80;

    let total = line.chars().count();
    if total <= MAX_LEN {
        return line.to_string();
    }

    // Reserve 3 chars for the dots, then split the rest.
    let available = MAX_LEN.saturating_sub(3);
    let head_len = available - available / 2; // ceil
    let tail_len = available / 2; // floor

    let head_end = line
        .char_indices()
        .nth(head_len)
        .map(|(i, _)| i)
        .unwrap_or(line.len());
    let tail_start = line
        .char_indices()
        .nth(total - tail_len)
        .map(|(i, _)| i)
        .unwrap_or(line.len());

    format!("{}...{}", &line[..head_end], &line[tail_start..])
}
