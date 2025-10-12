use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{adw, gtk};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};

use collomatique_ops::SubjectsUpdateOp;

mod subject_params;
mod subjects_display;

#[derive(Debug)]
pub enum SubjectsInput {
    Update(
        collomatique_state_colloscopes::periods::Periods<collomatique_state_colloscopes::PeriodId>,
        collomatique_state_colloscopes::subjects::Subjects<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::PeriodId,
        >,
    ),
    AddSubjectClicked,

    EditSubjectClicked(collomatique_state_colloscopes::SubjectId),
    DeleteSubjectClicked(collomatique_state_colloscopes::SubjectId),
    MoveUpSubjectClicked(collomatique_state_colloscopes::SubjectId),
    MoveDownSubjectClicked(collomatique_state_colloscopes::SubjectId),
    PeriodStatusUpdated(collomatique_state_colloscopes::SubjectId, usize, bool),

    SubjectParamsSelected(collomatique_state_colloscopes::SubjectParameters),
}

#[derive(Debug)]
enum SubjectParamsSelectionReason {
    New,
    Edit(collomatique_state_colloscopes::SubjectId),
}

pub struct Subjects {
    periods:
        collomatique_state_colloscopes::periods::Periods<collomatique_state_colloscopes::PeriodId>,
    subjects: collomatique_state_colloscopes::subjects::Subjects<
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    >,
    subjects_list: FactoryVecDeque<subjects_display::Entry>,

    subject_params_selection_reason: SubjectParamsSelectionReason,

    subject_params_dialog: Controller<subject_params::Dialog>,
}

#[relm4::component(pub)]
impl Component for Subjects {
    type Input = SubjectsInput;
    type Output = SubjectsUpdateOp;
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
                        set_icon_name: "edit-add",
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
            });

        let model = Subjects {
            periods: collomatique_state_colloscopes::periods::Periods::default(),
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
            SubjectsInput::Update(new_periods, new_subjects) => {
                self.periods = new_periods;
                self.subjects = new_subjects;

                crate::tools::factories::update_vec_deque(
                    &mut self.subjects_list,
                    self.subjects.ordered_subject_list.iter().map(|(id, desc)| {
                        subjects_display::EntryData {
                            subject_params: desc.parameters.clone(),
                            global_first_week: self.periods.first_week.clone(),
                            periods: self
                                .periods
                                .ordered_period_list
                                .iter()
                                .map(|(id, period_desc)| subjects_display::PeriodData {
                                    week_count: period_desc.len(),
                                    status: !desc.excluded_periods.contains(id),
                                })
                                .collect(),
                            subject_id: id.clone(),
                            subject_count: self.subjects.ordered_subject_list.len(),
                        }
                    }),
                    |data| subjects_display::EntryInput::UpdateData(data),
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
                sender.output(SubjectsUpdateOp::DeleteSubject(id)).unwrap();
            }
            SubjectsInput::MoveUpSubjectClicked(id) => {
                sender.output(SubjectsUpdateOp::MoveSubjectUp(id)).unwrap();
            }
            SubjectsInput::MoveDownSubjectClicked(id) => {
                sender
                    .output(SubjectsUpdateOp::MoveSubjectDown(id))
                    .unwrap();
            }
            SubjectsInput::PeriodStatusUpdated(id, period_num, status) => {
                sender
                    .output(SubjectsUpdateOp::UpdatePeriodStatus(
                        id,
                        self.periods.ordered_period_list[period_num].0,
                        status,
                    ))
                    .unwrap();
            }
            SubjectsInput::SubjectParamsSelected(params) => {
                sender
                    .output(match self.subject_params_selection_reason {
                        SubjectParamsSelectionReason::New => {
                            SubjectsUpdateOp::AddNewSubject(params)
                        }
                        SubjectParamsSelectionReason::Edit(id) => {
                            SubjectsUpdateOp::UpdateSubject(id, params)
                        }
                    })
                    .unwrap();
            }
        }
    }
}
