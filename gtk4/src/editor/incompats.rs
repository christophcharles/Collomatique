use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{gtk, ComponentController};
use relm4::{Component, ComponentParts, ComponentSender, Controller, RelmWidgetExt};

use std::num::NonZeroU32;

use collomatique_ops::IncompatibilitiesUpdateOp;

mod incompat_params;
mod incompats_display;

#[derive(Debug)]
pub enum IncompatsInput {
    Update(
        collomatique_state_colloscopes::subjects::Subjects<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::PeriodId,
        >,
        collomatique_state_colloscopes::week_patterns::WeekPatterns<
            collomatique_state_colloscopes::WeekPatternId,
        >,
        collomatique_state_colloscopes::incompats::Incompats<
            collomatique_state_colloscopes::IncompatId,
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    ),

    IncompatParamsSelected(
        collomatique_state_colloscopes::incompats::Incompatibility<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    ),
    DeleteIncompat(collomatique_state_colloscopes::IncompatId),
    EditIncompat(collomatique_state_colloscopes::IncompatId),
    AddIncompat(collomatique_state_colloscopes::SubjectId),
}

#[derive(Debug)]
enum IncompatParamsSelectionReason {
    New,
    Edit(collomatique_state_colloscopes::IncompatId),
}

pub struct Incompats {
    subjects: collomatique_state_colloscopes::subjects::Subjects<
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    >,
    week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns<
        collomatique_state_colloscopes::WeekPatternId,
    >,
    incompats: collomatique_state_colloscopes::incompats::Incompats<
        collomatique_state_colloscopes::IncompatId,
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::WeekPatternId,
    >,
    incompats_list: FactoryVecDeque<incompats_display::Entry>,

    incompat_params_dialog: Controller<incompat_params::Dialog>,
    incompat_params_selection_reason: IncompatParamsSelectionReason,
}

#[relm4::component(pub)]
impl Component for Incompats {
    type Input = IncompatsInput;
    type Output = IncompatibilitiesUpdateOp;
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
                gtk::Label {
                    set_margin_top: 10,
                    #[watch]
                    set_visible: model.subjects.ordered_subject_list.is_empty(),
                    set_halign: gtk::Align::Start,
                    set_label: "<big><b>Aucune matière à afficher</b></big>",
                    set_use_markup: true,
                },
                #[local_ref]
                subjects_box -> gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_top: 20,
                    set_spacing: 30,
                    #[watch]
                    set_visible: !model.subjects.ordered_subject_list.is_empty(),
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let incompats_list = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |msg| match msg {
                incompats_display::EntryOutput::DeleteIncompat(slot_id) => {
                    IncompatsInput::DeleteIncompat(slot_id)
                }
                incompats_display::EntryOutput::EditIncompat(slot_id) => {
                    IncompatsInput::EditIncompat(slot_id)
                }
                incompats_display::EntryOutput::AddIncompat(subject_id) => {
                    IncompatsInput::AddIncompat(subject_id)
                }
            });

        let incompat_params_dialog = incompat_params::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                incompat_params::DialogOutput::Accepted(params) => {
                    IncompatsInput::IncompatParamsSelected(params)
                }
            });

        let model = Incompats {
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns::default(),
            incompats: collomatique_state_colloscopes::incompats::Incompats::default(),
            incompats_list,
            incompat_params_dialog,
            incompat_params_selection_reason: IncompatParamsSelectionReason::New,
        };

        let subjects_box = model.incompats_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            IncompatsInput::Update(subjects, week_patterns, incompats) => {
                self.subjects = subjects;
                self.week_patterns = week_patterns;
                self.incompats = incompats;

                let new_data: Vec<_> = self
                    .subjects
                    .ordered_subject_list
                    .iter()
                    .map(|(id, desc)| {
                        let subject_incompats = self
                            .incompats
                            .incompat_map
                            .iter()
                            .filter_map(|(incompat_id, incompat)| {
                                if incompat.subject_id != *id {
                                    return None;
                                }
                                Some((incompat_id.clone(), incompat.clone()))
                            })
                            .collect();
                        incompats_display::EntryData {
                            subject_params: desc.parameters.clone(),
                            subject_id: id.clone(),
                            week_patterns: self.week_patterns.clone(),
                            subject_incompats,
                        }
                    })
                    .collect();

                crate::tools::factories::update_vec_deque(
                    &mut self.incompats_list,
                    new_data.into_iter(),
                    |data| incompats_display::EntryInput::UpdateData(data),
                );
            }

            IncompatsInput::DeleteIncompat(incompat_id) => {
                sender
                    .output(IncompatibilitiesUpdateOp::DeleteIncompat(incompat_id))
                    .unwrap();
            }
            IncompatsInput::EditIncompat(incompat_id) => {
                self.incompat_params_selection_reason =
                    IncompatParamsSelectionReason::Edit(incompat_id);
                let current_incompat = self
                    .incompats
                    .incompat_map
                    .get(&incompat_id)
                    .expect("Incompat ID should be valid")
                    .clone();
                self.incompat_params_dialog
                    .sender()
                    .send(incompat_params::DialogInput::Show(
                        self.subjects.clone(),
                        self.week_patterns.clone(),
                        current_incompat,
                    ))
                    .unwrap();
            }
            IncompatsInput::AddIncompat(subject_id) => {
                self.incompat_params_selection_reason = IncompatParamsSelectionReason::New;
                let default_incompat = collomatique_state_colloscopes::incompats::Incompatibility {
                    subject_id,
                    name: String::new(),
                    slots: Vec::new(),
                    minimum_free_slots: NonZeroU32::new(1).unwrap(),
                    week_pattern_id: None,
                };
                self.incompat_params_dialog
                    .sender()
                    .send(incompat_params::DialogInput::Show(
                        self.subjects.clone(),
                        self.week_patterns.clone(),
                        default_incompat,
                    ))
                    .unwrap();
            }
            IncompatsInput::IncompatParamsSelected(params) => {
                match self.incompat_params_selection_reason {
                    IncompatParamsSelectionReason::Edit(incompat_id) => {
                        sender
                            .output(IncompatibilitiesUpdateOp::UpdateIncompat(
                                incompat_id,
                                params,
                            ))
                            .unwrap();
                    }
                    IncompatParamsSelectionReason::New => {
                        sender
                            .output(IncompatibilitiesUpdateOp::AddNewIncompat(params))
                            .unwrap();
                    }
                }
            }
        }
    }
}
