//! The MPS export, from the click in the *Outils avancés* panel down to the file on disk:
//! configure the model, choose the file, build the model, write it.
//!
//! Nothing here is visible. The component owns no widget of its own — its root is an
//! unattached, invisible box — and only sequences the two [`build_model`] dialogs and the file
//! chooser. It is a component rather than a handful of panel messages because it has to own
//! dialog controllers, which a relm4 `Worker` (running off the main thread) cannot do.
//!
//! [`build_model`]: crate::editor::build_model

pub mod checker_extension;

use std::path::PathBuf;

use gtk::prelude::WidgetExt;
use relm4::gtk;
use relm4::{Component, ComponentController, ComponentParts, ComponentSender, Controller};

use collomatique_constraints_colloscopes::{ConfiguredColloscopeModel, SolveConfig};
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::colloscopes::Colloscope;

use crate::editor::build_model::{config_dialog, loading_dialog};
use crate::editor::diagnostics;
use crate::tools::open_save::{self, DefaultSaveFile, ParentWindowHandle};

use checker_extension::MpsExportOptions;

/// The model-configuration dialog instantiated for the export path: its extension slot holds the
/// full/checker toggles, so the dialog hands back an [`MpsExportOptions`] alongside the
/// [`SolveConfig`].
type ConfigDialog = config_dialog::Dialog<checker_extension::Extension>;

/// Where the export in flight stands. Each step hands the next one what it still needs; there is
/// never more than one export in flight, because every step of the flow is modal.
enum Pending {
    /// The configuration dialog is up.
    Configuring {
        colloscope: Colloscope,
        default_file: DefaultSaveFile,
    },
    /// The configuration is settled and the file chooser is up.
    ChoosingFile {
        colloscope: Colloscope,
        /// The parameters the configuration was validated against.
        params: Parameters,
    },
    /// The file is chosen and the model is building.
    Building { path: PathBuf },
}

pub struct Workflow {
    /// The widget the file chooser resolves its parent window from.
    parent: gtk::Widget,
    /// Remembered across exports so a second export reopens where the first one left off, and
    /// dropped back to the defaults when another document is loaded. Kept apart from the
    /// colloscope panel's own solve configuration: the two answer different questions.
    config: SolveConfig,
    options: MpsExportOptions,
    pending: Option<Pending>,
    config_dialog: Controller<ConfigDialog>,
    loading_dialog: Controller<loading_dialog::Dialog>,
}

#[derive(Debug)]
pub enum WorkflowInput {
    /// Start an export from the document as it stands, snapshot taken by the editor at click
    /// time. The rest of the flow works off this snapshot, so later edits cannot change what is
    /// written.
    Start {
        params: Parameters,
        colloscope: Colloscope,
        default_file: DefaultSaveFile,
    },
    ConfigAccepted(SolveConfig, MpsExportOptions, Parameters),
    ConfigCancelled,
    ModelBuilt(ConfiguredColloscopeModel),
    BuildCancelled,
    /// One of this component's dialogs just closed: the window underneath must come back to the
    /// front.
    PresentParent,
    /// Another document was loaded: forget the configuration remembered from the previous one.
    Reset,
}

#[derive(Debug)]
pub enum WorkflowOutput {
    /// The model is built and the file is being written.
    Writing(PathBuf),
    Successful(PathBuf),
    Failed(PathBuf, String),
    /// A dialog of this component just closed: the window underneath should be brought back to
    /// the front, because Windows will not do it on its own.
    PresentParent,
}

#[derive(Debug)]
pub enum WorkflowCommandOutput {
    /// The file chooser answered: `None` when it was dismissed.
    FileChosen(Option<PathBuf>),
    Written(PathBuf, Result<(), String>),
}

#[relm4::component(pub)]
impl Component for Workflow {
    type Init = gtk::Widget;

    type Input = WorkflowInput;
    type Output = WorkflowOutput;
    type CommandOutput = WorkflowCommandOutput;

    view! {
        // Never attached to anything: this component exists for its dialogs alone.
        #[root]
        gtk::Box {
            set_visible: false,
        }
    }

    fn init(
        parent: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // `parent` is the advanced-tools panel, not a window: relm4 resolves the toplevel late,
        // once the widget tree is built, which is how every other dialog here gets its parent.
        let config_dialog = ConfigDialog::builder()
            .transient_for(&parent)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                config_dialog::DialogOutput::Accepted(config, options, params) => {
                    WorkflowInput::ConfigAccepted(config, options, params)
                }
                config_dialog::DialogOutput::Cancelled => WorkflowInput::ConfigCancelled,
                config_dialog::DialogOutput::PresentParent => WorkflowInput::PresentParent,
            });

        let loading_dialog = loading_dialog::Dialog::builder()
            .transient_for(&parent)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                loading_dialog::DialogOutput::ModelReady(model) => WorkflowInput::ModelBuilt(model),
                loading_dialog::DialogOutput::Cancelled => WorkflowInput::BuildCancelled,
                loading_dialog::DialogOutput::PresentParent => WorkflowInput::PresentParent,
            });

        let model = Workflow {
            parent,
            config: SolveConfig::default(),
            options: MpsExportOptions::default(),
            pending: None,
            config_dialog,
            loading_dialog,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            WorkflowInput::Start {
                params,
                colloscope,
                default_file,
            } => {
                self.pending = Some(Pending::Configuring {
                    colloscope,
                    default_file,
                });
                self.config_dialog
                    .sender()
                    .send(config_dialog::DialogInput::Show(
                        self.config.clone(),
                        self.options,
                        params,
                    ))
                    .unwrap();
            }
            WorkflowInput::ConfigAccepted(config, options, params) => {
                // Remembered for the next export, and used for this one.
                self.config = config;
                self.options = options;

                let Some(Pending::Configuring {
                    colloscope,
                    default_file,
                }) = self.pending.take()
                else {
                    return;
                };
                self.pending = Some(Pending::ChoosingFile { colloscope, params });

                let parent = ParentWindowHandle::from_widget(&self.parent);
                sender.oneshot_command(async move {
                    WorkflowCommandOutput::FileChosen(
                        open_save::save_mps_dialog(parent, default_file).await,
                    )
                });
            }
            WorkflowInput::ModelBuilt(model) => {
                let Some(Pending::Building { path }) = self.pending.take() else {
                    return;
                };

                // One build carries both problems; which one is written was settled in the
                // configuration dialog.
                let problem = if self.options.checker {
                    model.checker_problem().clone()
                } else {
                    model.problem().clone()
                };

                sender
                    .output(WorkflowOutput::Writing(path.clone()))
                    .unwrap();
                sender.oneshot_command(async move {
                    let result = diagnostics::export_to_mps(&problem, &path)
                        .await
                        .map_err(|e| e.to_string());
                    WorkflowCommandOutput::Written(path, result)
                });
            }
            WorkflowInput::ConfigCancelled | WorkflowInput::BuildCancelled => {
                // The export was abandoned before anything was written.
                self.pending = None;
            }
            WorkflowInput::PresentParent => {
                sender.output(WorkflowOutput::PresentParent).unwrap();
            }
            WorkflowInput::Reset => {
                self.config = SolveConfig::default();
                self.options = MpsExportOptions::default();
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            WorkflowCommandOutput::FileChosen(None) => {
                self.pending = None;
            }
            WorkflowCommandOutput::FileChosen(Some(path)) => {
                let Some(Pending::ChoosingFile { colloscope, params }) = self.pending.take() else {
                    return;
                };
                self.pending = Some(Pending::Building { path });
                self.loading_dialog
                    .sender()
                    .send(loading_dialog::DialogInput::Show(
                        self.config.clone(),
                        params,
                        colloscope,
                    ))
                    .unwrap();
            }
            WorkflowCommandOutput::Written(path, Ok(())) => {
                sender.output(WorkflowOutput::Successful(path)).unwrap();
            }
            WorkflowCommandOutput::Written(path, Err(error)) => {
                sender.output(WorkflowOutput::Failed(path, error)).unwrap();
            }
        }
    }
}
