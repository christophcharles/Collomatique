use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::prelude::FactoryVecDeque;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};
use relm4::{adw, gtk};

use collomatique_ops::ColloscopeUpdateOp;

use crate::editor::build_model::{config_dialog, loading_dialog};
use crate::editor::run_solver;

/// The solver dialog instantiated for the configured colloscope ILP model. The solve path runs
/// on the configured model (with pins/anchors/objectified cross-period constraints), so it is
/// parameterized by [`ConfiguredExtra`]/[`ConfiguredConstraintDesc`] rather than the base model's
/// spaces.
///
/// [`ConfiguredExtra`]: collomatique_constraints_colloscopes::ConfiguredExtra
/// [`ConfiguredConstraintDesc`]: collomatique_constraints_colloscopes::ConfiguredConstraintDesc
type SolverDialog = run_solver::Dialog<
    collomatique_constraints_colloscopes::Var,
    collomatique_constraints_colloscopes::ConfiguredExtra,
    collomatique_constraints_colloscopes::ConfiguredConstraintDesc,
>;

/// Flattened variable of the configured colloscope model (the solve path's ILP variable).
type ConfiguredInternalVar = collomatique_constraints_colloscopes::InternalVar<
    collomatique_constraints_colloscopes::Var,
    collomatique_constraints_colloscopes::ConfiguredExtra,
>;

const DEBOUNCE_DURATION: std::time::Duration = std::time::Duration::from_millis(500);

/// The model-configuration dialog instantiated for the solve path: its extension slot holds the
/// conductor-strategy widgets, so the dialog hands back a strategy alongside the [`SolveConfig`].
///
/// [`SolveConfig`]: collomatique_constraints_colloscopes::SolveConfig
type ConfigDialog = config_dialog::Dialog<run_solver::strategy_extension::Extension>;

mod blame_dialog;
mod colloscope_display;
mod group_list_dialog;
mod group_lists_display;
mod interrogation_dialog;

/// Build the incremental epoch payload from the freshly-built model: every `StudentInGroup` base
/// variable is solved first (epoch 0), then each `GroupInInterrogation` variable is solved in the
/// epoch matching its week (week + 1), so the schedule fills in week by week on top of the fixed
/// group assignment. Base variables absent from the map fall into the strategy's final epoch.
fn build_incremental_payload(
    model: &collomatique_constraints_colloscopes::ConfiguredColloscopeModel,
) -> collomatique_strategies::ConductorPayload<collomatique_constraints_colloscopes::Var> {
    let epochs = collomatique_constraints_colloscopes::build_incremental_epochs(model);
    collomatique_strategies::ConductorPayload {
        incremental: collomatique_strategies::IncrementalPayload { epochs },
    }
}

#[derive(Debug)]
pub enum ColloscopeInput {
    Update(
        collomatique_state_colloscopes::colloscope_params::Parameters,
        collomatique_state_colloscopes::colloscopes::Colloscope,
    ),

    EditGroupList(collomatique_state_colloscopes::GroupListId),
    GroupListAccepted(std::collections::BTreeMap<collomatique_state_colloscopes::StudentId, u32>),

    EditInterrogation(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::WeekId,
    ),
    InterrogationAccepted(std::collections::BTreeSet<u32>),

    SolveColloscopeClicked,
    ResetSolveConfig,
    ConductorConfigAccepted(
        collomatique_constraints_colloscopes::SolveConfig,
        collomatique_strategies::ConductorStrategy,
        collomatique_state_colloscopes::colloscope_params::Parameters,
    ),
    ConductorConfigCancelled,
    /// The model build was abandoned from the loading dialog.
    ModelBuildCancelled,
    ModelBuilt(collomatique_constraints_colloscopes::ConfiguredColloscopeModel),
    SolveResult(collomatique_ilp::ConfigData<ConfiguredInternalVar>),
    EraseColloscopeClicked,
    EraseGroupListsClicked,

    ShowBlamedConstraints,
    /// A dialog of this panel just closed. The panel hosts no window of its
    /// own, so it passes the request up to the editor.
    PresentParent,
}

#[derive(Debug)]
pub enum ColloscopeCommandOutput {
    DebounceElapsed,
    IlpProblemComputed {
        env: collomatique_state_colloscopes::colloscope_params::Parameters,
        result: Result<collomatique_constraints_colloscopes::ColloscopeModel, String>,
    },
    IlpReprComputed(IlpRepr),
}

#[derive(Debug)]
pub enum ColloscopeOutput {
    UpdateOp(ColloscopeUpdateOp),
    NewColloscope(collomatique_state_colloscopes::colloscopes::Colloscope),
    UpdateIlpProblem(Option<collomatique_constraints_colloscopes::IlpInnerProblem>),
    /// A dialog of this panel just closed: the window underneath should be
    /// brought back to the front, because Windows will not do it on its own.
    PresentParent,
}

/// The ILP problem together with the parameters needed to rebuild a colloscope
/// from a solver solution. Built on a debounce and handed to the solver dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IlpProblem {
    env: collomatique_state_colloscopes::colloscope_params::Parameters,
    problem: collomatique_constraints_colloscopes::ColloscopeModel,
}

/// The verification result for a given colloscope under a given ILP problem:
/// the colloscope it was computed for, together with the warnings it raised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IlpRepr {
    colloscope: collomatique_state_colloscopes::colloscopes::Colloscope,
    warnings: Vec<(collomatique_constraints_colloscopes::SeverityLevel, String)>,
}

/// The computation data we currently have (as opposed to the data in flight).
///
/// Building the ILP model either failed (`BuildFailed`, e.g. a database error)
/// or succeeded (`Built`); in the latter case we may additionally have verified
/// the current colloscope into an [`IlpRepr`]. Every variant retains the `env`
/// it was computed from so its staleness can be checked against the latest
/// `Update`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputationArtifact {
    BuildFailed {
        env: collomatique_state_colloscopes::colloscope_params::Parameters,
        message: String,
    },
    Built {
        ilp_problem: IlpProblem,
        ilp_repr: Option<IlpRepr>,
    },
}

impl ComputationArtifact {
    /// The parameters this artifact was computed from.
    fn env(&self) -> &collomatique_state_colloscopes::colloscope_params::Parameters {
        match self {
            ComputationArtifact::BuildFailed { env, .. } => env,
            ComputationArtifact::Built { ilp_problem, .. } => &ilp_problem.env,
        }
    }
}

/// Which computation, if any, is currently in flight. Collomatique never runs
/// two computations at once, and neither step is cancelable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InflightComputation {
    None,
    IlpProblem,
    IlpRepr,
}

/// What is currently in flight: a debounce timer, a computation, or both.
#[derive(Debug, Clone, Copy)]
pub struct InflightCommand {
    debouncing: bool,
    computation: InflightComputation,
}

pub struct Colloscope {
    params: collomatique_state_colloscopes::colloscope_params::Parameters,
    colloscope: collomatique_state_colloscopes::colloscopes::Colloscope,

    group_list_entries: FactoryVecDeque<group_lists_display::Entry>,
    group_list_dialog: Controller<group_list_dialog::Dialog>,
    colloscope_display: Controller<colloscope_display::Display>,
    interrogation_dialog: Controller<interrogation_dialog::Dialog>,
    blame_dialog: Controller<blame_dialog::Dialog>,
    run_solver_dialog: Controller<SolverDialog>,
    config_dialog: Controller<ConfigDialog>,
    loading_dialog: Controller<loading_dialog::Dialog>,
    /// The last-validated solve configuration, kept so the config dialog reopens pre-primed
    /// instead of resetting every time.
    solve_config: collomatique_constraints_colloscopes::SolveConfig,
    /// The last-validated conductor strategy, bolted onto the solve request by this UI (the
    /// modelization-only [`collomatique_constraints_colloscopes::SolveConfig`] does not carry
    /// it). Defaults to the parallel strategy.
    strategy: collomatique_strategies::ConductorStrategy,

    edited_group_list: Option<collomatique_state_colloscopes::GroupListId>,
    edited_interrogation: Option<(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::WeekId,
    )>,

    // The instant of the last `Update` signal, used to debounce recomputation.
    last_update: Option<std::time::Instant>,
    // The latest finished computation data: the best data available apart from
    // any in-flight computation. It may lag behind `params`/`colloscope`, but it
    // is never surfaced while stale — `inflight_cmd.debouncing` stays `true`
    // whenever it could be, and `check_state_and_launch_computation` reconciles
    // it against the latest `Update` (rebuilding as needed).
    computation_artifact: Option<ComputationArtifact>,
    // What is currently in flight (debounce timer and/or a computation).
    inflight_cmd: InflightCommand,
}

impl Colloscope {
    /// Whether nothing is in flight: results (if any) may now be displayed.
    fn is_settled(&self) -> bool {
        self.last_update.is_some()
            && !self.inflight_cmd.debouncing
            && self.inflight_cmd.computation == InflightComputation::None
    }

    /// The warnings to display, available only once the computation is settled
    /// on a successfully-built problem with a verified colloscope.
    fn settled_warnings(
        &self,
    ) -> Option<&Vec<(collomatique_constraints_colloscopes::SeverityLevel, String)>> {
        if !self.is_settled() {
            return None;
        }
        match &self.computation_artifact {
            Some(ComputationArtifact::Built {
                ilp_repr: Some(ilp_repr),
                ..
            }) => Some(&ilp_repr.warnings),
            _ => None,
        }
    }

    /// The ILP problem, available for solving/exporting. Relies on the invariant
    /// upheld by `check_state_and_launch_computation`: `debouncing` stays `true`
    /// whenever the artifact could be stale (including while a leftover
    /// computation blocks the slot), so `!debouncing && computation != IlpProblem`
    /// implies the built problem is consistent with the latest `Update`.
    fn get_ilp_problem(&self) -> Option<&IlpProblem> {
        if self.inflight_cmd.debouncing
            || self.inflight_cmd.computation == InflightComputation::IlpProblem
        {
            return None;
        }
        match &self.computation_artifact {
            Some(ComputationArtifact::Built { ilp_problem, .. }) => Some(ilp_problem),
            _ => None,
        }
    }

    fn is_debouncing(&self) -> bool {
        // Before the first `Update` there is no data yet: keep the startup
        // spinner up until the first parameters arrive.
        self.inflight_cmd.debouncing || self.last_update.is_none()
    }

    fn is_constructing_constraints(&self) -> bool {
        !self.inflight_cmd.debouncing
            && self.inflight_cmd.computation == InflightComputation::IlpProblem
    }

    fn is_rebuilding_warnings(&self) -> bool {
        !self.inflight_cmd.debouncing
            && self.inflight_cmd.computation == InflightComputation::IlpRepr
    }

    fn has_warnings(&self) -> bool {
        match self.settled_warnings() {
            Some(warnings) => !warnings.is_empty(),
            None => false,
        }
    }

    fn has_evaluation_error(&self) -> bool {
        self.is_settled()
            && matches!(
                &self.computation_artifact,
                Some(ComputationArtifact::BuildFailed { .. })
            )
    }

    fn has_success(&self) -> bool {
        match self.settled_warnings() {
            Some(warnings) => warnings.is_empty(),
            None => false,
        }
    }

    fn generate_warning_text(&self) -> String {
        match self.settled_warnings() {
            Some(warnings) => format!("<small><i>{}</i></small>", warnings.len()),
            None => String::new(),
        }
    }

    fn worst_severity_level(&self) -> Option<collomatique_constraints_colloscopes::SeverityLevel> {
        match self.settled_warnings() {
            Some(warnings) => warnings.first().map(|(s, _)| *s),
            None => None,
        }
    }

    fn warning_icon_name(&self) -> &'static str {
        use collomatique_constraints_colloscopes::SeverityLevel;
        match self.worst_severity_level() {
            Some(SeverityLevel::Infeasibility) => "computer-fail-symbolic",
            Some(SeverityLevel::Structural | SeverityLevel::Quality) => "dialog-error-symbolic",
            Some(SeverityLevel::Progressive) => "dialog-warning-symbolic",
            Some(SeverityLevel::Preference) => "dialog-information-symbolic",
            None => "dialog-warning-symbolic",
        }
    }

    fn warning_css_class(&self) -> &'static str {
        use collomatique_constraints_colloscopes::SeverityLevel;
        match self.worst_severity_level() {
            Some(
                SeverityLevel::Infeasibility | SeverityLevel::Structural | SeverityLevel::Quality,
            ) => "error",
            _ => "warning",
        }
    }
}

#[relm4::component(pub)]
impl Component for Colloscope {
    type Input = ColloscopeInput;
    type Output = ColloscopeOutput;
    type Init = ();
    type CommandOutput = ColloscopeCommandOutput;

    view! {
        #[root]
        gtk::Paned {
            set_hexpand: true,
            set_margin_all: 5,
            set_orientation: gtk::Orientation::Vertical,
            #[wrap(Some)]
            set_start_child = &gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Horizontal,
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_label: "Colloscope",
                        set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                    },
                    gtk::Button {
                        #[watch]
                        set_sensitive: !model.colloscope.are_interrogations_empty(),
                        set_icon_name: "edit-delete-symbolic",
                        add_css_class: "flat",
                        set_tooltip_text: Some("Effacer le colloscope"),
                        connect_clicked => ColloscopeInput::EraseColloscopeClicked,
                    },
                    gtk::Box {
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Horizontal,
                    },
                    gtk::Button {
                        set_margin_start: 5,
                        add_css_class: "flat",
                        set_tooltip: "Afficher les erreurs du colloscope",
                        connect_clicked => ColloscopeInput::ShowBlamedConstraints,
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 5,
                                #[watch]
                                set_visible: model.is_debouncing(),
                                adw::Spinner {
                                    set_halign: gtk::Align::Start,
                                },
                                gtk::Label {
                                    set_label: "<i><small>En attente des données...</small></i>",
                                    set_use_markup: true,
                                },
                            },
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 5,
                                #[watch]
                                set_visible: model.is_constructing_constraints(),
                                adw::Spinner {
                                    set_halign: gtk::Align::Start,
                                },
                                gtk::Label {
                                    set_label: "<i><small>Construction des contraintes...</small></i>",
                                    set_use_markup: true,
                                },
                            },
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 5,
                                #[watch]
                                set_visible: model.is_rebuilding_warnings(),
                                adw::Spinner {
                                    set_halign: gtk::Align::Start,
                                },
                                gtk::Label {
                                    set_label: "<i><small>Vérification du colloscope...</small></i>",
                                    set_use_markup: true,
                                },
                            },
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 5,
                                #[watch]
                                set_visible: model.has_success(),
                                gtk::Image {
                                    set_icon_name: Some("object-select-symbolic"),
                                },
                                gtk::Label {
                                    set_label: "<i><small>Colloscope valide</small></i>",
                                    set_use_markup: true,
                                },
                            },
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 5,
                                add_css_class: "error",
                                #[watch]
                                set_visible: model.has_evaluation_error(),
                                gtk::Image {
                                    set_icon_name: Some("dialog-error-symbolic"),
                                },
                                gtk::Label {
                                    set_label: "<b>Erreur de base de données</b>",
                                    set_use_markup: true,
                                },
                            },
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 5,
                                #[watch]
                                remove_css_class: "error",
                                #[watch]
                                remove_css_class: "warning",
                                #[watch]
                                add_css_class: model.warning_css_class(),
                                #[watch]
                                set_visible: model.has_warnings(),
                                gtk::Image {
                                    #[watch]
                                    set_icon_name: Some(model.warning_icon_name()),
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: &model.generate_warning_text(),
                                    set_use_markup: true,
                                },
                            },
                        },
                    },
                    gtk::Button {
                        add_css_class: "frame",
                        add_css_class: "accent",
                        set_margin_all: 5,
                        adw::ButtonContent {
                            set_icon_name: "system-run-symbolic",
                            set_label: "Générer le colloscope automatiquement",
                        },
                        connect_clicked => ColloscopeInput::SolveColloscopeClicked,
                    },
                },
                #[local_ref]
                colloscope_display_box -> gtk::Box {
                    set_hexpand: true,
                    set_vexpand: true,
                },
            },
            #[wrap(Some)]
            set_end_child = &gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                #[watch]
                set_visible: model
                    .params
                    .group_lists
                    .group_list_map
                    .iter()
                    .any(|(_id, group_list)| !group_list.is_prefilled()),
                gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 10,
                    gtk::Box {
                        set_margin_top: 10,
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Horizontal,
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: "Groupes à répartir",
                            set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                        },
                        gtk::Button {
                            #[watch]
                            set_sensitive: !model.colloscope.are_group_lists_empty(),
                            set_icon_name: "edit-delete-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Effacer les listes de groupes"),
                            connect_clicked => ColloscopeInput::EraseGroupListsClicked,
                        },
                    },
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                        gtk::Box {
                            set_hexpand: true,
                            set_orientation: gtk::Orientation::Vertical,
                            #[local_ref]
                            list_box -> gtk::ListBox {
                                set_hexpand: true,
                                add_css_class: "boxed-list",
                                set_selection_mode: gtk::SelectionMode::None,
                            },
                            gtk::Box {
                                set_hexpand: true,
                                set_vexpand: true,
                            },
                        },
                    },
                },
            },
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let group_list_entries = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |msg| match msg {
                group_lists_display::EntryOutput::EditGroupList(id) => {
                    ColloscopeInput::EditGroupList(id)
                }
            });

        let group_list_dialog = group_list_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                group_list_dialog::DialogOutput::Accepted(collo_group_list) => {
                    ColloscopeInput::GroupListAccepted(collo_group_list)
                }
                group_list_dialog::DialogOutput::PresentParent => ColloscopeInput::PresentParent,
            });

        let colloscope_display = colloscope_display::Display::builder().launch(()).forward(
            sender.input_sender(),
            |msg| match msg {
                colloscope_display::DisplayOutput::InterrogationClicked(slot_id, week_id) => {
                    ColloscopeInput::EditInterrogation(slot_id, week_id)
                }
            },
        );

        let interrogation_dialog = interrogation_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                interrogation_dialog::DialogOutput::Accepted(interrogation) => {
                    ColloscopeInput::InterrogationAccepted(interrogation)
                }
                interrogation_dialog::DialogOutput::PresentParent => ColloscopeInput::PresentParent,
            });

        let blame_dialog = blame_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                blame_dialog::DialogOutput::PresentParent => ColloscopeInput::PresentParent,
            });

        let run_solver_dialog = SolverDialog::builder()
            .transient_for(&root)
            .launch(run_solver::DialogSettings {
                title: "Résolution du colloscope".to_string(),
                cancel_warning: "Toutes les modifications sur le colloscope seront perdues."
                    .to_string(),
            })
            .forward(sender.input_sender(), |msg| match msg {
                run_solver::DialogOutput::NewConfig(config) => ColloscopeInput::SolveResult(config),
                run_solver::DialogOutput::PresentParent => ColloscopeInput::PresentParent,
            });

        let config_dialog = ConfigDialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                config_dialog::DialogOutput::Accepted(config, strategy, params) => {
                    ColloscopeInput::ConductorConfigAccepted(config, strategy, params)
                }
                config_dialog::DialogOutput::Cancelled => ColloscopeInput::ConductorConfigCancelled,
                config_dialog::DialogOutput::PresentParent => ColloscopeInput::PresentParent,
            });

        let loading_dialog = loading_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                loading_dialog::DialogOutput::ModelReady(model) => {
                    ColloscopeInput::ModelBuilt(model)
                }
                loading_dialog::DialogOutput::Cancelled => ColloscopeInput::ModelBuildCancelled,
                loading_dialog::DialogOutput::PresentParent => ColloscopeInput::PresentParent,
            });

        let model = Colloscope {
            params: collomatique_state_colloscopes::colloscope_params::Parameters::default(),
            colloscope: collomatique_state_colloscopes::colloscopes::Colloscope::default(),
            group_list_entries,
            group_list_dialog,
            edited_group_list: None,
            colloscope_display,
            interrogation_dialog,
            edited_interrogation: None,
            blame_dialog,
            run_solver_dialog,
            config_dialog,
            loading_dialog,
            solve_config: collomatique_constraints_colloscopes::SolveConfig::default(),
            strategy: collomatique_strategies::ConductorStrategy::with_parallelism_defaults(),
            last_update: None,
            computation_artifact: None,
            inflight_cmd: InflightCommand {
                debouncing: false,
                computation: InflightComputation::None,
            },
        };

        let list_box = model.group_list_entries.widget();
        let colloscope_display_box = model.colloscope_display.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            ColloscopeInput::Update(params, colloscope) => {
                self.params = params;
                self.colloscope = colloscope;
                self.last_update = Some(std::time::Instant::now());

                // (Re)arm the debounce so the UI shows "En attente des données...".
                // The debounce timer, when it fires, drives the choke point; we do
                // not call it directly here.
                if !self.inflight_cmd.debouncing {
                    self.launch_debounce(sender.clone());
                }

                self.update_group_list_entries();
                self.update_colloscope_display();
                self.notify_children(&sender);
            }
            ColloscopeInput::EditGroupList(group_list_id) => {
                self.edited_group_list = Some(group_list_id);
                self.group_list_dialog
                    .sender()
                    .send(group_list_dialog::DialogInput::Show(
                        self.params.students.clone(),
                        self.params
                            .group_lists
                            .group_list_map
                            .get(&group_list_id)
                            .cloned()
                            .expect("Group list ID should be valid"),
                        self.colloscope
                            .group_list(group_list_id)
                            .cloned()
                            .unwrap_or_default(),
                    ))
                    .unwrap();
            }
            ColloscopeInput::GroupListAccepted(groups_for_students) => {
                let group_list_id = self
                    .edited_group_list
                    .take()
                    .expect("A group list id should have been stored for edition");
                sender
                    .output(ColloscopeOutput::UpdateOp(
                        ColloscopeUpdateOp::UpdateColloscopeGroupList(
                            group_list_id,
                            groups_for_students,
                        ),
                    ))
                    .unwrap();
            }
            ColloscopeInput::EditInterrogation(slot_id, week_id) => {
                // The edit is only meaningful on a possible interrogation cell.
                if !self.params.is_interrogation_possible(slot_id, week_id) {
                    return;
                }
                self.edited_interrogation = Some((slot_id, week_id));

                let (period_id, _pos) = self
                    .params
                    .weeks
                    .week_position(week_id)
                    .expect("week id should be valid");
                let (subject_id, _pos) = self
                    .params
                    .slots
                    .find_slot_subject_and_position(slot_id)
                    .expect("Slot ID should be valid");
                let group_list_id = self
                    .params
                    .group_lists
                    .subjects_associations
                    .get(&(period_id, subject_id))
                    .expect("A group list is needed to be able to edit a slot");
                let group_list = self
                    .params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                    .expect("Group list ID should be valid");

                // « Groupe 3 » or « Groupe 3 : B2 » — the number always shows
                // here, because it is what the colloscope cell stores.
                let group_titles: Vec<_> = (0..group_list.params().group_names.len() as u32)
                    .map(|num| {
                        let name = collomatique_ui_text::rendering::render_group_name(
                            &self.params.group_lists,
                            *group_list_id,
                            num,
                        )
                        .expect("the group comes from the document being displayed");
                        match name {
                            Some(name) => format!("Groupe {} : {}", num + 1, name),
                            None => format!("Groupe {}", num + 1),
                        }
                    })
                    .collect();

                let assigned_groups = self
                    .colloscope
                    .interrogation(slot_id, week_id)
                    .cloned()
                    .unwrap_or_default();

                self.interrogation_dialog
                    .sender()
                    .send(interrogation_dialog::DialogInput::Show(
                        group_titles,
                        assigned_groups,
                    ))
                    .unwrap();
            }
            ColloscopeInput::InterrogationAccepted(assigned_groups) => {
                let (slot_id, week_id) = self
                    .edited_interrogation
                    .take()
                    .expect("Interrogation information should have been stored for edition");
                sender
                    .output(ColloscopeOutput::UpdateOp(
                        ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                            slot_id,
                            week_id,
                            assigned_groups,
                        ),
                    ))
                    .unwrap();
            }
            ColloscopeInput::EraseColloscopeClicked => {
                sender
                    .output(ColloscopeOutput::UpdateOp(
                        ColloscopeUpdateOp::EraseColloscope,
                    ))
                    .unwrap();
            }
            ColloscopeInput::EraseGroupListsClicked => {
                sender
                    .output(ColloscopeOutput::UpdateOp(
                        ColloscopeUpdateOp::EraseGroupLists,
                    ))
                    .unwrap();
            }
            ColloscopeInput::SolveColloscopeClicked => {
                // The model is (re)built at solve time from the current parameters, so this no
                // longer depends on the debounce artifact being ready. The solve dialogs are
                // modal and freeze `params`, so the built model matches what is on screen.
                self.config_dialog
                    .sender()
                    .send(config_dialog::DialogInput::Show(
                        self.solve_config.clone(),
                        self.strategy.clone(),
                        self.params.clone(),
                    ))
                    .unwrap();
            }
            ColloscopeInput::ConductorConfigAccepted(config, strategy, params) => {
                // Configuration confirmed: persist the config and strategy so the next solve
                // reopens pre-primed, then build the (possibly refined) model for this solve in
                // the loading dialog, which streams the build log and hands back the model.
                self.solve_config = config.clone();
                self.strategy = strategy;
                self.loading_dialog
                    .sender()
                    .send(loading_dialog::DialogInput::Show(
                        config,
                        params,
                        self.colloscope.clone(),
                    ))
                    .unwrap();
            }
            ColloscopeInput::ModelBuilt(model) => {
                // The model has been built: launch the solver with the stored strategy and the
                // incremental epoch payload.
                let payload = build_incremental_payload(&model);
                self.run_solver_dialog
                    .sender()
                    .send(run_solver::DialogInput::Run(
                        self.strategy.clone(),
                        model,
                        payload,
                    ))
                    .unwrap();
            }
            ColloscopeInput::ConductorConfigCancelled | ColloscopeInput::ModelBuildCancelled => {
                // The solve was abandoned before it started; nothing to undo.
            }
            ColloscopeInput::ResetSolveConfig => {
                // A new document was loaded; drop the previous file's stored config and strategy
                // back to the defaults so the config dialog reopens on the parallel default.
                self.solve_config = collomatique_constraints_colloscopes::SolveConfig::default();
                self.strategy =
                    collomatique_strategies::ConductorStrategy::with_parallelism_defaults();
            }
            ColloscopeInput::SolveResult(config_data) => {
                // Translate the raw ILP config back into a colloscope, using the
                // current problem (still the dispatched one thanks to modal dialogs).
                if let Some(ilp_problem) = self.get_ilp_problem() {
                    // Drop the non-base variables straight from the config the solver returned,
                    // rather than rebuilding and re-checking a full Solution (~100ms on the UI
                    // thread) only to throw it away and keep the base values. The solved config
                    // is over the *configured* model's variables, so strip to base variables
                    // directly (the base export model's extra space no longer matches).
                    let base_config = config_data.filter_transmute(|var| match var {
                        collomatique_constraints_colloscopes::InternalVar::Base(b) => {
                            Some(b.clone())
                        }
                        _ => None,
                    });
                    if let Some(colloscope) =
                        collomatique_constraints_colloscopes::convert::build_colloscope(
                            &ilp_problem.env,
                            &base_config,
                        )
                    {
                        sender
                            .output(ColloscopeOutput::NewColloscope(colloscope))
                            .unwrap();
                    }
                }
            }
            ColloscopeInput::ShowBlamedConstraints => {
                self.blame_dialog
                    .sender()
                    .send(blame_dialog::DialogInput::Show)
                    .unwrap();
            }
            ColloscopeInput::PresentParent => {
                sender.output(ColloscopeOutput::PresentParent).unwrap();
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            ColloscopeCommandOutput::DebounceElapsed => {
                self.inflight_cmd.debouncing = false;
            }
            ColloscopeCommandOutput::IlpProblemComputed { env, result } => {
                self.inflight_cmd.computation = InflightComputation::None;
                // Keep the result only if it still matches the latest `Update`;
                // otherwise discard it (the choke point will rebuild as needed).
                if env == self.params {
                    self.computation_artifact = Some(match result {
                        Ok(problem) => ComputationArtifact::Built {
                            ilp_problem: IlpProblem { env, problem },
                            ilp_repr: None,
                        },
                        Err(message) => ComputationArtifact::BuildFailed { env, message },
                    });
                }
            }
            ColloscopeCommandOutput::IlpReprComputed(ilp_repr) => {
                self.inflight_cmd.computation = InflightComputation::None;
                // Attach the verification only if the problem is still the one it
                // was computed against and the colloscope still matches.
                if ilp_repr.colloscope == self.colloscope {
                    if let Some(ComputationArtifact::Built {
                        ilp_problem,
                        ilp_repr: slot,
                    }) = &mut self.computation_artifact
                    {
                        if ilp_problem.env == self.params {
                            *slot = Some(ilp_repr);
                        }
                    }
                }
            }
        }

        self.check_state_and_launch_computation(sender.clone());
        self.notify_children(&sender);
    }
}

impl Colloscope {
    /// (Re)arm the debounce timer. At most one timer exists at a time, so this
    /// must only be called when no timer is already running.
    fn launch_debounce(&mut self, sender: ComponentSender<Self>) {
        assert!(!self.inflight_cmd.debouncing);
        self.inflight_cmd.debouncing = true;

        sender.oneshot_command(async move {
            tokio::time::sleep(DEBOUNCE_DURATION).await;
            ColloscopeCommandOutput::DebounceElapsed
        });
    }

    /// Build the ILP model from the current parameters/colloscope. Requires that
    /// no computation is already in flight.
    fn launch_ilp_problem(&mut self, sender: ComponentSender<Self>) {
        assert!(self.inflight_cmd.computation == InflightComputation::None);
        self.inflight_cmd.computation = InflightComputation::IlpProblem;

        let params = self.params.clone();

        sender.spawn_oneshot_command(move || {
            let result: Result<collomatique_constraints_colloscopes::ColloscopeModel, String> =
                Ok(collomatique_constraints_colloscopes::build_model(&params));

            ColloscopeCommandOutput::IlpProblemComputed {
                env: params,
                result,
            }
        });
    }

    /// Verify the current colloscope against the built ILP problem, producing the
    /// warnings. Requires no computation in flight and a successfully built
    /// problem in the artifact.
    fn launch_ilp_repr(&mut self, sender: ComponentSender<Self>) {
        assert!(self.inflight_cmd.computation == InflightComputation::None);
        let ilp_problem = match &self.computation_artifact {
            Some(ComputationArtifact::Built { ilp_problem, .. }) => ilp_problem.clone(),
            _ => panic!("launch_ilp_repr requires a built ILP problem"),
        };
        self.inflight_cmd.computation = InflightComputation::IlpRepr;

        let colloscope = self.colloscope.clone();

        sender.spawn_oneshot_command(move || {
            let config_data = collomatique_constraints_colloscopes::convert::build_complete_config(
                &ilp_problem.env,
                &colloscope,
            );
            let solver =
                collomatique_ilp::solvers::collo_cbc::ColloCbcSolver::with_disable_logging(true);
            let sol = ilp_problem
                .problem
                .checker_solution_from_data(&config_data, &solver)
                .expect("There should be a complete ilp config for the colloscope");
            let mut warnings: Vec<_> = sol
                .minimal_blame()
                .iter()
                .map(|desc| (desc.severity_level(), desc.user_readable(&ilp_problem.env)))
                .collect();
            warnings.sort_by_key(|(s, _)| *s);
            ColloscopeCommandOutput::IlpReprComputed(IlpRepr {
                colloscope,
                warnings,
            })
        });
    }

    /// The single choke point deciding what to compute next. Called after every
    /// command message (debounce timer or a completed computation). It compares
    /// the artifact against the latest `Update` data and launches at most one
    /// computation (or re-arms the debounce).
    fn check_state_and_launch_computation(&mut self, sender: ComponentSender<Self>) {
        let Some(last_update) = self.last_update else {
            return; // No data received yet.
        };

        let quiet_elapsed = last_update.elapsed() >= DEBOUNCE_DURATION;
        let slot_free = self.inflight_cmd.computation == InflightComputation::None;

        // Not ready to act: either we are still within the debounce window, or a
        // (possibly now-stale) computation still occupies the single slot. In both
        // cases keep debouncing, so the latest data stays marked stale until we
        // can rebuild for it. The running command (or the timer) will call us
        // again once it completes.
        if !quiet_elapsed || !slot_free {
            if !self.inflight_cmd.debouncing {
                self.launch_debounce(sender);
            }
            return;
        }

        // The slot is free and enough time has passed, but a debounce timer is
        // still pending; wait for it to fire and re-resolve the state.
        if self.inflight_cmd.debouncing {
            return;
        }

        // Settled: reconcile the artifact with the current data and launch the
        // next step if needed.
        enum Next {
            BuildProblem,
            BuildRepr,
            Nothing,
        }

        let next = match &self.computation_artifact {
            None => Next::BuildProblem,
            Some(artifact) if *artifact.env() != self.params => Next::BuildProblem,
            Some(ComputationArtifact::BuildFailed { .. }) => Next::Nothing,
            Some(ComputationArtifact::Built { ilp_repr: None, .. }) => Next::BuildRepr,
            Some(ComputationArtifact::Built {
                ilp_repr: Some(ilp_repr),
                ..
            }) => {
                if ilp_repr.colloscope != self.colloscope {
                    Next::BuildRepr
                } else {
                    Next::Nothing
                }
            }
        };

        match next {
            Next::BuildProblem => {
                // The current artifact (if any) is stale for the new parameters.
                self.computation_artifact = None;
                self.launch_ilp_problem(sender);
            }
            Next::BuildRepr => {
                // Invalidate any stale verification before recomputing it.
                if let Some(ComputationArtifact::Built { ilp_repr, .. }) =
                    &mut self.computation_artifact
                {
                    *ilp_repr = None;
                }
                self.launch_ilp_repr(sender);
            }
            Next::Nothing => {}
        }
    }

    /// Push the current computation phase to the child components (blame dialog
    /// and export panel).
    fn notify_children(&self, sender: &ComponentSender<Self>) {
        let blame_state = if self.inflight_cmd.debouncing || self.last_update.is_none() {
            blame_dialog::ComputationState::Debouncing
        } else {
            match self.inflight_cmd.computation {
                InflightComputation::IlpProblem => {
                    blame_dialog::ComputationState::ComputingConstraints
                }
                InflightComputation::IlpRepr => blame_dialog::ComputationState::RecomputingWarnings,
                InflightComputation::None => match &self.computation_artifact {
                    Some(ComputationArtifact::BuildFailed { message, .. }) => {
                        blame_dialog::ComputationState::ResultAvailable(Err(message.clone()))
                    }
                    Some(ComputationArtifact::Built {
                        ilp_repr: Some(ilp_repr),
                        ..
                    }) => blame_dialog::ComputationState::ResultAvailable(Ok(ilp_repr
                        .warnings
                        .clone())),
                    // Transient state between deciding and launching a computation.
                    _ => blame_dialog::ComputationState::ComputingConstraints,
                },
            }
        };
        self.blame_dialog
            .sender()
            .send(blame_dialog::DialogInput::Update(blame_state))
            .unwrap();

        // Just re-send the current inner problem on every notify rather than
        // caching whether it changed. Cloning is not free, but availability only
        // flips a handful of times per computation cycle (and `None` during
        // debounce is cheap), so it is not worth a cache whose correctness would
        // hinge on the fragile invariant that the problem only ever changes while
        // unavailable.
        let inner = self.get_ilp_problem().map(|p| p.problem.problem().clone());
        sender
            .output(ColloscopeOutput::UpdateIlpProblem(inner))
            .unwrap();
    }

    fn update_group_list_entries(&mut self) {
        let mut group_lists_vec: Vec<_> = self
            .params
            .group_lists
            .group_list_map
            .iter()
            .filter(|(_id, group_list)| !group_list.is_prefilled())
            .map(|(id, group_list)| group_lists_display::EntryData {
                id,
                group_list: group_list.clone(),
                groups_for_students: self.colloscope.group_list(id).cloned().unwrap_or_default(),
                total_student_count: self.params.students.student_map.len(),
            })
            .collect();

        group_lists_vec.sort_by_key(|data| (data.group_list.params().name.clone(), data.id));

        crate::tools::factories::update_vec_deque(
            &mut self.group_list_entries,
            group_lists_vec.into_iter(),
            group_lists_display::EntryInput::UpdateData,
        );
    }

    fn update_colloscope_display(&self) {
        self.colloscope_display
            .sender()
            .send(colloscope_display::DisplayInput::Update(
                self.params.periods.clone(),
                self.params.weeks.clone(),
                self.params.subjects.clone(),
                self.params.slots.clone(),
                self.params.teachers.clone(),
                self.params.students.clone(),
                self.params.group_lists.clone(),
                self.params.week_patterns.clone(),
                self.colloscope.clone(),
            ))
            .unwrap();
    }
}
