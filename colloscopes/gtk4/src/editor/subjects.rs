use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};
use relm4::{adw, gtk};

use collomatique_ops::SubjectsUpdateOp;

mod subject_params;
mod subjects_display;

#[derive(Debug)]
pub enum SubjectsInput {
    Update(
        collomatique_state_colloscopes::periods::Periods,
        collomatique_state_colloscopes::weeks::Weeks,
        collomatique_state_colloscopes::week_patterns::WeekPatterns,
        collomatique_state_colloscopes::subjects::Subjects,
    ),
    AddSubjectClicked,

    EditSubjectClicked(collomatique_state_colloscopes::SubjectId),
    DeleteSubjectClicked(collomatique_state_colloscopes::SubjectId),
    MoveUpSubjectClicked(collomatique_state_colloscopes::SubjectId),
    MoveDownSubjectClicked(collomatique_state_colloscopes::SubjectId),
    PeriodStatusUpdated(collomatique_state_colloscopes::SubjectId, usize, bool),
    WeekPatternUpdated(
        collomatique_state_colloscopes::SubjectId,
        Option<collomatique_state_colloscopes::WeekPatternId>,
    ),

    SubjectParamsSelected(collomatique_state_colloscopes::SubjectParameters),
    /// A dialog of this panel just closed. The panel hosts no window of its
    /// own, so it passes the request up to the editor.
    PresentParent,
}

#[derive(Debug)]
pub enum SubjectsOutput {
    UpdateOp(SubjectsUpdateOp),
    /// A dialog of this panel just closed: the window underneath should be
    /// brought back to the front, because Windows will not do it on its own.
    PresentParent,
}

#[derive(Debug)]
enum SubjectParamsSelectionReason {
    New,
    Edit(collomatique_state_colloscopes::SubjectId),
}

pub struct Subjects {
    periods: collomatique_state_colloscopes::periods::Periods,
    weeks: collomatique_state_colloscopes::weeks::Weeks,
    /// The week patterns as the rows offer them: by name, then by id to break
    /// ties.
    ordered_week_patterns: Vec<(collomatique_state_colloscopes::WeekPatternId, String)>,
    subjects: collomatique_state_colloscopes::subjects::Subjects,
    subjects_list: FactoryVecDeque<subjects_display::Entry>,

    subject_params_selection_reason: SubjectParamsSelectionReason,

    subject_params_dialog: Controller<subject_params::Dialog>,
}

#[relm4::component(pub)]
impl Component for Subjects {
    type Input = SubjectsInput;
    type Output = SubjectsOutput;
    type Init = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hexpand: true,
            set_margin_all: 5,
            set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Automatic),
            gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 5,
                #[local_ref]
                subjects_box -> gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_top: 20,
                    set_spacing: 30,
                    #[watch]
                    set_visible: !model.subjects.ordered_subject_list.is_empty(),
                },
                gtk::Button {
                    set_margin_top: 10,
                    connect_clicked => SubjectsInput::AddSubjectClicked,
                    adw::ButtonContent {
                        set_icon_name: "list-add-symbolic",
                        set_label: "Ajouter une matière",
                    },
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let subject_params_dialog = subject_params::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                subject_params::DialogOutput::Accepted(params) => {
                    SubjectsInput::SubjectParamsSelected(params)
                }
                subject_params::DialogOutput::PresentParent => SubjectsInput::PresentParent,
            });

        let subjects_list = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |msg| match msg {
                subjects_display::EntryOutput::EditClicked(id) => {
                    SubjectsInput::EditSubjectClicked(id)
                }
                subjects_display::EntryOutput::DeleteClicked(id) => {
                    SubjectsInput::DeleteSubjectClicked(id)
                }
                subjects_display::EntryOutput::MoveUpClicked(id) => {
                    SubjectsInput::MoveUpSubjectClicked(id)
                }
                subjects_display::EntryOutput::MoveDownClicked(id) => {
                    SubjectsInput::MoveDownSubjectClicked(id)
                }
                subjects_display::EntryOutput::PeriodStatusUpdated(id, period_num, status) => {
                    SubjectsInput::PeriodStatusUpdated(id, period_num, status)
                }
                subjects_display::EntryOutput::WeekPatternUpdated(id, week_pattern_id) => {
                    SubjectsInput::WeekPatternUpdated(id, week_pattern_id)
                }
            });

        let model = Subjects {
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            weeks: collomatique_state_colloscopes::weeks::Weeks::default(),
            ordered_week_patterns: Vec::new(),
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            subjects_list,
            subject_params_selection_reason: SubjectParamsSelectionReason::New,
            subject_params_dialog,
        };
        let subjects_box = model.subjects_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            SubjectsInput::Update(new_periods, new_weeks, new_week_patterns, new_subjects) => {
                self.periods = new_periods;
                self.weeks = new_weeks;
                self.ordered_week_patterns = {
                    let mut week_patterns: Vec<_> = new_week_patterns
                        .week_pattern_map
                        .iter()
                        .map(|(id, week_pattern)| (id, week_pattern.name.clone()))
                        .collect();
                    week_patterns.sort_by_key(|(id, name)| (name.clone(), *id));
                    week_patterns
                };
                self.subjects = new_subjects;

                crate::tools::factories::update_vec_deque(
                    &mut self.subjects_list,
                    self.subjects
                        .ordered_subject_list
                        .iter()
                        .map(|(id, desc)| subjects_display::EntryData {
                            subject_params: desc.parameters.clone(),
                            ordered_week_patterns: self.ordered_week_patterns.clone(),
                            week_pattern: desc.week_pattern,
                            periods: self
                                .periods
                                .period_ids()
                                .map(|id| subjects_display::PeriodData {
                                    title: collomatique_ui_text::rendering::render_period(
                                        &self.periods,
                                        &self.weeks,
                                        id,
                                    )
                                    .expect("the period comes from the document being displayed"),
                                    status: !desc.excluded_periods.contains(&id),
                                })
                                .collect(),
                            subject_id: id,
                            subject_count: self.subjects.ordered_subject_list.len(),
                        })
                        .collect::<Vec<_>>()
                        .into_iter(),
                    subjects_display::EntryInput::UpdateData,
                );
            }
            SubjectsInput::AddSubjectClicked => {
                self.subject_params_selection_reason = SubjectParamsSelectionReason::New;
                self.subject_params_dialog
                    .sender()
                    .send(subject_params::DialogInput::Show(
                        self.periods.first_week.clone(),
                        collomatique_state_colloscopes::SubjectParameters::default(),
                    ))
                    .unwrap();
            }
            SubjectsInput::EditSubjectClicked(id) => {
                self.subject_params_selection_reason = SubjectParamsSelectionReason::Edit(id);
                let current_subject = self.subjects.find_subject(id).expect("valid position");
                self.subject_params_dialog
                    .sender()
                    .send(subject_params::DialogInput::Show(
                        self.periods.first_week.clone(),
                        current_subject.parameters.clone(),
                    ))
                    .unwrap();
            }
            SubjectsInput::DeleteSubjectClicked(id) => {
                sender
                    .output(SubjectsOutput::UpdateOp(SubjectsUpdateOp::DeleteSubject(
                        id,
                    )))
                    .unwrap();
            }
            SubjectsInput::MoveUpSubjectClicked(id) => {
                sender
                    .output(SubjectsOutput::UpdateOp(SubjectsUpdateOp::MoveSubjectUp(
                        id,
                    )))
                    .unwrap();
            }
            SubjectsInput::MoveDownSubjectClicked(id) => {
                sender
                    .output(SubjectsOutput::UpdateOp(SubjectsUpdateOp::MoveSubjectDown(
                        id,
                    )))
                    .unwrap();
            }
            SubjectsInput::PeriodStatusUpdated(id, period_num, status) => {
                sender
                    .output(SubjectsOutput::UpdateOp(
                        SubjectsUpdateOp::UpdatePeriodStatus(
                            id,
                            self.periods
                                .period_id_at(period_num)
                                .expect("valid period index"),
                            status,
                        ),
                    ))
                    .unwrap();
            }
            SubjectsInput::WeekPatternUpdated(id, week_pattern_id) => {
                sender
                    .output(SubjectsOutput::UpdateOp(
                        SubjectsUpdateOp::SetSubjectWeekPattern(id, week_pattern_id),
                    ))
                    .unwrap();
            }
            SubjectsInput::SubjectParamsSelected(params) => {
                sender
                    .output(SubjectsOutput::UpdateOp(
                        match self.subject_params_selection_reason {
                            SubjectParamsSelectionReason::New => {
                                SubjectsUpdateOp::AddNewSubject(params)
                            }
                            SubjectParamsSelectionReason::Edit(id) => {
                                SubjectsUpdateOp::UpdateSubject(id, params)
                            }
                        },
                    ))
                    .unwrap();
            }
            SubjectsInput::PresentParent => {
                sender.output(SubjectsOutput::PresentParent).unwrap();
            }
        }
    }
}
