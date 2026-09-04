use adw::prelude::{ActionRowExt, ComboRowExt, PreferencesRowExt};
use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::FactorySender;
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque, RelmWidgetExt};
use relm4::{adw, gtk};

#[derive(Debug, Clone)]
pub struct PeriodData {
    /// The period as [collomatique_ui_text::rendering::render_period] names it.
    pub title: String,
    pub status: bool,
}

#[derive(Debug, Clone)]
pub struct EntryData {
    pub subject_params: collomatique_state_colloscopes::SubjectParameters,
    /// The week patterns the row offers, in the order it offers them.
    pub ordered_week_patterns: Vec<(collomatique_state_colloscopes::WeekPatternId, String)>,
    pub week_pattern: Option<collomatique_state_colloscopes::WeekPatternId>,
    pub periods: Vec<PeriodData>,
    pub subject_id: collomatique_state_colloscopes::SubjectId,
    pub subject_count: usize,
}

#[derive(Debug)]
pub struct Entry {
    index: DynamicIndex,
    subject_params: collomatique_state_colloscopes::SubjectParameters,
    ordered_week_patterns: Vec<(collomatique_state_colloscopes::WeekPatternId, String)>,
    week_pattern: Option<collomatique_state_colloscopes::WeekPatternId>,
    /// Whether the week pattern list itself just changed, so the combo row has
    /// to be given a new model. Rebuilding it resets the row's selection, so it
    /// must not happen on every redraw.
    week_pattern_list_changed: bool,
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
    WeekPatternSelected(u32),
}

#[derive(Debug)]
pub enum EntryOutput {
    EditClicked(collomatique_state_colloscopes::SubjectId),
    DeleteClicked(collomatique_state_colloscopes::SubjectId),
    MoveUpClicked(collomatique_state_colloscopes::SubjectId),
    MoveDownClicked(collomatique_state_colloscopes::SubjectId),
    PeriodStatusUpdated(collomatique_state_colloscopes::SubjectId, usize, bool),
    WeekPatternUpdated(
        collomatique_state_colloscopes::SubjectId,
        Option<collomatique_state_colloscopes::WeekPatternId>,
    ),
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
        if let Some(interrogation_parameters) = &self.subject_params.interrogation_parameters {
            format!(
                "<b>Élèves par groupes :</b> {}",
                Self::range_to_text(&interrogation_parameters.students_per_group),
            )
        } else {
            String::new()
        }
    }

    fn generate_groups_per_interrogation_text(&self) -> String {
        if let Some(interrogation_parameters) = &self.subject_params.interrogation_parameters {
            format!(
                "<b>Groupes par colle :</b> {}",
                Self::range_to_text(&interrogation_parameters.groups_per_interrogation),
            )
        } else {
            String::new()
        }
    }

    fn generate_periodicity_text(&self) -> String {
        let Some(interrogation_parameters) = &self.subject_params.interrogation_parameters else {
            return String::new();
        };
        use collomatique_state_colloscopes::SubjectPeriodicity;
        match &interrogation_parameters.periodicity {
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
            SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                weeks_per_block,
                minimum_week_separation,
            } => match minimum_week_separation.get() {
                1 => format!(
                    "<b>Périodicité :</b> {} semaines (par bloc)",
                    weeks_per_block,
                ),
                _ => format!(
                    "<b>Périodicité :</b> {} semaines (par bloc - séparation de {} semaines minimum)",
                    weeks_per_block,
                    minimum_week_separation.get(),
                ),
            },
            SubjectPeriodicity::AmountForEveryArbitraryBlock {
                blocks: _,
                minimum_week_separation,
            } => match *minimum_week_separation {
                0 => "<b>Périodicité :</b> découpage en blocs".into(),
                1 => "<b>Périodicité :</b> découpage en blocs (séparation de 1 semaine minimum) "
                    .into(),
                _ => format!(
                    "<b>Périodicité :</b> découpage en blocs (séparation de {} semaines minimum)",
                    *minimum_week_separation
                ),
            },
        }
    }

    fn generate_duration_text(&self) -> String {
        let Some(interrogation_parameters) = &self.subject_params.interrogation_parameters else {
            return String::new();
        };

        format!(
            "<i>{} minutes</i>{}",
            interrogation_parameters.duration.get(),
            if interrogation_parameters.take_duration_into_account {
                ""
            } else {
                " (non-comptées)"
            }
        )
    }

    fn generate_week_patterns_model(&self) -> gtk::StringList {
        let week_pattern_names_list: Vec<_> = ["Aucun (toutes les semaines)"]
            .into_iter()
            .chain(
                self.ordered_week_patterns
                    .iter()
                    .map(|(_id, name)| name.as_str()),
            )
            .collect();
        gtk::StringList::new(&week_pattern_names_list[..])
    }

    fn week_pattern_selected(&self) -> u32 {
        let Some(week_pattern_id) = self.week_pattern else {
            return 0;
        };
        for (i, (id, _)) in self.ordered_week_patterns.iter().enumerate() {
            if *id == week_pattern_id {
                return (i as u32) + 1;
            }
        }
        panic!("Week pattern ID should be in list");
    }

    fn week_pattern_selected_to_id(
        &self,
        selected: u32,
    ) -> Option<collomatique_state_colloscopes::WeekPatternId> {
        if selected == 0 {
            return None;
        }
        Some(self.ordered_week_patterns[(selected - 1) as usize].0)
    }

    fn period_switch_data(
        periods: Vec<PeriodData>,
    ) -> impl ExactSizeIterator<Item = PeriodSwitchData> {
        periods
            .into_iter()
            .enumerate()
            .map(|(period_num, period_data)| PeriodSwitchData {
                title: period_data.title,
                period_num,
                state: period_data.status,
            })
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
                    set_icon_name: "document-edit-symbolic",
                    add_css_class: "flat",
                    set_tooltip_text: Some("Modifier la matière"),
                    connect_clicked => EntryInput::EditClicked,
                },
                gtk::Box {
                    set_hexpand: true,
                },
                gtk::Button {
                    set_icon_name: "go-up-symbolic",
                    add_css_class: "flat",
                    #[watch]
                    set_sensitive: self.index.current_index() != 0,
                    set_tooltip_text: Some("Remonter dans la liste"),
                    connect_clicked => EntryInput::MoveUpClicked,
                },
                gtk::Button {
                    set_icon_name: "go-down-symbolic",
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
                    set_icon_name: "edit-delete-symbolic",
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
                    set_label: "<b>Pas de colles</b>",
                    set_use_markup: true,
                    #[watch]
                    set_visible: self.subject_params.interrogation_parameters.is_none(),
                },
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    #[watch]
                    set_label: &self.generate_students_per_group_text(),
                    set_use_markup: true,
                    #[watch]
                    set_visible: self.subject_params.interrogation_parameters.is_some(),
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
                    #[watch]
                    set_visible: self.subject_params.interrogation_parameters.is_some(),
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
                    #[watch]
                    set_visible: self.subject_params.interrogation_parameters.is_some(),
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
                    #[watch]
                    set_visible: self.subject_params.interrogation_parameters.is_some(),
                },
            },
            gtk::ListBox {
                set_hexpand: true,
                add_css_class: "boxed-list",
                set_selection_mode: gtk::SelectionMode::None,
                #[name(week_pattern_row)]
                adw::ComboRow {
                    set_title: "Modèle de périodicité",
                    set_subtitle: "Restreint les semaines où la matière a des colles",
                    #[track(self.week_pattern_list_changed)]
                    #[block_signal(week_pattern_handler)]
                    set_model: Some(&self.generate_week_patterns_model()),
                    #[track(week_pattern_row.selected() != self.week_pattern_selected())]
                    #[block_signal(week_pattern_handler)]
                    set_selected: self.week_pattern_selected(),
                    connect_selected_notify[sender] => move |widget| {
                        sender.input(EntryInput::WeekPatternSelected(widget.selected()));
                    } @week_pattern_handler,
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
            ordered_week_patterns: data.ordered_week_patterns,
            week_pattern: data.week_pattern,
            week_pattern_list_changed: true,
            subject_id: data.subject_id,
            subject_count: data.subject_count,
            periods,
        };

        crate::tools::factories::update_vec_deque(
            &mut model.periods,
            Self::period_switch_data(data.periods),
            PeriodInput::UpdateData,
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
        self.week_pattern_list_changed = false;
        match msg {
            EntryInput::UpdateData(new_data) => {
                self.subject_params = new_data.subject_params;
                self.week_pattern_list_changed =
                    self.ordered_week_patterns != new_data.ordered_week_patterns;
                self.ordered_week_patterns = new_data.ordered_week_patterns;
                self.week_pattern = new_data.week_pattern;
                self.subject_id = new_data.subject_id;
                self.subject_count = new_data.subject_count;

                crate::tools::factories::update_vec_deque(
                    &mut self.periods,
                    Self::period_switch_data(new_data.periods),
                    PeriodInput::UpdateData,
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
            EntryInput::WeekPatternSelected(selected) => {
                let new_week_pattern = self.week_pattern_selected_to_id(selected);
                if self.week_pattern == new_week_pattern {
                    // Ignore a selection that brought the row inline with
                    // internal data
                    return;
                }
                // Otherwise, bring internal data to the correct state right away
                // to avoid endless loops
                self.week_pattern = new_week_pattern;
                sender
                    .output(EntryOutput::WeekPatternUpdated(
                        self.subject_id,
                        new_week_pattern,
                    ))
                    .unwrap();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PeriodSwitchData {
    /// The period as [collomatique_ui_text::rendering::render_period] names it.
    pub title: String,
    pub period_num: usize,
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
        format!("Période {}", self.data.title)
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
