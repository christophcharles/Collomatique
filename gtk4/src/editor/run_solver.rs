use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{Component, ComponentController, adw, gtk};
use relm4::{ComponentParts, ComponentSender, Controller, RelmWidgetExt};

use std::marker::PhantomData;

use collomatique_ilp::{ConfigData, UsableData};
use collomatique_ilp_modeler::{InternalVar, Model};
use collomatique_strategies::{
    ConductorProgress, ConductorStrategy, SerializableProgress, SolveStatus, Strategy,
    StrategyKind, StrategyOutcome, StrategyProgressData,
};
use collomatique_subprocesses::StrategySubprocess;

mod error_dialog;
mod strategy_display;
mod warning_running;

use strategy_display::{
    StrategyDisplayInput, StrategyFrame, StrategyStatusBar, StrategyStatusBarOutput,
    strategy_name_from_kind,
};

/// The conductor worker whose activity is mirrored into the dialog's frame and
/// status bar. Only this *number* is hardcoded; everything else (which strategy,
/// its name, metrics, echo) is derived from the live `ConductorProgress` stream, so
/// this generalizes to N workers later.
const DISPLAY_WORKER_NUM: u32 = 0;

pub struct Dialog<B: UsableData, E: UsableData, C: UsableData> {
    hidden: bool,
    is_running: bool,
    end_with_error: bool,
    title: String,
    worker_strategy: Option<StrategyKind>,
    strategy_frame: Controller<StrategyFrame>,
    strategy_status_bar: Controller<StrategyStatusBar>,
    error_dialog: Controller<error_dialog::Dialog>,
    warning_running: Controller<warning_running::Dialog>,
    subprocess: Option<StrategySubprocess>,
    result_config: Option<ConfigData<InternalVar<B, E>>>,
    _phantom: PhantomData<fn() -> C>,
}

#[derive(Debug)]
pub enum DialogInput<B: UsableData, E: UsableData, C: UsableData> {
    Run(ConductorStrategy, Model<B, E, C>),
    CancelRequest,
    Accept,

    Cancel,
    Echo(String),
    WorkerEcho(u32, String),
    WorkerAssigned(u32, Option<StrategyKind>),
    StrategyUpdate(u32, StrategyProgressData),
    Finished(StrategyOutcome<InternalVar<B, E>>),
    ToggleDebug(bool),
    SpawnError(String),
}

#[derive(Debug)]
pub enum DialogOutput<B: UsableData, E: UsableData> {
    NewConfig(ConfigData<InternalVar<B, E>>),
}

#[relm4::component(pub)]
impl<B, E, C> Component for Dialog<B, E, C>
where
    B: UsableData + 'static,
    E: UsableData + 'static,
    C: UsableData + 'static,
{
    type Init = String;

    type Input = DialogInput<B, E, C>;
    type Output = DialogOutput<B, E>;
    type CommandOutput = ();

    view! {
        #[root]
        adw::Window {
            set_modal: true,
            set_default_size: (700, 400),
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some(model.title.as_str()),
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
                    append = model.strategy_frame.widget(),
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_hexpand: true,
                        set_margin_all: 10,
                        set_margin_top: 0,
                        set_spacing: 10,
                        gtk::Label {
                            #[watch]
                            set_label: &model.strategy_name_label(),
                            set_valign: gtk::Align::Center,
                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                        },
                        append = model.strategy_status_bar.widget(),
                    },
                }
            }
        }
    }

    fn init(
        title: Self::Init,
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

        let strategy_frame = StrategyFrame::builder().launch(()).detach();

        let strategy_status_bar = StrategyStatusBar::builder().launch(()).forward(
            sender.input_sender(),
            |msg| match msg {
                StrategyStatusBarOutput::ToggleDebug(active) => DialogInput::ToggleDebug(active),
            },
        );

        let model = Dialog {
            hidden: true,
            is_running: false,
            end_with_error: false,
            title,
            worker_strategy: None,
            strategy_frame,
            strategy_status_bar,
            error_dialog,
            warning_running,
            subprocess: None,
            result_config: None,
            _phantom: PhantomData,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            DialogInput::Run(strategy, model) => {
                self.hidden = false;
                self.is_running = true;
                self.end_with_error = false;
                self.result_config = None;
                self.worker_strategy = None;
                self.emit_strategy(StrategyDisplayInput::Clear);

                let input = sender.input_sender().clone();
                let log_input = input.clone();
                let progress_input = input.clone();
                let result_input = input.clone();
                let log_cb = move |line: &str| {
                    log_input.emit(DialogInput::Echo(line.trim_end().to_owned()));
                };
                // The conductor hands us typed per-worker progress; mirror only the
                // displayed worker, erasing its inner progress to the form the scalar
                // display needs here at the (type-aware) Dialog boundary.
                let (_, progress_var_order) = model.to_desc();
                let progress_cb = move |progress: Result<
                    ConductorProgress<InternalVar<B, E>>,
                    String,
                >| {
                    match progress {
                        Ok(ConductorProgress::Worker {
                            worker_num,
                            progress,
                        }) => {
                            let data =
                                SerializableProgress::into_data(&*progress, &progress_var_order)
                                    .unwrap_or_else(|e| match e {});
                            progress_input.emit(DialogInput::StrategyUpdate(worker_num, data));
                        }
                        Ok(ConductorProgress::WorkerEcho { worker_num, echo }) => {
                            progress_input.emit(DialogInput::WorkerEcho(
                                worker_num,
                                echo.trim_end().to_owned(),
                            ));
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
                        // TODO: a top-level Err is not attributable to a specific worker
                        // (we can't decode which one). `todo!()` so it fails loudly rather
                        // than vanishing.
                        Err(_e) => todo!("surface non-worker-attributable strategy IPC errors"),
                        // Conductor aggregate + non-displayed workers: ignored for now.
                        _ => {}
                    }
                };
                let result_cb = move |outcome: StrategyOutcome<InternalVar<B, E>>| {
                    result_input.emit(DialogInput::Finished(outcome));
                };

                let spawn_result = StrategySubprocess::spawn(
                    &model,
                    &strategy,
                    None,
                    result_cb,
                    progress_cb,
                    log_cb,
                );

                match spawn_result {
                    Ok(handle) => self.subprocess = Some(handle),
                    Err(e) => sender.input(DialogInput::SpawnError(e.to_string())),
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
                    subprocess.kill();
                }
            }
            DialogInput::Echo(_line) => {
                // For now, sink the conductor echo
            }
            DialogInput::WorkerEcho(worker_num, line) => {
                if worker_num == DISPLAY_WORKER_NUM {
                    self.emit_strategy(StrategyDisplayInput::Echo(line));
                }
            }
            DialogInput::WorkerAssigned(worker_num, assignment) => {
                if worker_num == DISPLAY_WORKER_NUM {
                    self.worker_strategy = assignment.clone();
                    let name = assignment.as_ref().map(strategy_name_from_kind);
                    self.emit_strategy(StrategyDisplayInput::Assigned(name));
                }
            }
            DialogInput::StrategyUpdate(worker_num, progress) => {
                if worker_num == DISPLAY_WORKER_NUM {
                    self.emit_strategy(StrategyDisplayInput::StrategyUpdate(progress));
                }
            }
            DialogInput::Finished(outcome) => {
                self.is_running = false;
                self.subprocess = None;

                let usable =
                    !matches!(outcome.status, SolveStatus::Error | SolveStatus::Infeasible);
                match (usable, outcome.solution) {
                    (true, Some(config)) => {
                        self.result_config = Some(config);
                    }
                    _ => self.end_with_error = true,
                }
            }
            DialogInput::ToggleDebug(active) => {
                self.strategy_frame
                    .emit(StrategyDisplayInput::ToggleDebug(active));
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
                if let Some(config) = self.result_config.take() {
                    sender.output(DialogOutput::NewConfig(config)).unwrap();
                }
            }
        }
    }
}

impl<B: UsableData, E: UsableData, C: UsableData> Dialog<B, E, C> {
    fn emit_strategy(&self, input: StrategyDisplayInput) {
        self.strategy_frame.emit(input.clone());
        self.strategy_status_bar.emit(input);
    }

    fn strategy_name_label(&self) -> String {
        let n = DISPLAY_WORKER_NUM + 1;
        match &self.worker_strategy {
            Some(kind) => format!("Tâche {n} : {}", kind.ui_name()),
            None => format!("Tâche {n}"),
        }
    }
}
