use collomatique_state::traits::Manager;
use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{adw, gtk};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};

use collomatique_state::AppState;
use collomatique_state_colloscopes::Data;

mod period_cut;
mod period_duration;
mod periods_display;
mod select_start_date;

#[derive(Debug)]
pub enum GeneralPlanningInput {
    Update(collomatique_state_colloscopes::periods::Periods),

    DeleteFirstWeekClicked,
    EditFirstWeekClicked,
    FirstWeekChanged(collomatique_time::NaiveMondayDate),

    AddPeriodClicked,
    WeekCountSelected(usize),

    EditPeriodClicked(collomatique_state_colloscopes::PeriodId),
    CutPeriodClicked(collomatique_state_colloscopes::PeriodId),
    DeletePeriodClicked(collomatique_state_colloscopes::PeriodId),
    WeekStatusUpdated(collomatique_state_colloscopes::PeriodId, usize, bool),
}

#[derive(Debug)]
enum WeekCountSelectionReason {
    New,
    Edit(collomatique_state_colloscopes::PeriodId),
    Cut(collomatique_state_colloscopes::PeriodId),
}

#[derive(Debug)]
pub enum GeneralPlanningUpdateOp {
    DeleteFirstWeek,
    UpdateFirstWeek(collomatique_time::NaiveMondayDate),
    AddNewPeriod(usize),
    UpdatePeriodWeekCount(collomatique_state_colloscopes::PeriodId, usize),
    DeletePeriod(collomatique_state_colloscopes::PeriodId),
    CutPeriod(collomatique_state_colloscopes::PeriodId, usize),
    UpdateWeekStatus(collomatique_state_colloscopes::PeriodId, usize, bool),
}

impl GeneralPlanningUpdateOp {
    pub fn apply(
        &self,
        data: &mut AppState<Data>,
    ) -> Result<(), collomatique_state_colloscopes::Error> {
        match self {
            GeneralPlanningUpdateOp::DeleteFirstWeek => {
                data.apply(collomatique_state_colloscopes::Op::Period(
                    collomatique_state_colloscopes::PeriodOp::ChangeStartDate(None),
                ))
            }
            GeneralPlanningUpdateOp::UpdateFirstWeek(date) => {
                data.apply(collomatique_state_colloscopes::Op::Period(
                    collomatique_state_colloscopes::PeriodOp::ChangeStartDate(Some(date.clone())),
                ))
            }
            GeneralPlanningUpdateOp::AddNewPeriod(week_count) => {
                let new_desc = vec![true; *week_count];
                data.apply(collomatique_state_colloscopes::Op::Period(
                    match data.get_data().get_periods().ordered_period_list.last() {
                        Some((id, _)) => {
                            collomatique_state_colloscopes::PeriodOp::AddAfter(*id, new_desc)
                        }
                        None => collomatique_state_colloscopes::PeriodOp::AddFront(new_desc),
                    },
                ))
            }
            GeneralPlanningUpdateOp::UpdatePeriodWeekCount(period_id, week_count) => {
                let pos = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .expect("period id should be valid");
                let mut desc = data.get_data().get_periods().ordered_period_list[pos]
                    .1
                    .clone();

                desc.resize(*week_count, desc.last().copied().unwrap_or(true));

                data.apply(collomatique_state_colloscopes::Op::Period(
                    collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
                ))
            }
            GeneralPlanningUpdateOp::DeletePeriod(period_id) => {
                data.apply(collomatique_state_colloscopes::Op::Period(
                    collomatique_state_colloscopes::PeriodOp::Remove(*period_id),
                ))
            }
            GeneralPlanningUpdateOp::CutPeriod(period_id, new_week_count) => {
                let pos = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .expect("period id should be valid");
                let mut desc = data.get_data().get_periods().ordered_period_list[pos]
                    .1
                    .clone();
                let new_desc = desc.split_off(*new_week_count);

                let mut session = collomatique_state::AppSession::new(data);

                session.apply(collomatique_state_colloscopes::Op::Period(
                    collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
                ))?;
                session.apply(collomatique_state_colloscopes::Op::Period(
                    collomatique_state_colloscopes::PeriodOp::AddAfter(*period_id, new_desc),
                ))?;

                session.commit();
                Ok(())
            }
            GeneralPlanningUpdateOp::UpdateWeekStatus(period_id, week_num, state) => {
                let pos = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .expect("period id should be valid");
                let mut desc = data.get_data().get_periods().ordered_period_list[pos]
                    .1
                    .clone();
                desc[*week_num] = *state;

                data.apply(collomatique_state_colloscopes::Op::Period(
                    collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
                ))
            }
        }
    }
}

pub struct GeneralPlanning {
    periods: collomatique_state_colloscopes::periods::Periods,
    week_selection_reason: WeekCountSelectionReason,
    periods_list: FactoryVecDeque<periods_display::Entry>,

    select_start_date_dialog: Controller<select_start_date::Dialog>,
    period_duration_dialog: Controller<period_duration::Dialog>,
    period_cut_dialog: Controller<period_cut::Dialog>,
}

impl GeneralPlanning {
    fn generate_first_week_text(&self) -> String {
        format!(
            "<b><big>Début de la première semaine de colles :</big></b> {}",
            match &self.periods.first_week {
                Some(date) => {
                    date.inner().format("%d/%m/%Y").to_string()
                }
                None => "non sélectionné".to_string(),
            }
        )
    }

    fn count_interrogation_weeks(&self) -> usize {
        let mut count = 0usize;
        for (_id, desc) in &self.periods.ordered_period_list {
            for v in desc {
                if *v {
                    count += 1;
                }
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
                            set_icon_name: "edit-symbolic",
                            add_css_class: "flat",
                            connect_clicked => GeneralPlanningInput::EditFirstWeekClicked,
                            set_tooltip_text: Some("Modifier"),
                        },
                        gtk::Button {
                            #[watch]
                            set_sensitive: model.periods.first_week.is_some(),
                            set_icon_name: "edit-delete",
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
                    set_margin_top: 30,
                    set_margin_bottom: 30,
                    set_spacing: 30,
                },
                gtk::Button {
                    connect_clicked => GeneralPlanningInput::AddPeriodClicked,
                    adw::ButtonContent {
                        set_icon_name: "edit-add",
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
                periods_display::EntryOutput::WeekStatusUpdated(period_id, num, state) => {
                    GeneralPlanningInput::WeekStatusUpdated(period_id, num, state)
                }
            });
        let model = GeneralPlanning {
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            week_selection_reason: WeekCountSelectionReason::New,
            periods_list,
            select_start_date_dialog,
            period_duration_dialog,
            period_cut_dialog,
        };
        let periods_box = model.periods_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            GeneralPlanningInput::Update(new_periods) => {
                self.periods = new_periods;

                let new_data = self
                    .periods
                    .ordered_period_list
                    .iter()
                    .scan(0usize, |acc, (id, desc)| {
                        let current_first_week = *acc;
                        *acc += desc.len();
                        Some(periods_display::EntryData {
                            global_first_week: self.periods.first_week.clone(),
                            first_week_num: current_first_week,
                            desc: desc.clone(),
                            period_id: id.clone(),
                        })
                    })
                    .collect::<Vec<_>>();

                crate::tools::factories::update_vec_deque(
                    &mut self.periods_list,
                    new_data.into_iter(),
                    |data| periods_display::EntryInput::UpdateData(data),
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
                            None => collomatique_time::NaiveMondayDate::from_today(),
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
                let pos = self
                    .periods
                    .find_period_position(period_id)
                    .expect("valid position");
                let current_week_count = self.periods.ordered_period_list[pos].1.len();
                self.period_duration_dialog
                    .sender()
                    .send(period_duration::DialogInput::Show(current_week_count))
                    .unwrap();
            }
            GeneralPlanningInput::CutPeriodClicked(period_id) => {
                self.week_selection_reason = WeekCountSelectionReason::Cut(period_id);
                let pos = self
                    .periods
                    .find_period_position(period_id)
                    .expect("valid position");
                let current_week_count = self.periods.ordered_period_list[pos].1.len();
                self.period_cut_dialog
                    .sender()
                    .send(period_cut::DialogInput::Show(current_week_count))
                    .unwrap();
            }
            GeneralPlanningInput::DeletePeriodClicked(period_id) => sender
                .output(GeneralPlanningUpdateOp::DeletePeriod(period_id))
                .unwrap(),
            GeneralPlanningInput::WeekStatusUpdated(period_id, week_num, state) => sender
                .output(GeneralPlanningUpdateOp::UpdateWeekStatus(
                    period_id, week_num, state,
                ))
                .unwrap(),
        }
    }
}
