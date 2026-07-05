use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::prelude::FactoryVecDeque;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};
use relm4::{adw, gtk};

use collomatique_ops::ColloscopeUpdateOp;

use crate::editor::run_solver;

/// The solver dialog instantiated for the colloscope ILP model.
type SolverDialog = run_solver::Dialog<
    collomatique_constraints_colloscopes::Var,
    collomatique_constraints_colloscopes::ExtraVarName,
    collomatique_constraints_colloscopes::ConstraintDesc,
>;

const DEBOUNCE_DURATION: std::time::Duration = std::time::Duration::from_millis(500);

mod blame_dialog;
mod colloscope_display;
mod config_dialog;
mod group_list_dialog;
mod group_lists_display;
mod interrogation_dialog;

#[derive(Debug)]
pub enum ColloscopeInput {
    Update(
        collomatique_state_colloscopes::colloscope_params::Parameters,
        collomatique_state_colloscopes::colloscopes::Colloscope,
    ),

    EditGroupList(collomatique_state_colloscopes::GroupListId),
    GroupListAccepted(collomatique_state_colloscopes::colloscopes::ColloscopeGroupList),

    EditInterrogation(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::PeriodId,
        usize,
    ),
    InterrogationAccepted(collomatique_state_colloscopes::colloscopes::ColloscopeInterrogation),

    SolveColloscopeClicked,
    ConductorConfigAccepted(collomatique_strategies::ConductorStrategy),
    ConductorConfigCancelled,
    SolveResult(
        collomatique_ilp::ConfigData<collomatique_constraints_colloscopes::ProblemInternalVar>,
    ),
    EraseColloscopeClicked,
    EraseGroupListsClicked,

    ShowBlamedConstraints,
}

#[derive(Debug)]
pub enum ColloscopeCommandOutput {
    DebouncedStart(std::time::Instant),
    IlpProblemComputed(Result<IlpProblem, String>),
    IlpReprComputed(IlpRepr),
}

#[derive(Debug)]
pub enum ColloscopeOutput {
    UpdateOp(ColloscopeUpdateOp),
    NewColloscope(collomatique_state_colloscopes::colloscopes::Colloscope),
    UpdateIlpProblem(Option<super::export_panel::IlpInnerProblem>),
}

/// The ILP problem together with the parameters needed to rebuild a colloscope
/// from a solver solution. Built on a debounce and handed to the solver dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IlpProblem {
    env: collomatique_state_colloscopes::colloscope_params::Parameters,
    problem: collomatique_constraints_colloscopes::ColloscopeModel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IlpRepr {
    ilp_problem: IlpProblem,
    colloscope: collomatique_state_colloscopes::colloscopes::Colloscope,
    warnings: Vec<(collomatique_constraints_colloscopes::SeverityLevel, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ComputationState {
    Debouncing(std::time::Instant),
    ComputingConstraints,
    RecomputingWarnings,
    ResultAvailable(Result<IlpRepr, String>),
}

impl ComputationState {
    fn as_ref(&self) -> Option<&Result<IlpRepr, String>> {
        match self {
            ComputationState::ResultAvailable(res) => Some(res),
            _ => None,
        }
    }
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
    config_dialog: Controller<config_dialog::Dialog>,

    // The problem currently being solved, kept so its solution config can be
    // turned back into a colloscope when the solver dialog returns.
    solving_problem: Option<IlpProblem>,

    edited_group_list: Option<collomatique_state_colloscopes::GroupListId>,
    edited_interrogation: Option<(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::PeriodId,
        usize,
    )>,

    computation_state: Option<ComputationState>,
}

impl Colloscope {
    fn get_ilp_repr(&self) -> Option<&Result<IlpRepr, String>> {
        self.computation_state.as_ref().and_then(|s| s.as_ref())
    }

    fn is_debouncing(&self) -> bool {
        match &self.computation_state {
            None => true,
            Some(s) => matches!(s, ComputationState::Debouncing(_)),
        }
    }

    fn is_constructing_constraints(&self) -> bool {
        matches!(
            &self.computation_state,
            Some(ComputationState::ComputingConstraints)
        )
    }

    fn is_rebuilding_warnings(&self) -> bool {
        matches!(
            &self.computation_state,
            Some(ComputationState::RecomputingWarnings)
        )
    }

    fn has_warnings(&self) -> bool {
        match self.get_ilp_repr() {
            Some(Ok(ilp_repr)) => !ilp_repr.warnings.is_empty(),
            _ => false,
        }
    }

    fn has_evaluation_error(&self) -> bool {
        matches!(
            &self.computation_state,
            Some(ComputationState::ResultAvailable(Err(_)))
        )
    }

    fn has_success(&self) -> bool {
        match self.get_ilp_repr() {
            Some(Ok(ilp_repr)) => ilp_repr.warnings.is_empty(),
            _ => false,
        }
    }

    fn generate_warning_text(&self) -> String {
        match self.get_ilp_repr() {
            Some(Ok(ilp_repr)) => format!("<small><i>{}</i></small>", ilp_repr.warnings.len()),
            _ => String::new(),
        }
    }

    fn worst_severity_level(&self) -> Option<collomatique_constraints_colloscopes::SeverityLevel> {
        match self.get_ilp_repr() {
            Some(Ok(ilp_repr)) => ilp_repr.warnings.first().map(|(s, _)| *s),
            _ => None,
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
                        set_sensitive: !model.colloscope.is_empty(),
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
                                    set_icon_name: Some("emblem-ok-symbolic"),
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
                set_visible: !model.colloscope.group_lists.is_empty(),
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
            });

        let colloscope_display = colloscope_display::Display::builder().launch(()).forward(
            sender.input_sender(),
            |msg| match msg {
                colloscope_display::DisplayOutput::InterrogationClicked(
                    slot_id,
                    period_id,
                    week_in_period,
                ) => ColloscopeInput::EditInterrogation(slot_id, period_id, week_in_period),
            },
        );

        let interrogation_dialog = interrogation_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                interrogation_dialog::DialogOutput::Accepted(interrogation) => {
                    ColloscopeInput::InterrogationAccepted(interrogation)
                }
            });

        let blame_dialog = blame_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .detach();

        let run_solver_dialog = SolverDialog::builder()
            .transient_for(&root)
            .launch("Résolution du colloscope".to_string())
            .forward(sender.input_sender(), |msg| match msg {
                run_solver::DialogOutput::NewConfig(config) => ColloscopeInput::SolveResult(config),
            });

        let config_dialog = config_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                config_dialog::DialogOutput::Accepted(strategy) => {
                    ColloscopeInput::ConductorConfigAccepted(strategy)
                }
                config_dialog::DialogOutput::Cancelled => ColloscopeInput::ConductorConfigCancelled,
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
            computation_state: None,
            blame_dialog,
            run_solver_dialog,
            config_dialog,
            solving_problem: None,
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

                match &self.computation_state {
                    None => {
                        self.debounce_compute(sender.clone());
                    }
                    Some(ComputationState::ResultAvailable(Ok(ilp_repr))) => {
                        if ilp_repr.ilp_problem.env != self.params {
                            self.debounce_compute(sender.clone());
                        } else if ilp_repr.colloscope != self.colloscope {
                            let ilp_problem = ilp_repr.ilp_problem.clone();
                            self.recompute_warnings(sender.clone(), ilp_problem);
                        }
                    }
                    Some(_) => {
                        self.debounce_compute(sender.clone());
                    }
                }

                self.update_group_list_entries();
                self.update_colloscope_display();
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
                            .group_lists
                            .get(&group_list_id)
                            .cloned()
                            .expect("Group list ID should be valid"),
                    ))
                    .unwrap();
            }
            ColloscopeInput::GroupListAccepted(collo_group_list) => {
                let group_list_id = self
                    .edited_group_list
                    .take()
                    .expect("A group list id should have been stored for edition");
                sender
                    .output(ColloscopeOutput::UpdateOp(
                        ColloscopeUpdateOp::UpdateColloscopeGroupList(
                            group_list_id,
                            collo_group_list,
                        ),
                    ))
                    .unwrap();
            }
            ColloscopeInput::EditInterrogation(slot_id, period_id, week_in_period) => {
                self.edited_interrogation = Some((slot_id, period_id, week_in_period));

                let (subject_id, _pos) = self
                    .params
                    .slots
                    .find_slot_subject_and_position(slot_id)
                    .expect("Slot ID should be valid");
                let period_associations = self
                    .params
                    .group_lists
                    .subjects_associations
                    .get(&period_id)
                    .expect("Period ID should be valid");
                let group_list_id = period_associations
                    .get(&subject_id)
                    .expect("A group list is needed to be able to edit a slot");
                let group_list = self
                    .params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                    .expect("Group list ID should be valid")
                    .clone();

                let collo_period = self
                    .colloscope
                    .period_map
                    .get(&period_id)
                    .expect("Period ID should be valid");
                let collo_slot = collo_period
                    .slot_map
                    .get(&slot_id)
                    .expect("Slot ID should be valid for this period");
                let interrogation_opt = collo_slot
                    .interrogations
                    .get(week_in_period)
                    .expect("Week number should be valid");
                let interrogation = interrogation_opt
                    .clone()
                    .expect("There should be an interrogation to edit!");

                self.interrogation_dialog
                    .sender()
                    .send(interrogation_dialog::DialogInput::Show(
                        group_list,
                        interrogation,
                    ))
                    .unwrap();
            }
            ColloscopeInput::InterrogationAccepted(interrogation) => {
                let (slot_id, period_id, week_in_period) = self
                    .edited_interrogation
                    .take()
                    .expect("Interrogation information should have been stored for edition");
                sender
                    .output(ColloscopeOutput::UpdateOp(
                        ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                            period_id,
                            slot_id,
                            week_in_period,
                            interrogation,
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
                // Only solvable once the ILP model is built (debounce finished). If no
                // problem is ready yet, ignore the click.
                if let Some(Ok(ilp_repr)) = self.get_ilp_repr() {
                    let ilp_problem = ilp_repr.ilp_problem.clone();
                    self.solving_problem = Some(ilp_problem);
                    self.config_dialog
                        .sender()
                        .send(config_dialog::DialogInput::Show)
                        .unwrap();
                }
            }
            ColloscopeInput::ConductorConfigAccepted(strategy) => {
                // The problem was stashed when the solve was requested; launch the solver
                // dialog now that the conductor configuration has been confirmed.
                if let Some(ilp_problem) = &self.solving_problem {
                    let model = ilp_problem.problem.clone();
                    self.run_solver_dialog
                        .sender()
                        .send(run_solver::DialogInput::Run(strategy, model))
                        .unwrap();
                }
            }
            ColloscopeInput::ConductorConfigCancelled => {
                // Drop the stashed problem: the solve was abandoned before it started.
                self.solving_problem = None;
            }
            ColloscopeInput::SolveResult(config_data) => {
                // Translate the raw ILP config back into a colloscope, using the
                // problem we dispatched. A real solution should always rebuild; a
                // failure here is dropped rather than surfaced.
                if let Some(ilp_problem) = self.solving_problem.take() {
                    if let Some(sol) = ilp_problem.problem.solution_from_complete_data(config_data)
                    {
                        let base_config = sol.get_data();
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
            }
            ColloscopeInput::ShowBlamedConstraints => {
                self.blame_dialog
                    .sender()
                    .send(blame_dialog::DialogInput::Show)
                    .unwrap();
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
            ColloscopeCommandOutput::IlpProblemComputed(result) => {
                match result {
                    Ok(ilp_problem) => {
                        if ilp_problem.env != self.params {
                            return; // Ignore old computation that are no longer relevant
                        }
                        self.recompute_warnings(sender, ilp_problem);
                    }
                    Err(msg) => {
                        self.update_ilp_repr(ComputationState::ResultAvailable(Err(msg)), &sender);
                    }
                }
            }
            ColloscopeCommandOutput::IlpReprComputed(ilp_repr) => {
                if ilp_repr.ilp_problem.env != self.params {
                    return; // Ignore old computation that are no longer relevant
                }
                self.update_ilp_repr(ComputationState::ResultAvailable(Ok(ilp_repr)), &sender);
            }
            ColloscopeCommandOutput::DebouncedStart(instant) => {
                if matches!(
                    &self.computation_state,
                    Some(ComputationState::Debouncing(t)) if *t == instant
                ) {
                    self.compute_ilp_repr(sender);
                }
            }
        }
    }
}

impl Colloscope {
    fn recompute_warnings(&mut self, sender: ComponentSender<Self>, ilp_problem: IlpProblem) {
        self.update_ilp_repr(ComputationState::RecomputingWarnings, &sender);

        let inner_problem = ilp_problem.problem.problem().clone();
        sender
            .output(ColloscopeOutput::UpdateIlpProblem(Some(inner_problem)))
            .unwrap();

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
                ilp_problem,
                colloscope,
                warnings,
            })
        });
    }

    fn debounce_compute(&mut self, sender: ComponentSender<Self>) {
        let instant = std::time::Instant::now();
        self.update_ilp_repr(ComputationState::Debouncing(instant.clone()), &sender);

        sender.oneshot_command(async move {
            tokio::time::sleep(DEBOUNCE_DURATION).await;
            ColloscopeCommandOutput::DebouncedStart(instant)
        });
    }

    fn compute_ilp_repr(&mut self, sender: ComponentSender<Self>) {
        self.update_ilp_repr(ComputationState::ComputingConstraints, &sender);

        let params = self.params.clone();
        let colloscope = self.colloscope.clone();

        sender.oneshot_command(async move {
            let result: Result<IlpProblem, String> = async {
                let inner_data = collomatique_state_colloscopes::InnerData {
                    params,
                    colloscope,
                    ..Default::default()
                };
                let env = inner_data.params.clone();

                let pool = sqlx::SqlitePool::connect(":memory:")
                    .await
                    .map_err(|e| format!("{}", e))?;
                collomatique_sqlite_state::create_schema(&pool)
                    .await
                    .map_err(|e| format!("{}", e))?;
                collomatique_sqlite_state::inner_data_to_sqlite(&pool, &inner_data)
                    .await
                    .map_err(|e| format!("{}", e))?;

                let problem = collomatique_constraints_colloscopes::build_model(&pool).await;
                Ok(IlpProblem { env, problem })
            }
            .await;

            match result {
                Ok(ilp_problem) => ColloscopeCommandOutput::IlpProblemComputed(Ok(ilp_problem)),
                Err(msg) => ColloscopeCommandOutput::IlpProblemComputed(Err(msg)),
            }
        });
    }

    fn update_blame_dialog(&self) {
        let ilp_repr_opt = match &self.computation_state {
            Some(ComputationState::Debouncing(_)) => blame_dialog::ComputationState::Debouncing,
            Some(ComputationState::ComputingConstraints) => {
                blame_dialog::ComputationState::ComputingConstraints
            }
            Some(ComputationState::RecomputingWarnings) => {
                blame_dialog::ComputationState::RecomputingWarnings
            }
            Some(ComputationState::ResultAvailable(r)) => {
                blame_dialog::ComputationState::ResultAvailable(
                    r.as_ref()
                        .map(|x| x.warnings.clone())
                        .map_err(|e| e.clone()),
                )
            }
            None => blame_dialog::ComputationState::ComputingConstraints,
        };

        self.blame_dialog
            .sender()
            .send(blame_dialog::DialogInput::Update(ilp_repr_opt))
            .unwrap();
    }

    fn update_ilp_repr(&mut self, new_state: ComputationState, sender: &ComponentSender<Self>) {
        self.computation_state = Some(new_state);
        match &self.computation_state {
            Some(ComputationState::Debouncing(_) | ComputationState::ComputingConstraints) => {
                sender
                    .output(ColloscopeOutput::UpdateIlpProblem(None))
                    .unwrap();
            }
            _ => {}
        }
        self.update_blame_dialog();
    }

    fn update_group_list_entries(&mut self) {
        let mut group_lists_vec: Vec<_> = self
            .params
            .group_lists
            .group_list_map
            .iter()
            .filter(|(_id, group_list)| !group_list.is_prefilled())
            .map(|(id, group_list)| group_lists_display::EntryData {
                id: *id,
                group_list: group_list.clone(),
                collo_group_list: self
                    .colloscope
                    .group_lists
                    .get(id)
                    .expect("Non-prefilled group list should have colloscope entry")
                    .clone(),
                total_student_count: self.params.students.student_map.len(),
            })
            .collect();

        group_lists_vec.sort_by_key(|data| (data.group_list.params.name.clone(), data.id));

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
                self.params.subjects.clone(),
                self.params.slots.clone(),
                self.params.teachers.clone(),
                self.params.students.clone(),
                self.params.group_lists.clone(),
                self.colloscope.clone(),
            ))
            .unwrap();
    }
}
