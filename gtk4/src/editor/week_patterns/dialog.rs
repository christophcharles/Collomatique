use adw::prelude::{EditableExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::FactorySender;
use relm4::{adw, gtk};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    periods: collomatique_state_colloscopes::periods::Periods,
    week_pattern: collomatique_state_colloscopes::week_patterns::WeekPattern,
    period_entries: FactoryVecDeque<PeriodEntry>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        collomatique_state_colloscopes::periods::Periods,
        collomatique_state_colloscopes::week_patterns::WeekPattern,
    ),
    Cancel,
    Accept,
    UpdateName(String),
    UpdateStatusInPattern(usize, bool),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(collomatique_state_colloscopes::week_patterns::WeekPattern),
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
            set_title: Some("Configuration du modèle"),
            set_size_request: (500, 700),
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
                        connect_clicked => DialogInput::Accept,
                    },
                },
                #[name(scrolled_window)]
                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_hexpand: true,
                    set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                    gtk::Box {
                        set_hexpand: true,
                        set_margin_all: 5,
                        set_spacing: 10,
                        set_orientation: gtk::Orientation::Vertical,
                        adw::PreferencesGroup {
                            set_title: "",
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[name(name_entry)]
                            adw::EntryRow {
                                set_hexpand: true,
                                set_title: "Nom du modèle",
                                #[track(model.should_redraw)]
                                set_text: &model.week_pattern.name,
                                connect_text_notify[sender] => move |widget| {
                                    let text : String = widget.text().into();
                                    sender.input(DialogInput::UpdateName(text));
                                },
                            },
                        },
                        #[local_ref]
                        period_entries_widget -> gtk::Box {
                            set_hexpand: true,
                            set_margin_all: 0,
                            set_spacing: 10,
                            set_orientation: gtk::Orientation::Vertical,
                        },
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
        let periods = collomatique_state_colloscopes::periods::Periods::default();
        let week_pattern = collomatique_state_colloscopes::week_patterns::WeekPattern {
            name: "Placeholder".into(),
            weeks: vec![],
        };

        let period_entries = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |msg| match msg {
                PeriodOutput::UpdateStatusInPattern(week_num, new_status) => {
                    DialogInput::UpdateStatusInPattern(week_num, new_status)
                }
            });

        let model = Dialog {
            hidden: true,
            should_redraw: false,
            periods,
            week_pattern,
            period_entries,
        };

        let period_entries_widget = model.period_entries.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(periods, week_pattern) => {
                self.hidden = false;
                self.should_redraw = true;
                self.periods = periods;
                self.week_pattern = week_pattern;

                let new_data = self
                    .periods
                    .ordered_period_list
                    .iter()
                    .scan(0usize, |acc, (_id, desc)| {
                        let current_first_week = *acc;
                        *acc += desc.len();
                        Some(PeriodData {
                            global_first_week: self.periods.first_week.clone(),
                            first_week_num: current_first_week,
                            period_desc: desc.clone(),
                            weeks_in_pattern: (current_first_week
                                ..(current_first_week + desc.len()))
                                .into_iter()
                                .map(|index| {
                                    self.week_pattern.weeks.get(index).cloned().unwrap_or(true)
                                })
                                .collect(),
                        })
                    })
                    .collect::<Vec<_>>();

                crate::tools::factories::update_vec_deque(
                    &mut self.period_entries,
                    new_data.into_iter(),
                    |data| PeriodInput::UpdateData(data),
                );
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.week_pattern.clone()))
                    .unwrap();
            }
            DialogInput::UpdateName(new_name) => {
                if self.week_pattern.name == new_name {
                    return;
                }
                self.week_pattern.name = new_name;
            }
            DialogInput::UpdateStatusInPattern(week_num, new_status) => {
                if self
                    .week_pattern
                    .weeks
                    .get(week_num)
                    .cloned()
                    .unwrap_or(true)
                    == new_status
                {
                    return;
                }
                if week_num >= self.week_pattern.weeks.len() {
                    self.week_pattern.weeks.resize(week_num + 1, true);
                }
                self.week_pattern.weeks[week_num] = new_status;
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.should_redraw {
            let adj = widgets.scrolled_window.vadjustment();
            adj.set_value(0.);
            widgets.name_entry.grab_focus();
        }
    }
}

#[derive(Debug, Clone)]
struct PeriodData {
    global_first_week: Option<collomatique_time::NaiveMondayDate>,
    first_week_num: usize,
    period_desc: Vec<bool>,
    weeks_in_pattern: Vec<bool>,
}

#[derive(Debug)]
struct PeriodEntry {
    data: PeriodData,
    index: DynamicIndex,
    should_redraw: bool,
    week_entries: FactoryVecDeque<WeekEntry>,
}

#[derive(Debug, Clone)]
enum PeriodInput {
    UpdateData(PeriodData),
    UpdateStatusInPattern(usize, bool),
}

#[derive(Debug, Clone)]
enum PeriodOutput {
    UpdateStatusInPattern(usize, bool),
}

impl PeriodEntry {
    fn generate_period_title(&self) -> String {
        super::super::generate_period_title(
            &self.data.global_first_week,
            self.index.current_index(),
            self.data.first_week_num,
            self.data.period_desc.len(),
        )
    }
}

#[relm4::factory]
impl FactoryComponent for PeriodEntry {
    type Init = PeriodData;
    type Input = PeriodInput;
    type Output = PeriodOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Box {
            set_hexpand: true,
            #[local_ref]
            week_entries_widget -> adw::PreferencesGroup {
                #[watch]
                set_title: &self.generate_period_title(),
                set_margin_all: 5,
                set_hexpand: true,
            },
        },
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let week_entries = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |msg| match msg {
                WeekOutput::UpdateStatusInPattern(index, new_status) => {
                    PeriodInput::UpdateStatusInPattern(index, new_status)
                }
            });

        let mut model = Self {
            data,
            index: index.clone(),
            should_redraw: false,
            week_entries,
        };

        model.update_factory();

        model
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let week_entries_widget = self.week_entries.widget();
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        self.should_redraw = false;
        match msg {
            PeriodInput::UpdateData(new_data) => {
                self.data = new_data;
                self.should_redraw = true;

                self.update_factory();
            }
            PeriodInput::UpdateStatusInPattern(index, new_status) => {
                if self.data.weeks_in_pattern[index] == new_status {
                    return;
                }
                self.data.weeks_in_pattern[index] = new_status;
                let global_index = self.data.first_week_num + index;
                sender
                    .output(PeriodOutput::UpdateStatusInPattern(
                        global_index,
                        new_status,
                    ))
                    .unwrap();
            }
        }
    }
}

impl PeriodEntry {
    fn update_factory(&mut self) {
        assert_eq!(
            self.data.weeks_in_pattern.len(),
            self.data.period_desc.len()
        );
        crate::tools::factories::update_vec_deque(
            &mut self.week_entries,
            self.data
                .weeks_in_pattern
                .iter()
                .enumerate()
                .map(|(index, status_in_pattern)| WeekData {
                    global_first_week: self.data.global_first_week.clone(),
                    first_week_num: self.data.first_week_num,
                    status_in_period: self.data.period_desc[index],
                    status_in_pattern: *status_in_pattern,
                }),
            |data| WeekInput::UpdateData(data),
        );
    }
}

#[derive(Debug, Clone)]
struct WeekData {
    global_first_week: Option<collomatique_time::NaiveMondayDate>,
    first_week_num: usize,
    status_in_period: bool,
    status_in_pattern: bool,
}

#[derive(Debug)]
struct WeekEntry {
    data: WeekData,
    index: DynamicIndex,
    should_redraw: bool,
}

#[derive(Debug, Clone)]
enum WeekInput {
    UpdateData(WeekData),
    UpdateStatusInPattern(bool),
}

#[derive(Debug, Clone)]
enum WeekOutput {
    UpdateStatusInPattern(usize, bool),
}

impl WeekEntry {
    fn generate_week_title(&self) -> String {
        let week_number = self.data.first_week_num + self.index.current_index();
        super::super::generate_week_title(&self.data.global_first_week, week_number)
    }
}

#[relm4::factory]
impl FactoryComponent for WeekEntry {
    type Init = WeekData;
    type Input = WeekInput;
    type Output = WeekOutput;
    type CommandOutput = ();
    type ParentWidget = adw::PreferencesGroup;

    view! {
        #[root]
        root_widget = adw::SwitchRow {
            set_hexpand: true,
            set_use_markup: false,
            #[watch]
            set_title: &self.generate_week_title(),
            #[track(self.should_redraw)]
            set_active: self.data.status_in_pattern,
            connect_active_notify[sender] => move |widget| {
                let status = widget.is_active();
                sender.input(
                    WeekInput::UpdateStatusInPattern(status)
                );
            },
        },
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            data,
            index: index.clone(),
            should_redraw: false,
        }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        if !self.data.status_in_period {
            root.add_css_class("dimmed");
        }

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        self.should_redraw = false;
        match msg {
            WeekInput::UpdateData(new_data) => {
                self.data = new_data;
                self.should_redraw = true;
            }
            WeekInput::UpdateStatusInPattern(new_status) => {
                if self.data.status_in_pattern == new_status {
                    return;
                }
                self.data.status_in_pattern = new_status;
                sender
                    .output(WeekOutput::UpdateStatusInPattern(
                        self.index.current_index(),
                        new_status,
                    ))
                    .unwrap();
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.data.status_in_period {
            widgets.root_widget.remove_css_class("dimmed");
        } else {
            widgets.root_widget.add_css_class("dimmed");
        }
    }
}
