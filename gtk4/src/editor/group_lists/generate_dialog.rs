mod advanced_dialog;
mod kept_list_row;
mod period_group;

use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, ToggleButtonExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    SimpleComponent,
};
use relm4::{adw, gtk};

use collomatique_constraints_groups::{
    GenerationRequest, GroupListSpec, GroupListSpecError, ObjectiveWeights,
};
use collomatique_state_colloscopes::NonEmptyRangeInclusive;
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_strategies::ConductorStrategy;
use std::num::NonZeroU32;

use crate::editor::run_solver::conductor_config;

pub struct Dialog {
    hidden: bool,
    /// The parameters the request is assembled against, set on `Show` and echoed back on
    /// `Accepted` so the rest of the chain builds its model from exactly these.
    params: Parameters,
    /// The solver strategy this window is assembling, seeded from the page's last choice.
    strategy: ConductorStrategy,
    /// The advanced solver-configuration dialog, opened via "Personnalisée".
    conductor_config_dialog: Controller<conductor_config::Dialog>,
    /// The objective weights this window carries: seeded from the page on `Show`, edited
    /// through the "Paramètres avancés" dialog, echoed back on `Accepted`.
    weights: ObjectiveWeights,
    /// The canonical group-size override this window carries, travelling with
    /// `weights`: `None` asks the planner to elect the size itself.
    canonical_range: Option<NonEmptyRangeInclusive<NonZeroU32>>,
    /// The advanced model-parameter dialog (objective weights and canonical
    /// size), opened via the "Paramètres avancés" button.
    advanced_dialog: Controller<advanced_dialog::Dialog>,
    /// One titled [`adw::PreferencesGroup`] per period, shown in the left panel.
    periods_list: FactoryVecDeque<period_group::PeriodGroup>,
    /// One switch row per existing prefilled list, shown in the right panel.
    kept_lists_list: FactoryVecDeque<kept_list_row::KeptListRow>,
    /// Per-period switch state backing `periods_list`. Rebuilt from `params` on every `Show`;
    /// carries its own ids, so reading the request back needs no re-derivation.
    periods_data: Vec<period_group::Data>,
    /// Per-prefilled-list switch state backing `kept_lists_list`, same contract.
    kept_lists_data: Vec<kept_list_row::Data>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        ConductorStrategy,
        ObjectiveWeights,
        Option<NonEmptyRangeInclusive<NonZeroU32>>,
        Parameters,
    ),
    Cancel,
    Accept,
    OpenAdvanced,
    UpdateStrategy(ConductorStrategy),
    OpenAdvancedParams,
    UpdateAdvancedParams(ObjectiveWeights, Option<NonEmptyRangeInclusive<NonZeroU32>>),
    IgnoreOrRefresh,
    /// (period index, subject index within that period, new value)
    SetSubjectRebuild(usize, usize, bool),
    /// (prefilled-list index, new value)
    SetKeptList(usize, bool),
}

#[derive(Debug)]
pub enum DialogOutput {
    Cancelled,
    Accepted(
        GenerationRequest,
        ConductorStrategy,
        ObjectiveWeights,
        Parameters,
    ),
}

impl Dialog {
    fn is_opt_strategy(&self) -> bool {
        self.strategy == ConductorStrategy::with_parallelism_defaults()
    }

    fn is_search_strategy(&self) -> bool {
        self.strategy == ConductorStrategy::default()
    }

    fn is_other_strategy(&self) -> bool {
        !self.is_opt_strategy() && !self.is_search_strategy()
    }
}

impl Dialog {
    /// Whether the left pane has anything at all: a period with at least one subject that has
    /// interrogations and is not excluded there. Empty periods never produce a group, so this is
    /// exactly "there is something that could be rebuilt".
    fn has_rebuildable_pairs(&self) -> bool {
        !self.periods_data.is_empty()
    }

    fn has_prefilled_lists(&self) -> bool {
        !self.kept_lists_data.is_empty()
    }

    /// Whether anything is actually selected for rebuild — "Valider" needs it.
    fn has_any_rebuild(&self) -> bool {
        self.periods_data
            .iter()
            .any(|period| period.subjects.iter().any(|subject| subject.rebuild))
    }

    /// Whether a subject *selected for rebuild* asks for group sizes its students cannot
    /// satisfy. Such a request has no solution at all, so "Valider" stays disabled: everything
    /// downstream of this window may then assume every spec is buildable.
    fn has_spec_errors(&self) -> bool {
        self.periods_data.iter().any(|period| {
            period
                .subjects
                .iter()
                .any(|subject| subject.rebuild && subject.error.is_some())
        })
    }

    /// The subjects eligible on a period, in document order: they must have interrogation
    /// parameters (the roadmap's rule) and must not exclude the period. The group size range
    /// comes along, since the eligibility filter is what proves it is there.
    fn eligible_subjects(
        &self,
        period_id: collomatique_state_colloscopes::PeriodId,
    ) -> Vec<(
        collomatique_state_colloscopes::SubjectId,
        String,
        collomatique_state_colloscopes::NonEmptyRangeInclusive<std::num::NonZeroU32>,
    )> {
        self.params
            .subjects
            .ordered_subject_list
            .iter()
            .filter(|(_id, subject)| !subject.excluded_periods.contains(&period_id))
            .filter_map(|(id, subject)| {
                let params = subject.parameters.interrogation_parameters.as_ref()?;
                Some((
                    id,
                    subject.parameters.name.clone(),
                    params.students_per_group.clone(),
                ))
            })
            .collect()
    }
}

/// The message shown under an unbuildable subject. `NoStudents` cannot reach the display —
/// a pair with nobody registered is legitimately skipped, not an error — but the match stays
/// exhaustive so a new variant is not silently mapped to something wrong.
fn spec_error_message(error: GroupListSpecError) -> String {
    match error {
        GroupListSpecError::NoStudents => String::from("Aucun élève inscrit dans cette matière"),
        GroupListSpecError::UnsatisfiableSize { students, min, max } => {
            format!("Impossible de répartir {students} élèves en groupes de {min} à {max} élèves")
        }
    }
}

impl Dialog {
    /// Rebuild both panes' switch state from `self.params`. Called on every `Show`: the defaults
    /// are a function of the current document (rebuild what has no list yet, keep every prefilled
    /// list), so nothing is carried over between openings.
    fn set_data_from_params(&mut self) {
        let periods_data: Vec<period_group::Data> = self
            .params
            .periods
            .period_ids()
            .filter_map(|period_id| {
                let eligible = self.eligible_subjects(period_id);
                // A period with nothing to offer gets no group at all, rather than an empty one.
                if eligible.is_empty() {
                    return None;
                }

                let period = collomatique_ops::rendering::render_period(
                    &self.params.periods,
                    &self.params.weeks,
                    period_id,
                )
                .expect("the period comes from the document being displayed");

                let subjects = eligible
                    .into_iter()
                    .map(|(subject_id, name, students_per_group)| {
                        // The feasibility gate, run here and nowhere else: a pair whose sizes
                        // cannot split its students blocks "Valider", so `build_generation_plan`
                        // downstream never sees one. A pair with nobody registered is not an
                        // error — it is skipped by the planner.
                        let error = match self.params.assignments.students(period_id, subject_id) {
                            Some(students) if !students.is_empty() => {
                                GroupListSpec::new(students.clone(), students_per_group)
                                    .err()
                                    .map(spec_error_message)
                            }
                            _ => None,
                        };

                        let current = self
                            .params
                            .group_lists
                            .subjects_associations
                            .get(&(period_id, subject_id))
                            .copied();
                        let subtitle = match current {
                            None => String::from("Aucune liste associée"),
                            Some(list_id) => format!(
                                "Liste actuelle : {}",
                                self.params
                                    .group_lists
                                    .group_list_map
                                    .get(&list_id)
                                    .expect("an association points at an existing list")
                                    .params()
                                    .name,
                            ),
                        };
                        period_group::SubjectData {
                            subject_id,
                            title: name,
                            subtitle,
                            // The default: rebuild exactly the pairs that have no list yet.
                            rebuild: current.is_none(),
                            error,
                        }
                    })
                    .collect();

                Some(period_group::Data {
                    period_id,
                    title: format!("Période {}", period),
                    subjects,
                })
            })
            .collect();

        let mut lists: Vec<_> = self
            .params
            .group_lists
            .group_list_map
            .iter()
            .filter(|(_id, list)| list.is_prefilled())
            .map(|(id, list)| {
                (
                    id,
                    list.params().name.clone(),
                    list.params().group_names.len(),
                    list.filling().iter_students().count(),
                )
            })
            .collect();
        // Same order as the group-lists page itself uses, so the two views agree.
        lists.sort_by_key(|(id, name, _groups, _students)| (name.clone(), *id));

        let kept_lists_data: Vec<kept_list_row::Data> = lists
            .into_iter()
            .map(
                |(group_list_id, title, groups, students)| kept_list_row::Data {
                    group_list_id,
                    title,
                    subtitle: format!("{} groupes, {} élèves", groups, students),
                    // The default: every existing prefilled list is kept as a stability anchor.
                    keep: true,
                },
            )
            .collect();

        self.periods_data = periods_data;
        self.kept_lists_data = kept_lists_data;
    }

    /// Read the switch state back out. Every id needed is stored in the data structs, so this
    /// never re-derives an ordering.
    fn request_from_data(&self) -> GenerationRequest {
        GenerationRequest {
            rebuild: self
                .periods_data
                .iter()
                .flat_map(|period| {
                    period
                        .subjects
                        .iter()
                        .filter(|subject| subject.rebuild)
                        .map(move |subject| (period.period_id, subject.subject_id))
                })
                .collect(),
            kept_lists: self
                .kept_lists_data
                .iter()
                .filter(|list| list.keep)
                .map(|list| list.group_list_id)
                .collect(),
            canonical_range: self.canonical_range.clone(),
        }
    }

    fn refresh_periods_list(&mut self) {
        crate::tools::factories::update_vec_deque(
            &mut self.periods_list,
            self.periods_data.iter().cloned(),
            period_group::PeriodGroupInput::UpdateData,
        );
    }

    fn refresh_kept_lists_list(&mut self) {
        crate::tools::factories::update_vec_deque(
            &mut self.kept_lists_list,
            self.kept_lists_data.iter().cloned(),
            kept_list_row::KeptListRowInput::UpdateData,
        );
    }
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        #[root]
        adw::Window {
            set_modal: true,
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Génération automatique de listes de groupes"),
            set_default_size: (1024, 576),
            // Unfinished feature: GNOME's development-build striping on the header bar,
            // as `run_python_script` does.
            add_css_class: "devel",
            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        set_label: "Annuler",
                        connect_clicked => DialogInput::Cancel,
                    },
                    pack_end = &gtk::Button {
                        set_label: "Valider",
                        add_css_class: "suggested-action",
                        // Nothing selected means an empty plan and an empty model; a selected
                        // subject with impossible group sizes has no solution at all.
                        #[watch]
                        set_sensitive: model.has_any_rebuild() && !model.has_spec_errors(),
                        // Both disabled states say why, and the tooltip clears when enabled.
                        #[watch]
                        set_tooltip_text: if !model.has_any_rebuild() {
                            Some("Aucune matière sélectionnée à recalculer")
                        } else if model.has_spec_errors() {
                            Some("Certaines matières sélectionnées demandent des tailles de groupes impossibles")
                        } else {
                            None
                        },
                        connect_clicked => DialogInput::Accept,
                    },
                },
                add_top_bar = &adw::Banner {
                    set_title: "Fonctionnalité en cours de développement : \
                                les listes produites peuvent être incorrectes ou incomplètes.",
                    set_revealed: true,
                },
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_margin_all: 0,
                    set_spacing: 0,
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Frame {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_margin_all: 5,
                        gtk::Paned {
                            set_hexpand: true,
                            set_vexpand: true,
                            set_margin_all: 0,
                            set_orientation: gtk::Orientation::Horizontal,
                            set_position: 510,
                            #[wrap(Some)]
                            set_start_child = &gtk::Box {
                                set_hexpand: true,
                                set_vexpand: true,
                                set_margin_all: 0,
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 5,
                                gtk::Label {
                                    set_halign: gtk::Align::Center,
                                    set_margin_all: 10,
                                    set_label: "<b><big>Matières à recalculer</big></b>",
                                    set_use_markup: true,
                                    #[watch]
                                    set_visible: model.has_rebuildable_pairs(),
                                },
                                gtk::ScrolledWindow {
                                    set_hexpand: true,
                                    set_vexpand: true,
                                    set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                                    #[watch]
                                    set_visible: model.has_rebuildable_pairs(),
                                    #[local_ref]
                                    periods_box -> gtk::Box {
                                        set_hexpand: true,
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_margin_all: 5,
                                        set_spacing: 10,
                                    },
                                },
                                gtk::Label {
                                    set_valign: gtk::Align::Center,
                                    set_vexpand: true,
                                    set_hexpand: true,
                                    set_justify: gtk::Justification::Center,
                                    set_label: "<b><big>Aucune matière avec interrogations</big></b>",
                                    set_use_markup: true,
                                    #[watch]
                                    set_visible: !model.has_rebuildable_pairs(),
                                },
                            },
                            #[wrap(Some)]
                            set_end_child = &gtk::Box {
                                set_hexpand: true,
                                set_vexpand: true,
                                set_margin_all: 0,
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 5,
                                gtk::Label {
                                    set_halign: gtk::Align::Center,
                                    set_margin_all: 10,
                                    set_label: "<b><big>Listes existantes à conserver</big></b>",
                                    set_use_markup: true,
                                    #[watch]
                                    set_visible: model.has_prefilled_lists(),
                                },
                                gtk::ScrolledWindow {
                                    set_hexpand: true,
                                    set_vexpand: true,
                                    set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                                    #[watch]
                                    set_visible: model.has_prefilled_lists(),
                                    #[local_ref]
                                    kept_lists_box -> adw::PreferencesGroup {
                                        set_hexpand: true,
                                        set_margin_all: 5,
                                    },
                                },
                                gtk::Label {
                                    set_valign: gtk::Align::Center,
                                    set_vexpand: true,
                                    set_hexpand: true,
                                    set_justify: gtk::Justification::Center,
                                    set_label: "<b><big>Aucune liste préremplie</big></b>",
                                    set_use_markup: true,
                                    #[watch]
                                    set_visible: !model.has_prefilled_lists(),
                                },
                            },
                        },
                    },
                    gtk::Box {
                        set_margin_all: 0,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_hexpand: true,
                        gtk::Frame {
                            set_hexpand: true,
                            set_margin_all: 5,
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 5,
                                gtk::Label {
                                    set_margin_start: 10,
                                    set_margin_all: 5,
                                    set_label: "<b>Configuration du résolveur :</b>",
                                    set_use_markup: true,
                                },
                                gtk::Box {
                                    set_spacing: 0,
                                    add_css_class: "linked",
                                    #[name(opt_toggle_btn)]
                                    gtk::ToggleButton {
                                        set_margin_top: 5,
                                        set_margin_bottom: 5,
                                        set_label: "Optimisation complète",
                                        #[track(opt_toggle_btn.is_active() != model.is_opt_strategy())]
                                        set_active: model.is_opt_strategy(),
                                        connect_toggled[sender] => move |widget| {
                                            let new_state = widget.is_active();
                                            sender.input(if new_state {
                                                DialogInput::UpdateStrategy(ConductorStrategy::with_parallelism_defaults())
                                            } else {
                                                DialogInput::IgnoreOrRefresh
                                            });
                                        }
                                    },
                                    #[name(search_toggle_btn)]
                                    gtk::ToggleButton {
                                        set_margin_top: 5,
                                        set_margin_bottom: 5,
                                        set_label: "Recherche simple",
                                        #[track(search_toggle_btn.is_active() != model.is_search_strategy())]
                                        set_active: model.is_search_strategy(),
                                        connect_toggled[sender] => move |widget| {
                                            let new_state = widget.is_active();
                                            sender.input(if new_state {
                                                DialogInput::UpdateStrategy(ConductorStrategy::default())
                                            } else {
                                                DialogInput::IgnoreOrRefresh
                                            });
                                        }
                                    },
                                },
                                gtk::Label {
                                    set_margin_all: 5,
                                    set_label: "<i><small>Personnalisée</small></i>",
                                    set_use_markup: true,
                                    #[watch]
                                    set_visible: model.is_other_strategy(),
                                },
                                gtk::Box {
                                    set_hexpand: true,
                                },
                                gtk::Button {
                                    add_css_class: "frame",
                                    set_margin_all: 5,
                                    set_label: "Personnalisée",
                                    connect_clicked => DialogInput::OpenAdvanced,
                                },
                            },
                        },
                        gtk::Button {
                            add_css_class: "frame",
                            add_css_class: "warning",
                            set_margin_all: 5,
                            adw::ButtonContent {
                                set_icon_name: "configure-symbolic",
                                set_label: "Paramètres avancés",
                            },
                            connect_clicked => DialogInput::OpenAdvancedParams,
                        },
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
        let conductor_config_dialog = conductor_config::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                conductor_config::DialogOutput::Accepted(strategy) => {
                    DialogInput::UpdateStrategy(strategy)
                }
                conductor_config::DialogOutput::Cancelled => DialogInput::IgnoreOrRefresh,
            });

        let advanced_dialog = advanced_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                advanced_dialog::DialogOutput::Accepted(weights, canonical_range) => {
                    DialogInput::UpdateAdvancedParams(weights, canonical_range)
                }
                advanced_dialog::DialogOutput::Cancelled => DialogInput::IgnoreOrRefresh,
            });

        let periods_list = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |msg| match msg {
                period_group::PeriodGroupOutput::SubjectToggled(period, subject, value) => {
                    DialogInput::SetSubjectRebuild(period, subject, value)
                }
            });
        let kept_lists_list = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |msg| match msg {
                kept_list_row::KeptListRowOutput::Toggled(index, value) => {
                    DialogInput::SetKeptList(index, value)
                }
            });

        let model = Dialog {
            hidden: true,
            params: Parameters::default(),
            strategy: ConductorStrategy::with_parallelism_defaults(),
            conductor_config_dialog,
            weights: ObjectiveWeights::default(),
            canonical_range: None,
            advanced_dialog,
            periods_list,
            kept_lists_list,
            periods_data: Vec::new(),
            kept_lists_data: Vec::new(),
        };

        let periods_box = model.periods_list.widget();
        let kept_lists_box = model.kept_lists_list.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Show(strategy, weights, canonical_range, params) => {
                self.hidden = false;
                self.params = params;
                self.strategy = strategy;
                self.weights = weights;
                self.canonical_range = canonical_range;
                self.set_data_from_params();

                self.refresh_periods_list();
                self.refresh_kept_lists_list();
            }
            DialogInput::OpenAdvanced => {
                self.conductor_config_dialog
                    .sender()
                    .send(conductor_config::DialogInput::Show(self.strategy.clone()))
                    .unwrap();
            }
            DialogInput::UpdateStrategy(strategy) => {
                self.strategy = strategy;
            }
            DialogInput::OpenAdvancedParams => {
                self.advanced_dialog
                    .sender()
                    .send(advanced_dialog::DialogInput::Show(
                        self.weights,
                        self.canonical_range.clone(),
                    ))
                    .unwrap();
            }
            DialogInput::UpdateAdvancedParams(weights, canonical_range) => {
                self.weights = weights;
                self.canonical_range = canonical_range;
            }
            DialogInput::IgnoreOrRefresh => {}
            DialogInput::SetSubjectRebuild(period, subject, value) => {
                if let Some(data) = self
                    .periods_data
                    .get_mut(period)
                    .and_then(|p| p.subjects.get_mut(subject))
                {
                    data.rebuild = value;
                }
                self.refresh_periods_list();
            }
            DialogInput::SetKeptList(index, value) => {
                if let Some(data) = self.kept_lists_data.get_mut(index) {
                    data.keep = value;
                }
                self.refresh_kept_lists_list();
            }
            DialogInput::Cancel => {
                self.hidden = true;
                sender.output(DialogOutput::Cancelled).unwrap();
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(
                        self.request_from_data(),
                        self.strategy.clone(),
                        self.weights,
                        self.params.clone(),
                    ))
                    .unwrap();
            }
        }
    }
}
