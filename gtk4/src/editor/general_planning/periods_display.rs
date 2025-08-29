use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque, RelmWidgetExt};
use relm4::FactorySender;

#[derive(Debug, Clone)]
pub struct EntryData {
    pub global_first_week: Option<collomatique_time::NaiveMondayDate>,
    pub first_week_num: usize,
    pub desc: Vec<bool>,
    pub period_id: collomatique_state_colloscopes::PeriodId,
}

#[derive(Debug)]
pub struct Entry {
    index: DynamicIndex,
    global_first_week: Option<collomatique_time::NaiveMondayDate>,
    first_week_num: usize,
    period_id: collomatique_state_colloscopes::PeriodId,
    weeks: FactoryVecDeque<Week>,
}

#[derive(Debug, Clone)]
pub enum EntryInput {
    UpdateData(EntryData),

    EditClicked,
    DeleteClicked,
    CutClicked,
    MergeClicked,

    WeekStatusUpdated(usize, bool),
}

#[derive(Debug)]
pub enum EntryOutput {
    EditClicked(collomatique_state_colloscopes::PeriodId),
    DeleteClicked(collomatique_state_colloscopes::PeriodId),
    CutClicked(collomatique_state_colloscopes::PeriodId),
    MergeClicked(collomatique_state_colloscopes::PeriodId),
    WeekStatusUpdated(collomatique_state_colloscopes::PeriodId, usize, bool),
}

impl Entry {
    fn generate_title_text(&self) -> String {
        let week_count = self.weeks.len();
        let index = self.index.current_index() + 1;

        if week_count == 0 {
            return format!("<b><big>Période {} (vide)</big></b>", index,);
        }

        let start_week = self.first_week_num + 1;
        let end_week = self.first_week_num + week_count;

        match &self.global_first_week {
            Some(global_start_date) => {
                let start_date = global_start_date
                    .inner()
                    .checked_add_days(chrono::Days::new(7 * (self.first_week_num as u64)))
                    .expect("Valid start date");
                let end_date = start_date
                    .checked_add_days(chrono::Days::new(7 * (week_count as u64) - 1))
                    .expect("Valid end date");
                if start_week != end_week {
                    format!(
                        "<b><big>Période {} du {} au {} (semaines {} à {})</big></b>",
                        index,
                        start_date.format("%d/%m/%Y").to_string(),
                        end_date.format("%d/%m/%Y").to_string(),
                        start_week,
                        end_week,
                    )
                } else {
                    format!(
                        "<b><big>Période {} du {} au {} (semaine {})</big></b>",
                        index,
                        start_date.format("%d/%m/%Y").to_string(),
                        end_date.format("%d/%m/%Y").to_string(),
                        start_week,
                    )
                }
            }
            None => {
                if start_week != end_week {
                    format!(
                        "<b><big>Période {} (semaines {} à {})</big></b>",
                        index, start_week, end_week,
                    )
                } else {
                    format!(
                        "<b><big>Période {} (semaine {})</big></b>",
                        index, start_week,
                    )
                }
            }
        }
    }

    fn update_week(
        global_first_week: Option<collomatique_time::NaiveMondayDate>,
        first_week_in_period: usize,
        week_num_in_period: usize,
        state: bool,
    ) -> WeekData {
        WeekData {
            global_first_week: global_first_week,
            first_week_in_period,
            week_num_in_period,
            state,
        }
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for Entry {
    type Init = EntryData;
    type Input = EntryInput;
    type Output = EntryOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Box {
            set_hexpand: true,
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,
            gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Horizontal,
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    #[watch]
                    set_label: &self.generate_title_text(),
                    set_use_markup: true,
                },
                gtk::Button {
                    set_icon_name: "edit-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Modifier la période"),
                    connect_clicked => EntryInput::EditClicked,
                },
                gtk::Button {
                    set_icon_name: "edit-cut-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Découper la période"),
                    connect_clicked => EntryInput::CutClicked,
                },
                gtk::Box {
                    set_hexpand: true,
                },
                gtk::Button {
                    set_icon_name: "go-up",
                    add_css_class: "flat",
                    #[watch]
                    set_visible: self.index.current_index() != 0,
                    set_tooltip_text: Some("Fusionner avec la période précédente"),
                    connect_clicked => EntryInput::MergeClicked,
                },
                gtk::Button {
                    set_icon_name: "edit-delete",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Supprimer la période"),
                    connect_clicked => EntryInput::DeleteClicked,
                },
            },
            #[local_ref]
            weeks_list -> gtk::ListBox {
                set_hexpand: true,
                add_css_class: "boxed-list",
                set_selection_mode: gtk::SelectionMode::None,
            },
        },
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let weeks = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |msg| match msg {
                WeekOutput::StatusChanged(week_num, status) => {
                    EntryInput::WeekStatusUpdated(week_num, status)
                }
            });

        let mut model = Self {
            index: index.clone(),
            global_first_week: data.global_first_week,
            first_week_num: data.first_week_num,
            period_id: data.period_id,
            weeks,
        };

        crate::tools::factories::update_vec_deque(
            &mut model.weeks,
            data.desc.into_iter().enumerate().map(|(num, state)| {
                Self::update_week(
                    model.global_first_week.clone(),
                    model.first_week_num,
                    num,
                    state,
                )
            }),
            |x| WeekInput::UpdateData(x),
        );

        model
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let weeks_list = self.weeks.widget();
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            EntryInput::UpdateData(new_data) => {
                self.global_first_week = new_data.global_first_week;
                self.first_week_num = new_data.first_week_num;
                self.period_id = new_data.period_id;
                crate::tools::factories::update_vec_deque(
                    &mut self.weeks,
                    new_data.desc.into_iter().enumerate().map(|(num, state)| {
                        Self::update_week(
                            self.global_first_week.clone(),
                            self.first_week_num,
                            num,
                            state,
                        )
                    }),
                    |x| WeekInput::UpdateData(x),
                );
            }
            EntryInput::EditClicked => {
                sender
                    .output(EntryOutput::EditClicked(self.period_id))
                    .unwrap();
            }
            EntryInput::CutClicked => {
                sender
                    .output(EntryOutput::CutClicked(self.period_id))
                    .unwrap();
            }
            EntryInput::MergeClicked => {
                sender
                    .output(EntryOutput::MergeClicked(self.period_id))
                    .unwrap();
            }
            EntryInput::DeleteClicked => {
                sender
                    .output(EntryOutput::DeleteClicked(self.period_id))
                    .unwrap();
            }
            EntryInput::WeekStatusUpdated(num, state) => {
                sender
                    .output(EntryOutput::WeekStatusUpdated(self.period_id, num, state))
                    .unwrap();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct WeekData {
    pub global_first_week: Option<collomatique_time::NaiveMondayDate>,
    pub first_week_in_period: usize,
    pub week_num_in_period: usize,
    pub state: bool,
}

#[derive(Debug)]
pub struct Week {
    data: WeekData,
}

#[derive(Debug, Clone)]
pub enum WeekInput {
    UpdateData(WeekData),

    StatusChanged(bool),
}

#[derive(Debug)]
pub enum WeekOutput {
    StatusChanged(usize, bool),
}

impl Week {
    fn generate_title_text(&self) -> String {
        let week_number = self.data.first_week_in_period + self.data.week_num_in_period;
        match &self.data.global_first_week {
            Some(global_start_date) => {
                let start_date = global_start_date
                    .inner()
                    .checked_add_days(chrono::Days::new(7 * (week_number as u64)))
                    .expect("Valid start date");
                let end_date = start_date
                    .checked_add_days(chrono::Days::new(6))
                    .expect("Valid end date");
                format!(
                    "Semaine {} du {} au {}",
                    week_number + 1,
                    start_date.format("%d/%m/%Y").to_string(),
                    end_date.format("%d/%m/%Y").to_string(),
                )
            }
            None => {
                format!("Semaine {}", week_number + 1)
            }
        }
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for Week {
    type Init = WeekData;
    type Input = WeekInput;
    type Output = WeekOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        root_widget = gtk::Box {
            set_hexpand: true,
            set_margin_all: 5,
            set_orientation: gtk::Orientation::Horizontal,
            gtk::Label {
                set_margin_all: 5,
                #[watch]
                set_label: &self.generate_title_text(),
            },
            gtk::Box {
                set_hexpand: true,
            },
            #[name(switch)]
            gtk::Switch {
                #[track(self.data.state != switch.is_active())]
                set_active: self.data.state,
                connect_state_set[sender] => move |_widget,state| {
                    sender.input(WeekInput::StatusChanged(state));
                    gtk::glib::Propagation::Proceed
                }
            },
        }
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { data }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        if !self.data.state {
            widgets.root_widget.add_css_class("dimmed");
        }

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            WeekInput::UpdateData(new_data) => {
                self.data = new_data;
            }
            WeekInput::StatusChanged(status) => {
                if self.data.state == status {
                    // Ignore status change that brought the component
                    // inline with internal data
                    return;
                }
                // Otherwise, bring internal data to the correct state right away
                // to avoid endless loops
                self.data.state = status;
                sender
                    .output(WeekOutput::StatusChanged(
                        self.data.week_num_in_period,
                        status,
                    ))
                    .unwrap();
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.data.state {
            widgets.root_widget.remove_css_class("dimmed");
        } else {
            widgets.root_widget.add_css_class("dimmed");
        }
    }
}
