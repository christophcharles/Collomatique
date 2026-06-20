use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{Component, ComponentController, adw, gtk};
use relm4::{ComponentParts, ComponentSender, Controller};

use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use collomatique_ilp::mat_repr::ProblemRepr;
use collomatique_ilp::{ConfigData, Problem, UsableData};
use collomatique_strategies::{StrategyKind, StrategyProgress};
use collomatique_subprocesses::{
    StrategyResult, StrategyStatus, StrategySubprocess, WorkerManager,
};

mod error_dialog;
mod strategy_display;
mod warning_running;

use strategy_display::{StrategyDisplay, StrategyDisplayInput};

pub struct Dialog<V: UsableData, C, P> {
    hidden: bool,
    is_running: bool,
    end_with_error: bool,
    title: String,
    worker_manager: Arc<Mutex<WorkerManager>>,
    strategy_display: Controller<StrategyDisplay>,
    error_dialog: Controller<error_dialog::Dialog>,
    warning_running: Controller<warning_running::Dialog>,
    subprocess: Option<StrategySubprocess>,
    var_order: Option<Vec<V>>,
    result_config: Option<ConfigData<V>>,
    _phantom: PhantomData<fn() -> (C, P)>,
}

#[derive(Debug)]
pub enum DialogInput<V: UsableData, C: UsableData, P: ProblemRepr<V>> {
    Run(StrategyKind, Problem<V, C, P>),
    CancelRequest,
    Accept,

    Cancel,
    Echo(String),
    StrategyUpdate(Result<StrategyProgress, String>),
    Finished(StrategyResult),
    SpawnError(String),
}

#[derive(Debug)]
pub enum DialogOutput<V: UsableData> {
    NewConfig(ConfigData<V>),
}

#[relm4::component(pub)]
impl<V, C, P> Component for Dialog<V, C, P>
where
    V: UsableData + 'static,
    C: UsableData + 'static,
    P: ProblemRepr<V> + 'static,
{
    type Init = (Arc<Mutex<WorkerManager>>, String);

    type Input = DialogInput<V, C, P>;
    type Output = DialogOutput<V>;
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
                    append = model.strategy_display.widget(),
                }
            }
        }
    }

    fn init(
        (worker_manager, title): Self::Init,
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

        let strategy_display = StrategyDisplay::builder().launch(()).detach();

        let model = Dialog {
            hidden: true,
            is_running: false,
            end_with_error: false,
            title,
            worker_manager,
            strategy_display,
            error_dialog,
            warning_running,
            subprocess: None,
            var_order: None,
            result_config: None,
            _phantom: PhantomData,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            DialogInput::Run(strategy, problem) => {
                self.hidden = false;
                self.is_running = true;
                self.end_with_error = false;
                self.result_config = None;
                self.strategy_display.emit(StrategyDisplayInput::Clear);

                let (desc, var_order) = problem.get_desc();
                self.var_order = Some(var_order);

                let input = sender.input_sender().clone();
                let log_input = input.clone();
                let progress_input = input.clone();
                let result_input = input.clone();
                let log_cb = move |line: &str| {
                    log_input.emit(DialogInput::Echo(line.trim_end().to_owned()));
                };
                let progress_cb = move |progress| {
                    progress_input.emit(DialogInput::StrategyUpdate(progress));
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
                self.strategy_display.emit(StrategyDisplayInput::Echo(line));
            }
            DialogInput::StrategyUpdate(progress) => {
                self.strategy_display
                    .emit(StrategyDisplayInput::StrategyUpdate(progress));
            }
            DialogInput::Finished(result) => {
                self.strategy_display.emit(StrategyDisplayInput::Finished);
                self.is_running = false;
                self.subprocess = None;

                let usable = !matches!(
                    result.status,
                    StrategyStatus::Error | StrategyStatus::Infeasible
                );
                match (usable, result.solution, self.var_order.as_ref()) {
                    (true, Some(solution), Some(var_order)) => {
                        self.result_config = Some(collomatique_ilp::solution_to_config_data(
                            &solution, var_order,
                        ));
                    }
                    _ => self.end_with_error = true,
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
                if let Some(config) = self.result_config.take() {
                    sender.output(DialogOutput::NewConfig(config)).unwrap();
                }
            }
        }
    }
}
