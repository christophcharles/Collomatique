use collo_ml::eval::Origin;
use collo_ml::problem::ConstraintDesc;
use collomatique_binding_colloscopes::views::ObjectId;
use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::prelude::FactoryVecDeque;
use relm4::{adw, gtk};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};

use collomatique_ops::ColloscopeUpdateOp;

mod colloscope_display;
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
}

#[derive(Debug)]
pub enum ColloscopeCommandOutput {
    IlpProblemComputed(IlpProblem),
    IlpReprComputed(IlpRepr),
}

#[derive(Debug)]
pub enum ColloscopeOutput {
    UpdateOp(ColloscopeUpdateOp),
    SolveColloscopeClicked,
}

#[derive(Debug, Clone)]
pub struct IlpProblem {
    params: collomatique_state_colloscopes::colloscope_params::Parameters,
    problem: collo_ml::problem::Problem<
        collomatique_binding_colloscopes::views::ObjectId,
        collomatique_binding_colloscopes::vars::Var,
    >,
}

#[derive(Debug, Clone)]
pub struct IlpRepr {
    ilp_problem: IlpProblem,
    colloscope: collomatique_state_colloscopes::colloscopes::Colloscope,
    warnings: Vec<Origin<ObjectId>>,
}

pub struct Colloscope {
    params: collomatique_state_colloscopes::colloscope_params::Parameters,
    colloscope: collomatique_state_colloscopes::colloscopes::Colloscope,

    group_list_entries: FactoryVecDeque<group_lists_display::Entry>,
    group_list_dialog: Controller<group_list_dialog::Dialog>,
    colloscope_display: Controller<colloscope_display::Display>,
    interrogation_dialog: Controller<interrogation_dialog::Dialog>,

    edited_group_list: Option<collomatique_state_colloscopes::GroupListId>,
    edited_interrogation: Option<(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::PeriodId,
        usize,
    )>,

    ilp_repr: Option<IlpRepr>,
}

impl Colloscope {
    fn has_warnings(&self) -> bool {
        match &self.ilp_repr {
            None => false,
            Some(ilp_repr) => !ilp_repr.warnings.is_empty(),
        }
    }

    fn generate_warning_text(&self) -> String {
        match &self.ilp_repr {
            None => String::new(),
            Some(ilp_repr) => format!("<small><i>{}</i></small>", ilp_repr.warnings.len()),
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
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_margin_start: 10,
                        set_spacing: 5,
                        #[watch]
                        set_visible: model.ilp_repr.is_none(),
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
                        set_margin_start: 10,
                        #[watch]
                        set_visible: model.ilp_repr.is_some() && !model.has_warnings(),
                        gtk::Image {
                            set_icon_name: Some("emblem-success"),
                        },
                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_margin_start: 10,
                        set_spacing: 5,
                        #[watch]
                        set_visible: model.has_warnings(),
                        gtk::Image {
                            set_icon_name: Some("emblem-warning"),
                        },
                        gtk::Label {
                            #[watch]
                            set_label: &model.generate_warning_text(),
                            set_use_markup: true,
                            add_css_class: "warning",
                        },
                    },
                    gtk::Box {
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Horizontal,
                    },
                    gtk::Button {
                        add_css_class: "frame",
                        add_css_class: "accent",
                        set_margin_all: 5,
                        adw::ButtonContent {
                            set_icon_name: "run-build-configure",
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
                gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 10,
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_margin_top: 10,
                        set_label: "Listes de groupes",
                        set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
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
                                #[watch]
                                set_visible: !model.colloscope.group_lists.is_empty(),
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "<i>Aucune liste à afficher</i>",
                                set_use_markup: true,
                                #[watch]
                                set_visible: model.colloscope.group_lists.is_empty(),
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
        _params: Self::Init,
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

        let model = Colloscope {
            params: collomatique_state_colloscopes::colloscope_params::Parameters::default(),
            colloscope: collomatique_state_colloscopes::colloscopes::Colloscope::default(),
            group_list_entries,
            group_list_dialog,
            edited_group_list: None,
            colloscope_display,
            interrogation_dialog,
            edited_interrogation: None,
            ilp_repr: None,
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

                match &self.ilp_repr {
                    Some(ilp_repr) => {
                        if ilp_repr.ilp_problem.params != self.params {
                            self.ilp_repr = None;
                            self.compute_ilp_repr(sender.clone());
                        } else if ilp_repr.colloscope != self.colloscope {
                            let ilp_problem = ilp_repr.ilp_problem.clone();
                            self.ilp_repr = None;
                            self.recompute_warnings(sender.clone(), ilp_problem);
                        } else {
                            // Everything is up to date
                        }
                    }
                    None => {
                        self.compute_ilp_repr(sender.clone());
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
            ColloscopeInput::SolveColloscopeClicked => {
                sender
                    .output(ColloscopeOutput::SolveColloscopeClicked)
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
            ColloscopeCommandOutput::IlpProblemComputed(ilp_problem) => {
                if ilp_problem.params != self.params {
                    return; // Ignore old computation that are no longer relevant
                }
                self.recompute_warnings(sender, ilp_problem);
            }
            ColloscopeCommandOutput::IlpReprComputed(ilp_repr) => {
                if ilp_repr.ilp_problem.params != self.params {
                    return; // Ignore old computation that are no longer relevant
                }
                self.ilp_repr = Some(ilp_repr);
            }
        }
    }
}

impl Colloscope {
    fn recompute_warnings(&self, sender: ComponentSender<Self>, ilp_problem: IlpProblem) {
        let env = collomatique_binding_colloscopes::views::Env::from(self.params.clone());
        let colloscope = self.colloscope.clone();

        sender.spawn_oneshot_command(move || {
            let config_data =
                collomatique_binding_colloscopes::convert::build_complete_config(&env, &colloscope);
            let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::with_disable_logging(true);
            let sol = ilp_problem
                .problem
                .solution_from_data(&config_data, &solver)
                .expect("There should be a complete ilp config for the colloscope");
            let to_blame_list = sol.blame();
            ColloscopeCommandOutput::IlpReprComputed(IlpRepr {
                ilp_problem,
                colloscope,
                warnings: to_blame_list
                    .into_iter()
                    .map(|(_constraint, desc)| {
                        let ConstraintDesc::InScript {
                            script_ref: _,
                            origin,
                        } = desc
                        else {
                            panic!(
                                "Reification constraints should all be satisfied! {:?}",
                                desc
                            )
                        };

                        origin
                    })
                    .collect(),
            })
        });
    }

    fn compute_ilp_repr(&self, sender: ComponentSender<Self>) {
        let params = self.params.clone();
        sender.spawn_oneshot_command(move || {
            use collomatique_binding_colloscopes::scripts::build_default_problem;
            let env = collomatique_binding_colloscopes::views::Env::from(params.clone());
            let problem = build_default_problem(&env);
            ColloscopeCommandOutput::IlpProblemComputed(IlpProblem { params, problem })
        });
    }

    fn update_group_list_entries(&mut self) {
        let mut group_lists_vec: Vec<_> = self
            .params
            .group_lists
            .group_list_map
            .iter()
            .map(|(id, group_list)| group_lists_display::EntryData {
                id: id.clone(),
                group_list: group_list.clone(),
                collo_group_list: self
                    .colloscope
                    .group_lists
                    .get(id)
                    .expect("Group list ID should be valid")
                    .clone(),
                total_student_count: self.params.students.student_map.len(),
            })
            .collect();

        group_lists_vec.sort_by_key(|data| (data.group_list.params.name.clone(), data.id.clone()));

        crate::tools::factories::update_vec_deque(
            &mut self.group_list_entries,
            group_lists_vec.into_iter(),
            |data| group_lists_display::EntryInput::UpdateData(data),
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
