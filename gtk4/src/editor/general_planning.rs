use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};
use relm4::{adw, gtk};

use collomatique_ops::GeneralPlanningUpdateOp;

mod annotation_dialog;
mod period_cut;
mod period_duration;
mod periods_display;
mod select_start_date;

#[derive(Debug)]
pub enum GeneralPlanningInput {
    Update(
        collomatique_state_colloscopes::periods::Periods,
        collomatique_state_colloscopes::weeks::Weeks,
    ),

    DeleteFirstWeekClicked,
    EditFirstWeekClicked,
    FirstWeekChanged(collomatique_time::WeekStart),

    AddPeriodClicked,
    WeekCountSelected(usize),

    EditPeriodClicked(collomatique_state_colloscopes::PeriodId),
    CutPeriodClicked(collomatique_state_colloscopes::PeriodId),
    DeletePeriodClicked(collomatique_state_colloscopes::PeriodId),
    MergePeriodClicked(collomatique_state_colloscopes::PeriodId),
    WeekStatusUpdated(collomatique_state_colloscopes::PeriodId, usize, bool),
    EditAnnotationClicked(collomatique_state_colloscopes::PeriodId, usize),
    AnnotationSelected(String),
}

#[derive(Debug)]
enum WeekCountSelectionReason {
    New,
    Edit(collomatique_state_colloscopes::PeriodId),
    Cut(collomatique_state_colloscopes::PeriodId),
}

pub struct GeneralPlanning {
    periods: collomatique_state_colloscopes::periods::Periods,
    weeks: collomatique_state_colloscopes::weeks::Weeks,
    week_selection_reason: WeekCountSelectionReason,
    periods_list: FactoryVecDeque<periods_display::Entry>,

    select_start_date_dialog: Controller<select_start_date::Dialog>,
    period_duration_dialog: Controller<period_duration::Dialog>,
    period_cut_dialog: Controller<period_cut::Dialog>,
    annotation_dialog: Controller<annotation_dialog::Dialog>,

    week_being_annotated: Option<(collomatique_state_colloscopes::PeriodId, usize)>,
}

impl GeneralPlanning {
    fn generate_first_week_text(&self) -> String {
        format!(
            "<b><big>Début de la première semaine de colles :</big></b> {}",
            match &self.periods.first_week {
                Some(date) => {
                    date.monday().format("%d/%m/%Y").to_string()
                }
                None => "non sélectionné".to_string(),
            }
        )
    }

    fn count_interrogation_weeks(&self) -> usize {
        let mut count = 0usize;
        for (_id, _week_id, v) in self.weeks.walk(&self.periods) {
            if v.interrogations {
                count += 1;
            }
        }
        count
    }

    fn generate_interrogation_week_count_text(&self) -> String {
        format!(
            "<b>Nombre total de semaines de colles :</b> {}",
            self.count_interrogation_weeks()
        )
    }

    /// The period as the shared vocabulary names it, noun-less — the row
    /// factory owns the « Période » in front.
    fn render_period(&self, id: collomatique_state_colloscopes::PeriodId) -> String {
        collomatique_ui_text::rendering::render_period(&self.periods, &self.weeks, id)
            .expect("the period comes from the document being displayed")
    }

    /// One `(week title, week state)` pair per week of a period, in order.
    fn render_weeks(
        &self,
        id: collomatique_state_colloscopes::PeriodId,
    ) -> Vec<(String, collomatique_state_colloscopes::weeks::WeekDesc)> {
        let Some(weeks) = self.weeks.weeks_for_period(id) else {
            return Vec::new();
        };
        weeks
            .map(|(week_id, week)| {
                let title = collomatique_ui_text::rendering::render_week(
                    &self.periods,
                    &self.weeks,
                    *week_id,
                )
                .expect("the week comes from the document being displayed");
                (title, week.desc())
            })
            .collect()
    }
}

#[relm4::component(pub)]
impl Component for GeneralPlanning {
    type Input = GeneralPlanningInput;
    type Output = GeneralPlanningUpdateOp;
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
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            #[watch]
                            set_label: &model.generate_first_week_text(),
                            set_use_markup: true,
                        },
                        gtk::Button {
                            set_icon_name: "document-edit-symbolic",
                            add_css_class: "flat",
                            connect_clicked => GeneralPlanningInput::EditFirstWeekClicked,
                            set_tooltip_text: Some("Modifier"),
                        },
                        gtk::Button {
                            #[watch]
                            set_sensitive: model.periods.first_week.is_some(),
                            set_icon_name: "edit-delete-symbolic",
                            add_css_class: "flat",
                            set_tooltip_text: Some("Effacer"),
                            connect_clicked => GeneralPlanningInput::DeleteFirstWeekClicked,
                        },
                    },
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        #[watch]
                        set_label: &model.generate_interrogation_week_count_text(),
                        set_use_markup: true,
                    },
                },
                #[local_ref]
                periods_box -> gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_top: 20,
                    set_spacing: 30,
                    #[watch]
                    set_visible: !model.periods.is_empty(),
                },
                gtk::Button {
                    set_margin_top: 10,
                    connect_clicked => GeneralPlanningInput::AddPeriodClicked,
                    adw::ButtonContent {
                        set_icon_name: "list-add-symbolic",
                        set_label: "Ajouter une période",
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
        let select_start_date_dialog = select_start_date::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                select_start_date::DialogOutput::Accepted(date) => {
                    GeneralPlanningInput::FirstWeekChanged(date)
                }
            });
        let period_duration_dialog = period_duration::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                period_duration::DialogOutput::Accepted(week_count) => {
                    GeneralPlanningInput::WeekCountSelected(week_count)
                }
            });
        let period_cut_dialog = period_cut::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                period_cut::DialogOutput::Accepted(week_count) => {
                    GeneralPlanningInput::WeekCountSelected(week_count)
                }
            });
        let annotation_dialog = annotation_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                annotation_dialog::DialogOutput::Accepted(new_annotation) => {
                    GeneralPlanningInput::AnnotationSelected(new_annotation)
                }
            });
        let periods_list = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |msg| match msg {
                periods_display::EntryOutput::EditClicked(period_id) => {
                    GeneralPlanningInput::EditPeriodClicked(period_id)
                }
                periods_display::EntryOutput::CutClicked(period_id) => {
                    GeneralPlanningInput::CutPeriodClicked(period_id)
                }
                periods_display::EntryOutput::DeleteClicked(period_id) => {
                    GeneralPlanningInput::DeletePeriodClicked(period_id)
                }
                periods_display::EntryOutput::MergeClicked(period_id) => {
                    GeneralPlanningInput::MergePeriodClicked(period_id)
                }
                periods_display::EntryOutput::WeekStatusUpdated(period_id, num, state) => {
                    GeneralPlanningInput::WeekStatusUpdated(period_id, num, state)
                }
                periods_display::EntryOutput::EditAnnotationClicked(period_id, num) => {
                    GeneralPlanningInput::EditAnnotationClicked(period_id, num)
                }
            });
        let model = GeneralPlanning {
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            weeks: collomatique_state_colloscopes::weeks::Weeks::default(),
            week_selection_reason: WeekCountSelectionReason::New,
            periods_list,
            select_start_date_dialog,
            period_duration_dialog,
            period_cut_dialog,
            annotation_dialog,
            week_being_annotated: None,
        };
        let periods_box = model.periods_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            GeneralPlanningInput::Update(new_periods, new_weeks) => {
                self.periods = new_periods;
                self.weeks = new_weeks;

                let new_data = self
                    .periods
                    .period_ids()
                    .map(|id| periods_display::EntryData {
                        title: self.render_period(id),
                        weeks: self.render_weeks(id),
                        period_id: id,
                    })
                    .collect::<Vec<_>>();

                crate::tools::factories::update_vec_deque(
                    &mut self.periods_list,
                    new_data.into_iter(),
                    periods_display::EntryInput::UpdateData,
                );
            }
            GeneralPlanningInput::DeleteFirstWeekClicked => {
                sender
                    .output(GeneralPlanningUpdateOp::DeleteFirstWeek)
                    .unwrap();
            }
            GeneralPlanningInput::EditFirstWeekClicked => {
                self.select_start_date_dialog
                    .sender()
                    .send(select_start_date::DialogInput::Show(
                        match &self.periods.first_week {
                            Some(date) => date.clone(),
                            None => collomatique_time::WeekStart::from_today(),
                        },
                    ))
                    .unwrap();
            }
            GeneralPlanningInput::FirstWeekChanged(date) => {
                sender
                    .output(GeneralPlanningUpdateOp::UpdateFirstWeek(date))
                    .unwrap();
            }
            GeneralPlanningInput::AddPeriodClicked => {
                self.week_selection_reason = WeekCountSelectionReason::New;
                self.period_duration_dialog
                    .sender()
                    .send(period_duration::DialogInput::Show(10))
                    .unwrap();
            }
            GeneralPlanningInput::WeekCountSelected(week_count) => sender
                .output(match self.week_selection_reason {
                    WeekCountSelectionReason::New => {
                        GeneralPlanningUpdateOp::AddNewPeriod(week_count)
                    }
                    WeekCountSelectionReason::Edit(id) => {
                        GeneralPlanningUpdateOp::UpdatePeriodWeekCount(id, week_count)
                    }
                    WeekCountSelectionReason::Cut(id) => {
                        GeneralPlanningUpdateOp::CutPeriod(id, week_count)
                    }
                })
                .unwrap(),
            GeneralPlanningInput::EditPeriodClicked(period_id) => {
                self.week_selection_reason = WeekCountSelectionReason::Edit(period_id);
                let current_week_count = self.weeks.week_count_for_period(period_id).unwrap_or(0);
                self.period_duration_dialog
                    .sender()
                    .send(period_duration::DialogInput::Show(current_week_count))
                    .unwrap();
            }
            GeneralPlanningInput::CutPeriodClicked(period_id) => {
                self.week_selection_reason = WeekCountSelectionReason::Cut(period_id);
                let current_week_count = self.weeks.week_count_for_period(period_id).unwrap_or(0);
                self.period_cut_dialog
                    .sender()
                    .send(period_cut::DialogInput::Show(current_week_count))
                    .unwrap();
            }
            GeneralPlanningInput::DeletePeriodClicked(period_id) => sender
                .output(GeneralPlanningUpdateOp::DeletePeriodAndWeeks(period_id))
                .unwrap(),
            GeneralPlanningInput::MergePeriodClicked(period_id) => sender
                .output(GeneralPlanningUpdateOp::MergeWithPreviousPeriod(period_id))
                .unwrap(),
            GeneralPlanningInput::WeekStatusUpdated(period_id, week_num, state) => sender
                .output(GeneralPlanningUpdateOp::UpdateWeekStatus(
                    period_id, week_num, state,
                ))
                .unwrap(),
            GeneralPlanningInput::EditAnnotationClicked(period_id, week_num) => {
                self.week_being_annotated = Some((period_id, week_num));
                let current_annotation = self
                    .weeks
                    .weeks_for_period(period_id)
                    .into_iter()
                    .flatten()
                    .map(|(_, w)| w)
                    .nth(week_num)
                    .expect("Week number should be valid")
                    .annotation
                    .clone()
                    .map(|x| x.into_inner())
                    .unwrap_or_default();
                self.annotation_dialog
                    .sender()
                    .send(annotation_dialog::DialogInput::Show(current_annotation))
                    .unwrap();
            }
            GeneralPlanningInput::AnnotationSelected(new_annotation) => {
                let (period_id, week_num) = self
                    .week_being_annotated
                    .take()
                    .expect("There should be a selected week for the annotation");

                sender
                    .output(GeneralPlanningUpdateOp::UpdateWeekAnnotation(
                        period_id,
                        week_num,
                        non_empty_string::NonEmptyString::new(new_annotation).ok(),
                    ))
                    .unwrap();
            }
        }
    }
}
