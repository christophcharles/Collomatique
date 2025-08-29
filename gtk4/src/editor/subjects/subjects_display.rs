use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::gtk;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque, RelmWidgetExt};
use relm4::FactorySender;

#[derive(Debug, Clone)]
pub struct PeriodData {
    pub week_count: usize,
    pub status: bool,
}

#[derive(Debug, Clone)]
pub struct EntryData {
    pub subject_params: collomatique_state_colloscopes::SubjectParameters,
    pub global_first_week: Option<collomatique_time::NaiveMondayDate>,
    pub periods: Vec<PeriodData>,
    pub subject_id: collomatique_state_colloscopes::SubjectId,
    pub subject_count: usize,
}

#[derive(Debug)]
pub struct Entry {
    index: DynamicIndex,
    subject_params: collomatique_state_colloscopes::SubjectParameters,
    global_first_week: Option<collomatique_time::NaiveMondayDate>,
    periods: FactoryVecDeque<Period>,
    subject_id: collomatique_state_colloscopes::SubjectId,
    subject_count: usize,
}

#[derive(Debug, Clone)]
pub enum EntryInput {
    UpdateData(EntryData),

    EditClicked,
    DeleteClicked,
    MoveUpClicked,
    MoveDownClicked,

    PeriodStatusUpdated(usize, bool),
}

#[derive(Debug)]
pub enum EntryOutput {
    EditClicked(collomatique_state_colloscopes::SubjectId),
    DeleteClicked(collomatique_state_colloscopes::SubjectId),
    MoveUpClicked(collomatique_state_colloscopes::SubjectId),
    MoveDownClicked(collomatique_state_colloscopes::SubjectId),
    PeriodStatusUpdated(collomatique_state_colloscopes::SubjectId, usize, bool),
}

impl Entry {
    fn range_to_text<T: Eq + ToString>(range: &std::ops::RangeInclusive<T>) -> String {
        let max = range.end();
        let min = range.start();

        if min == max {
            min.to_string()
        } else {
            format!("{} à {}", min.to_string(), max.to_string())
        }
    }

    fn generate_students_per_group_text(&self) -> String {
        format!(
            "<b>Élèves par groupes :</b> {}",
            Self::range_to_text(&self.subject_params.students_per_group),
        )
    }

    fn generate_groups_per_interrogation_text(&self) -> String {
        format!(
            "<b>Groupes par colle :</b> {}",
            Self::range_to_text(&self.subject_params.groups_per_interrogation),
        )
    }

    fn generate_periodicity_text(&self) -> String {
        use collomatique_state_colloscopes::SubjectPeriodicity;
        match &self.subject_params.periodicity {
            SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year,
                minimum_week_separation,
            } => {
                if *minimum_week_separation != 0 {
                    format!(
                        "<b>Colles dans l'année :</b> {} (séparées de {} semaines)",
                        Self::range_to_text(interrogation_count_in_year),
                        minimum_week_separation,
                    )
                } else {
                    format!(
                        "<b>Colles dans l'année :</b> {}",
                        Self::range_to_text(interrogation_count_in_year),
                    )
                }
            }
            SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks,
            } => {
                format!(
                    "<b>Périodicité :</b> {} semaines (glissantes)",
                    periodicity_in_weeks,
                )
            }
            SubjectPeriodicity::OnceForEveryBlockOfWeeks { weeks_per_block, minimum_week_separation } => {
                match minimum_week_separation.get() {
                    1 => format!(
                        "<b>Périodicité :</b> {} semaines (par bloc)",
                        weeks_per_block,
                    ),
                    _ => format!(
                        "<b>Périodicité :</b> {} semaines (par bloc - séparation de {} semaines minimum)",
                        weeks_per_block,
                        minimum_week_separation.get(),
                    )
                }
            }
            SubjectPeriodicity::AmountForEveryArbitraryBlock { blocks: _ , minimum_week_separation} => {
                match *minimum_week_separation {
                    0 => "<b>Périodicité :</b> découpage en blocs".into(),
                    1 => "<b>Périodicité :</b> découpage en blocs (séparation de 1 semaine minimum) ".into(),
                    _ => format!("<b>Périodicité :</b> découpage en blocs (séparation de {} semaines minimum)", *minimum_week_separation),
                }
            }
        }
    }

    fn generate_duration_text(&self) -> String {
        format!(
            "<i>{} minutes</i>{}",
            self.subject_params.duration.get(),
            if self.subject_params.take_duration_into_account {
                ""
            } else {
                " (non-comptées)"
            }
        )
    }

    fn update_period(
        global_first_week: Option<collomatique_time::NaiveMondayDate>,
        period_num: usize,
        first_week_in_period: usize,
        week_count: usize,
        state: bool,
    ) -> PeriodSwitchData {
        PeriodSwitchData {
            global_first_week,
            period_num,
            first_week_in_period,
            week_count,
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
                    set_label: &self.subject_params.name,
                    set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                },
                gtk::Button {
                    set_icon_name: "edit-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Modifier la matière"),
                    connect_clicked => EntryInput::EditClicked,
                },
                gtk::Box {
                    set_hexpand: true,
                },
                gtk::Button {
                    set_icon_name: "go-up",
                    add_css_class: "flat",
                    #[watch]
                    set_sensitive: self.index.current_index() != 0,
                    set_tooltip_text: Some("Remonter dans la liste"),
                    connect_clicked => EntryInput::MoveUpClicked,
                },
                gtk::Button {
                    set_icon_name: "go-down",
                    add_css_class: "flat",
                    #[watch]
                    set_sensitive: self.index.current_index() < self.subject_count-1,
                    set_tooltip_text: Some("Descendre dans la liste"),
                    connect_clicked => EntryInput::MoveDownClicked,

                },
                gtk::Separator {
                    set_orientation: gtk::Orientation::Horizontal,
                    add_css_class: "spacer",
                },
                gtk::Button {
                    set_icon_name: "edit-delete",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Supprimer la matière"),
                    connect_clicked => EntryInput::DeleteClicked,
                },
            },
            gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Horizontal,
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    #[watch]
                    set_label: &self.generate_students_per_group_text(),
                    set_use_markup: true,
                },
                gtk::Separator {
                    set_orientation: gtk::Orientation::Horizontal,
                    add_css_class: "spacer",
                },
                gtk::Separator {
                    set_orientation: gtk::Orientation::Horizontal,
                    add_css_class: "spacer",
                },
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    #[watch]
                    set_label: &self.generate_groups_per_interrogation_text(),
                    set_use_markup: true,
                },
                gtk::Separator {
                    set_orientation: gtk::Orientation::Horizontal,
                    add_css_class: "spacer",
                },
                gtk::Separator {
                    set_orientation: gtk::Orientation::Horizontal,
                    add_css_class: "spacer",
                },
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    #[watch]
                    set_label: &self.generate_periodicity_text(),
                    set_use_markup: true,
                },
                gtk::Box {
                    set_hexpand: true,
                },
                gtk::Label {
                    set_halign: gtk::Align::End,
                    #[watch]
                    set_label: &self.generate_duration_text(),
                    set_use_markup: true,
                    add_css_class: "dimmed",
                },
            },
            #[local_ref]
            periods_list -> gtk::ListBox {
                set_hexpand: true,
                add_css_class: "boxed-list",
                set_selection_mode: gtk::SelectionMode::None,
                #[watch]
                set_visible: !self.periods.is_empty(),
            },
        },
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let periods = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |msg| match msg {
                PeriodOutput::StatusChanged(period_num, status) => {
                    EntryInput::PeriodStatusUpdated(period_num, status)
                }
            });

        let mut model = Self {
            index: index.clone(),
            subject_params: data.subject_params,
            global_first_week: data.global_first_week,
            subject_id: data.subject_id,
            subject_count: data.subject_count,
            periods,
        };

        let transformed_data: Vec<_> = data
            .periods
            .into_iter()
            .enumerate()
            .scan(0usize, |current_week, (num, period_data)| {
                let new_period = Self::update_period(
                    model.global_first_week.clone(),
                    num,
                    *current_week,
                    period_data.week_count,
                    period_data.status,
                );
                *current_week += period_data.week_count;
                Some(new_period)
            })
            .collect();

        crate::tools::factories::update_vec_deque(
            &mut model.periods,
            transformed_data.into_iter(),
            |x| PeriodInput::UpdateData(x),
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
        let periods_list = self.periods.widget();
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            EntryInput::UpdateData(new_data) => {
                self.global_first_week = new_data.global_first_week;
                self.subject_params = new_data.subject_params;
                self.subject_id = new_data.subject_id;
                self.subject_count = new_data.subject_count;

                let transformed_data: Vec<_> = new_data
                    .periods
                    .into_iter()
                    .enumerate()
                    .scan(0usize, |current_week, (num, period_data)| {
                        let new_period = Self::update_period(
                            self.global_first_week.clone(),
                            num,
                            *current_week,
                            period_data.week_count,
                            period_data.status,
                        );
                        *current_week += period_data.week_count;
                        Some(new_period)
                    })
                    .collect();
                crate::tools::factories::update_vec_deque(
                    &mut self.periods,
                    transformed_data.into_iter(),
                    |x| PeriodInput::UpdateData(x),
                );
            }
            EntryInput::EditClicked => {
                sender
                    .output(EntryOutput::EditClicked(self.subject_id))
                    .unwrap();
            }
            EntryInput::DeleteClicked => {
                sender
                    .output(EntryOutput::DeleteClicked(self.subject_id))
                    .unwrap();
            }
            EntryInput::MoveUpClicked => {
                sender
                    .output(EntryOutput::MoveUpClicked(self.subject_id))
                    .unwrap();
            }
            EntryInput::MoveDownClicked => {
                sender
                    .output(EntryOutput::MoveDownClicked(self.subject_id))
                    .unwrap();
            }
            EntryInput::PeriodStatusUpdated(num, state) => {
                sender
                    .output(EntryOutput::PeriodStatusUpdated(
                        self.subject_id,
                        num,
                        state,
                    ))
                    .unwrap();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PeriodSwitchData {
    pub global_first_week: Option<collomatique_time::NaiveMondayDate>,
    pub period_num: usize,
    pub first_week_in_period: usize,
    pub week_count: usize,
    pub state: bool,
}

#[derive(Debug)]
pub struct Period {
    data: PeriodSwitchData,
}

#[derive(Debug, Clone)]
pub enum PeriodInput {
    UpdateData(PeriodSwitchData),

    StatusChanged(bool),
}

#[derive(Debug)]
pub enum PeriodOutput {
    StatusChanged(usize, bool),
}

impl Period {
    fn generate_title_text(&self) -> String {
        super::super::generate_period_title(
            &self.data.global_first_week,
            self.data.period_num,
            self.data.first_week_in_period,
            self.data.week_count,
        )
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for Period {
    type Init = PeriodSwitchData;
    type Input = PeriodInput;
    type Output = PeriodOutput;
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
                    sender.input(PeriodInput::StatusChanged(state));
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
            PeriodInput::UpdateData(new_data) => {
                self.data = new_data;
            }
            PeriodInput::StatusChanged(status) => {
                if self.data.state == status {
                    // Ignore status change that brought the component
                    // inline with internal data
                    return;
                }
                // Otherwise, bring internal data to the correct state right away
                // to avoid endless loops
                self.data.state = status;
                sender
                    .output(PeriodOutput::StatusChanged(self.data.period_num, status))
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
