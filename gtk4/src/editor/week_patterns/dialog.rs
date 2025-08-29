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
            set_size_request: (500, 300),
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
            .detach();

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
                    .scan(0usize, |acc, (id, desc)| {
                        let current_first_week = *acc;
                        *acc += desc.len();
                        Some(PeriodData {
                            global_first_week: self.periods.first_week.clone(),
                            first_week_num: current_first_week,
                            desc: desc.clone(),
                            period_id: id.clone(),
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
    desc: Vec<bool>,
    period_id: collomatique_state_colloscopes::PeriodId,
}

#[derive(Debug)]
struct PeriodEntry {
    data: PeriodData,
    index: DynamicIndex,
    should_redraw: bool,
}

#[derive(Debug, Clone)]
enum PeriodInput {
    UpdateData(PeriodData),
}

#[relm4::factory]
impl FactoryComponent for PeriodEntry {
    type Init = PeriodData;
    type Input = PeriodInput;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        adw::PreferencesGroup {
            set_title: "Période 42 (semaines 2 à 4)",
            set_margin_all: 5,
            set_hexpand: true,
            adw::SwitchRow {
                set_hexpand: true,
                set_title: "Semaine 2",
            },
            adw::SwitchRow {
                set_hexpand: true,
                set_title: "Semaine 3",
            },
            adw::SwitchRow {
                set_hexpand: true,
                set_title: "Semaine 4",
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

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        self.should_redraw = false;
        match msg {
            PeriodInput::UpdateData(new_data) => {
                self.data = new_data;
                self.should_redraw = true;
            }
        }
    }
}
