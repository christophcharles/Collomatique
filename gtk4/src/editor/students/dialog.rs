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
    student_data: collomatique_state_colloscopes::students::Student,
    periods: collomatique_state_colloscopes::periods::Periods,
    period_entries: FactoryVecDeque<PeriodEntry>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        collomatique_state_colloscopes::periods::Periods,
        collomatique_state_colloscopes::students::Student,
    ),
    Cancel,
    Accept,

    UpdateFirstname(String),
    UpdateSurname(String),
    UpdateTelephone(String),
    UpdateEmail(String),
    UpdatePeriodStatus(usize, bool),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(collomatique_state_colloscopes::students::Student),
}

impl Dialog {
    fn generate_email_text(&self) -> String {
        match &self.student_data.desc.email {
            None => String::new(),
            Some(text) => text.clone().into_inner(),
        }
    }

    fn generate_telephone_text(&self) -> String {
        match &self.student_data.desc.tel {
            None => String::new(),
            Some(text) => text.clone().into_inner(),
        }
    }
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
            set_title: Some("Configuration de l'élève"),
            set_default_size: (500, 300),
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
                            set_title: "Nom de l'élève",
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[name(firstname_entry)]
                            adw::EntryRow {
                                set_hexpand: true,
                                set_title: "Prénom",
                                #[track(model.should_redraw)]
                                set_text: &model.student_data.desc.firstname,
                                connect_text_notify[sender] => move |widget| {
                                    let text : String = widget.text().into();
                                    sender.input(DialogInput::UpdateFirstname(text));
                                },
                            },
                            #[name(surname_entry)]
                            adw::EntryRow {
                                set_hexpand: true,
                                set_title: "Nom de famille",
                                #[track(model.should_redraw)]
                                set_text: &model.student_data.desc.surname,
                                connect_text_notify[sender] => move |widget| {
                                    let text : String = widget.text().into();
                                    sender.input(DialogInput::UpdateSurname(text));
                                },
                            },
                        },
                        adw::PreferencesGroup {
                            set_title: "Contact",
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[name(tel_entry)]
                            adw::EntryRow {
                                set_hexpand: true,
                                set_title: "Téléphone",
                                #[track(model.should_redraw)]
                                set_text: &model.generate_telephone_text(),
                                connect_text_notify[sender] => move |widget| {
                                    let text : String = widget.text().into();
                                    sender.input(DialogInput::UpdateTelephone(text));
                                },
                            },
                            #[name(email_entry)]
                            adw::EntryRow {
                                set_hexpand: true,
                                set_title: "E-mail",
                                #[track(model.should_redraw)]
                                set_text: &model.generate_email_text(),
                                connect_text_notify[sender] => move |widget| {
                                    let text : String = widget.text().into();
                                    sender.input(DialogInput::UpdateEmail(text));
                                },
                            },
                        },
                        #[local_ref]
                        period_entries_widget -> adw::PreferencesGroup {
                            set_title: "Périodes concernées",
                            set_margin_all: 5,
                            set_hexpand: true,
                            #[watch]
                            set_visible: !model.periods.ordered_period_list.is_empty(),
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
        let student_data = collomatique_state_colloscopes::students::Student::default();
        let periods = collomatique_state_colloscopes::periods::Periods::default();

        let period_entries = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |msg| match msg {
                PeriodOutput::UpdateStatus(num, status) => {
                    DialogInput::UpdatePeriodStatus(num, status)
                }
            });

        let model = Dialog {
            hidden: true,
            should_redraw: false,
            student_data,
            periods,
            period_entries,
        };

        let period_entries_widget = model.period_entries.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(periods, student_data) => {
                self.hidden = false;
                self.should_redraw = true;
                self.periods = periods;
                self.student_data = student_data;

                let transformed_data: Vec<_> = self
                    .periods
                    .ordered_period_list
                    .iter()
                    .scan(0usize, |current_week, (id, period_data)| {
                        let new_period = PeriodData {
                            global_first_week: self.periods.first_week.clone(),
                            first_week_num: *current_week,
                            week_count: period_data.len(),
                            enable: !self.student_data.excluded_periods.contains(id),
                        };
                        *current_week += period_data.len();
                        Some(new_period)
                    })
                    .collect();

                crate::tools::factories::update_vec_deque(
                    &mut self.period_entries,
                    transformed_data.into_iter(),
                    |x| PeriodInput::UpdateData(x),
                );
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.student_data.clone()))
                    .unwrap();
            }
            DialogInput::UpdateFirstname(new_firstname) => {
                if self.student_data.desc.firstname == new_firstname {
                    return;
                }
                self.student_data.desc.firstname = new_firstname;
            }
            DialogInput::UpdateSurname(new_surname) => {
                if self.student_data.desc.surname == new_surname {
                    return;
                }
                self.student_data.desc.surname = new_surname;
            }
            DialogInput::UpdateTelephone(new_tel) => {
                let tel_opt = non_empty_string::NonEmptyString::new(new_tel).ok();
                if self.student_data.desc.tel == tel_opt {
                    return;
                }
                self.student_data.desc.tel = tel_opt;
            }
            DialogInput::UpdateEmail(new_email) => {
                let email_opt = non_empty_string::NonEmptyString::new(new_email).ok();
                if self.student_data.desc.email == email_opt {
                    return;
                }
                self.student_data.desc.email = email_opt;
            }
            DialogInput::UpdatePeriodStatus(period_num, new_status) => {
                assert!(period_num < self.periods.ordered_period_list.len());
                let period_id = self.periods.ordered_period_list[period_num].0;

                if new_status {
                    self.student_data.excluded_periods.remove(&period_id);
                } else {
                    self.student_data.excluded_periods.insert(period_id);
                }
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.should_redraw {
            let adj = widgets.scrolled_window.vadjustment();
            adj.set_value(0.);
            widgets.firstname_entry.grab_focus();
        }
    }
}

#[derive(Debug, Clone)]
struct PeriodData {
    global_first_week: Option<collomatique_time::NaiveMondayDate>,
    first_week_num: usize,
    week_count: usize,
    enable: bool,
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

    UpdateStatus(bool),
}

#[derive(Debug)]
enum PeriodOutput {
    UpdateStatus(usize, bool),
}

#[relm4::factory]
impl FactoryComponent for PeriodEntry {
    type Init = PeriodData;
    type Input = PeriodInput;
    type Output = PeriodOutput;
    type CommandOutput = ();
    type ParentWidget = adw::PreferencesGroup;

    view! {
        #[root]
        adw::SwitchRow {
            set_hexpand: true,
            set_use_markup: false,
            #[watch]
            set_title: &super::super::generate_period_title(
                &self.data.global_first_week,
                self.index.current_index(),
                self.data.first_week_num,
                self.data.week_count
            ),
            #[track(self.should_redraw)]
            set_active: self.data.enable,
            connect_active_notify[sender] => move |widget| {
                let status = widget.is_active();
                sender.input(
                    PeriodInput::UpdateStatus(status)
                );
            },
        }
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
            PeriodInput::UpdateStatus(new_status) => {
                if self.data.enable == new_status {
                    return;
                }
                self.data.enable = new_status;
                sender
                    .output(PeriodOutput::UpdateStatus(
                        self.index.current_index(),
                        new_status,
                    ))
                    .unwrap();
            }
        }
    }
}
